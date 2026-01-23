//! # RuvLLM Integration for Prime-Radiant
//!
//! This module provides integration between Prime-Radiant's coherence engine
//! and RuvLLM's LLM serving runtime. It enables:
//!
//! - **Coherence-gated LLM inference**: Use sheaf Laplacian energy to gate LLM outputs
//! - **Witness logging integration**: Connect RuvLLM's witness log to Prime-Radiant governance
//! - **Policy synchronization**: Share learned policies between systems
//! - **SONA integration bridge**: Connect SONA learning loops between both systems
//!
//! ## Architecture
//!
//! ```text
//! +-------------------+     +-------------------+
//! | Prime-Radiant     |<--->| RuvLLM            |
//! | CoherenceEngine   |     | RuvLLMEngine      |
//! +-------------------+     +-------------------+
//!         |                         |
//!         v                         v
//! +-------------------+     +-------------------+
//! | SheafGraph        |     | PolicyStore       |
//! | (Knowledge)       |     | (Ruvector)        |
//! +-------------------+     +-------------------+
//!         |                         |
//!         +----------+  +-----------+
//!                    |  |
//!                    v  v
//!            +-------------------+
//!            | UnifiedWitness    |
//!            | (Audit Trail)     |
//!            +-------------------+
//! ```
//!
//! ## Feature Gate
//!
//! This module requires the `ruvllm` feature flag:
//!
//! ```toml
//! [dependencies]
//! prime-radiant = { version = "0.1", features = ["ruvllm"] }
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use prime_radiant::ruvllm_integration::{
//!     LlmCoherenceGate, LlmCoherenceConfig,
//!     WitnessAdapter, PolicyBridge,
//! };
//!
//! // Create coherence-gated LLM inference
//! let config = LlmCoherenceConfig::default();
//! let gate = LlmCoherenceGate::new(coherence_engine, llm_engine, config)?;
//!
//! // Gate an LLM response
//! let decision = gate.evaluate_response(&response, &context)?;
//! if decision.is_allowed() {
//!     // Response passes coherence checks
//! }
//! ```

// ============================================================================
// SUBMODULE DECLARATIONS
// ============================================================================

mod adapter;
mod bridge;
mod coherence_validator;
mod confidence;
mod config;
mod error;
mod gate;
mod memory_layer;
pub mod pattern_bridge;
mod traits;
mod witness;
mod witness_log;

// ADR references for documentation
pub mod adr_references {
    /// ADR-CE-016: Coherence Validator
    pub const COHERENCE_VALIDATOR: &str = "ADR-CE-016";
    /// ADR-CE-017: Unified Witness Log
    pub const UNIFIED_WITNESS: &str = "ADR-CE-017";
    /// ADR-CE-018: Pattern-to-Restriction Bridge
    pub const PATTERN_BRIDGE: &str = "ADR-CE-018";
    /// ADR-CE-019: Memory as Nodes
    pub const MEMORY_AS_NODES: &str = "ADR-CE-019";
    /// ADR-CE-020: Confidence from Energy
    pub const CONFIDENCE_FROM_ENERGY: &str = "ADR-CE-020";
}

// ============================================================================
// PUBLIC RE-EXPORTS
// ============================================================================

pub use adapter::{
    RuvLlmAdapter, AdapterConfig as LlmAdapterConfig, AdapterStats,
};

pub use bridge::{
    PolicyBridge, PolicyBridgeConfig, PolicySyncResult,
    SonaBridge, SonaBridgeConfig, LearningFeedback,
};

pub use config::{
    LlmCoherenceConfig, GatingMode, ResponsePolicy,
    CoherenceThresholds, HallucinationPolicy,
};

pub use error::{
    RuvLlmIntegrationError, Result,
};

pub use gate::{
    LlmCoherenceGate, LlmGateDecision, LlmGateReason,
    ResponseCoherence, CoherenceAnalysis,
};

pub use witness::{
    WitnessAdapter, WitnessAdapterConfig, UnifiedWitnessEntry,
    WitnessCorrelation, CorrelationId,
};

pub use witness_log::{
    // Core unified witness log types
    UnifiedWitnessLog, GenerationWitness, GenerationWitnessId,
    // Witness summaries
    InferenceWitnessSummary, CoherenceWitnessSummary,
    // Query and statistics
    WitnessQuery, UnifiedWitnessStats,
    // Errors
    UnifiedWitnessError,
};

pub use confidence::{
    CoherenceConfidence, ConfidenceLevel, ConfidenceScore, EnergyContributor,
};

pub use coherence_validator::{
    // Core validator
    SheafCoherenceValidator, ValidatorConfig,
    // Context and weights
    ValidationContext, EdgeWeights,
    // Results
    ValidationResult, ValidationError,
    // Witness
    ValidationWitness, WitnessDecision,
};

pub use memory_layer::{
    // Core types
    MemoryCoherenceLayer, MemoryCoherenceConfig, MemoryCoherenceError,
    Result as MemoryResult,
    // Memory types
    MemoryType, MemoryEdgeType, MemoryEntry, MemoryId, CoherenceResult,
    // Traits
    AgenticMemory, WorkingMemory, EpisodicMemory,
};

// Pattern-to-Restriction Bridge (ADR-CE-018)
pub use pattern_bridge::{
    // Bridge core
    PatternToRestrictionBridge, BridgeConfig, BridgeStats, ExportResult,
    BridgeError, BridgeResult,
    // Pattern types
    PatternData, VerdictData,
    // Provider trait
    PatternProvider,
};

// Trait definitions for loose coupling
pub use traits::{
    // Coherence validation
    CoherenceValidatable, Claim, ClaimType, ContextSource, Fact, SemanticRelation, RelationType,
    // Unified witness
    UnifiedWitnessProvider, GenerationWitnessRef,
    // Pattern bridge trait
    PatternBridge, RestrictionMapRef,
    // Memory coherence (with aliases to avoid conflicts with memory_layer)
    MemoryType as TraitMemoryType, MemoryEntry as TraitMemoryEntry,
    MemoryCoherenceProvider, MemoryAddResult,
    // Confidence
    ConfidenceSource, ConfidenceResult as TraitConfidenceResult, UncertaintySource,
};

// ============================================================================
// CONVENIENCE CONSTRUCTORS
// ============================================================================

use std::sync::Arc;

use crate::coherence::CoherenceEngine;
use crate::governance::PolicyBundle;

/// Create a new LLM coherence gate with default configuration.
///
/// This is a convenience function for quickly setting up coherence-gated
/// LLM inference. For more control, use `LlmCoherenceGate::new()` directly.
///
/// # Arguments
///
/// * `coherence_engine` - The Prime-Radiant coherence engine (Arc-wrapped)
/// * `policy` - The policy bundle for gating decisions
///
/// # Example
///
/// ```rust,ignore
/// use prime_radiant::ruvllm_integration::create_llm_gate;
/// use std::sync::Arc;
///
/// let engine_arc = Arc::new(engine);
/// let gate = create_llm_gate(engine_arc, &policy)?;
/// ```
pub fn create_llm_gate(
    coherence_engine: Arc<CoherenceEngine>,
    policy: &PolicyBundle,
) -> Result<LlmCoherenceGate> {
    let config = LlmCoherenceConfig::default();
    LlmCoherenceGate::new(coherence_engine, policy.clone(), config)
}

/// Create a new witness adapter with default configuration.
///
/// Connects RuvLLM witness logging to Prime-Radiant governance.
///
/// # Example
///
/// ```rust,ignore
/// use prime_radiant::ruvllm_integration::create_witness_adapter;
///
/// let adapter = create_witness_adapter()?;
/// adapter.record(unified_entry)?;
/// ```
pub fn create_witness_adapter() -> Result<WitnessAdapter> {
    let config = WitnessAdapterConfig::default();
    WitnessAdapter::new(config)
}

/// Create a new policy bridge for synchronizing policies between systems.
///
/// # Example
///
/// ```rust,ignore
/// use prime_radiant::ruvllm_integration::create_policy_bridge;
///
/// let bridge = create_policy_bridge()?;
/// bridge.sync_policies()?;
/// ```
pub fn create_policy_bridge() -> Result<PolicyBridge> {
    let config = PolicyBridgeConfig::default();
    PolicyBridge::new(config)
}

// ============================================================================
// MODULE-LEVEL CONSTANTS
// ============================================================================

/// Default coherence threshold for LLM gating
pub const DEFAULT_COHERENCE_THRESHOLD: f64 = 0.8;

/// Default hallucination detection sensitivity
pub const DEFAULT_HALLUCINATION_SENSITIVITY: f64 = 0.7;

/// Default maximum response length before requiring escalation
pub const DEFAULT_MAX_RESPONSE_LENGTH: usize = 4096;

/// Default witness correlation window (seconds)
pub const DEFAULT_CORRELATION_WINDOW_SECS: u64 = 60;

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LlmCoherenceConfig::default();
        assert_eq!(config.coherence_threshold, DEFAULT_COHERENCE_THRESHOLD);
    }

    #[test]
    fn test_feature_gate() {
        // This test only compiles when the ruvllm feature is enabled
        assert!(true, "RuvLLM integration module loaded successfully");
    }
}
