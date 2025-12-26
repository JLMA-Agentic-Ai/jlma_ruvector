//! pgvector Drop-In Compatibility Test Suite for RuVector Postgres v2
//!
//! This module provides comprehensive validation that RuVector is a 100% compatible
//! drop-in replacement for pgvector. Tests cover:
//!
//! 1. Type Compatibility - vector(n), halfvec(n), sparsevec types
//! 2. Operator Compatibility - <->, <#>, <=>, +, -, * operators
//! 3. Function Compatibility - l2_distance, inner_product, cosine_distance, etc.
//! 4. Index Compatibility - HNSW and IVFFlat with all WITH options
//! 5. Query Compatibility - ORDER BY, LIMIT, WHERE clauses
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all pgvector compatibility tests
//! cargo pgrx test pgvector_compat
//!
//! # Run the comparison harness against both pgvector and ruvector
//! ./tests/pgvector_compat/run_comparison.sh
//! ```
//!
//! ## Test Categories
//!
//! - `types.rs` - Vector type creation, casting, and storage
//! - `operators.rs` - Distance operators and vector arithmetic
//! - `functions.rs` - SQL function compatibility
//! - `indexes.rs` - Index creation and usage
//! - `queries.rs` - Complex query patterns
//! - `edge_cases.rs` - Boundary conditions and error handling
//! - `comparison.rs` - Side-by-side pgvector/ruvector comparison

pub mod types;
pub mod operators;
pub mod functions;
pub mod indexes;
pub mod queries;
pub mod edge_cases;
pub mod comparison;

/// Version of pgvector API we are compatible with
pub const PGVECTOR_COMPAT_VERSION: &str = "0.7.0";

/// Maximum supported dimensions (matches pgvector)
pub const MAX_DIMENSIONS: usize = 16_000;

/// Epsilon for floating-point comparisons
pub const FLOAT_EPSILON: f32 = 1e-5;

/// Test precision epsilon (slightly looser for SIMD variations)
pub const TEST_EPSILON: f32 = 1e-4;
