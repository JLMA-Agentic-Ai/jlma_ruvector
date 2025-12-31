//! # RuVector Edge - Distributed AI Swarm Communication
//!
//! Edge AI swarm communication using `ruv-swarm-transport` with RuVector intelligence.
//!
//! ## Features
//!
//! - **WebSocket Transport**: Remote swarm communication
//! - **SharedMemory Transport**: High-performance local IPC
//! - **WASM Support**: Run in browser/edge environments
//! - **Intelligence Sync**: Distributed Q-learning across agents
//! - **Memory Sharing**: Shared vector memory for RAG
//! - **Tensor Compression**: Efficient pattern transfer
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ruvector_edge::{SwarmAgent, SwarmConfig, Transport};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = SwarmConfig::default()
//!         .with_transport(Transport::WebSocket)
//!         .with_agent_id("agent-001");
//!
//!     let agent = SwarmAgent::new(config).await.unwrap();
//!     agent.join_swarm("ws://coordinator:8080").await.unwrap();
//!
//!     // Sync learning patterns
//!     agent.sync_patterns().await.unwrap();
//! }
//! ```

pub mod transport;
pub mod intelligence;
pub mod memory;
pub mod compression;
pub mod protocol;
pub mod agent;

// Re-exports
pub use agent::{SwarmAgent, AgentRole};
pub use transport::{Transport, TransportConfig};
pub use intelligence::{IntelligenceSync, LearningState, Pattern};
pub use memory::{SharedMemory, VectorMemory};
pub use compression::{TensorCodec, CompressionLevel};
pub use protocol::{SwarmMessage, MessageType};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Swarm configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub agent_id: String,
    pub agent_role: AgentRole,
    pub transport: Transport,
    pub coordinator_url: Option<String>,
    pub sync_interval_ms: u64,
    pub compression_level: CompressionLevel,
    pub max_peers: usize,
    pub enable_learning: bool,
    pub enable_memory_sync: bool,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            agent_id: Uuid::new_v4().to_string(),
            agent_role: AgentRole::Worker,
            transport: Transport::WebSocket,
            coordinator_url: None,
            sync_interval_ms: 1000,
            compression_level: CompressionLevel::Fast,
            max_peers: 100,
            enable_learning: true,
            enable_memory_sync: true,
        }
    }
}

impl SwarmConfig {
    pub fn with_transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    pub fn with_agent_id(mut self, id: impl Into<String>) -> Self {
        self.agent_id = id.into();
        self
    }

    pub fn with_role(mut self, role: AgentRole) -> Self {
        self.agent_role = role;
        self
    }

    pub fn with_coordinator(mut self, url: impl Into<String>) -> Self {
        self.coordinator_url = Some(url.into());
        self
    }
}

/// Error types for edge swarm operations
#[derive(Debug, thiserror::Error)]
pub enum SwarmError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, SwarmError>;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        SwarmAgent, SwarmConfig, SwarmError, Result,
        Transport, AgentRole, MessageType,
        IntelligenceSync, SharedMemory,
        CompressionLevel,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = SwarmConfig::default()
            .with_agent_id("test-agent")
            .with_transport(Transport::SharedMemory)
            .with_role(AgentRole::Coordinator);

        assert_eq!(config.agent_id, "test-agent");
        assert!(matches!(config.transport, Transport::SharedMemory));
        assert!(matches!(config.agent_role, AgentRole::Coordinator));
    }
}
