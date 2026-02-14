//! RuVector Format runtime — the main user-facing API.
//!
//! This crate provides [`RvfStore`], the primary interface for creating,
//! opening, querying, and managing RVF vector stores. It ties together
//! the segment model, manifest system, HNSW indexing, quantization, and
//! compaction into a single cohesive runtime.
//!
//! # Architecture
//!
//! - **Append-only writes**: All mutations append new segments; no in-place edits.
//! - **Progressive boot**: Readers see results before the full file is loaded.
//! - **Single-writer / multi-reader**: Advisory lock file enforces exclusivity.
//! - **Background compaction**: Dead space is reclaimed without blocking queries.

pub mod compaction;
pub mod cow;
pub mod cow_compact;
pub mod cow_map;
pub mod deletion;
pub mod filter;
pub mod locking;
pub mod membership;
pub mod options;
pub mod read_path;
pub mod status;
pub mod store;
pub mod write_path;

pub use cow::{CowEngine, CowStats, WitnessEvent};
pub use cow_compact::CowCompactor;
pub use cow_map::CowMap;
pub use filter::FilterExpr;
pub use membership::MembershipFilter;
pub use options::{
    CompactionResult, DeleteResult, IngestResult, MetadataEntry, MetadataValue, QueryOptions,
    RvfOptions, SearchResult,
};
pub use status::StoreStatus;
pub use store::RvfStore;
