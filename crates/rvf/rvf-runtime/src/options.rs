//! Configuration types for the RVF runtime.

use crate::filter::FilterExpr;

/// Distance metric used for vector similarity search.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DistanceMetric {
    /// Squared Euclidean distance (L2).
    #[default]
    L2,
    /// Inner (dot) product distance (negated).
    InnerProduct,
    /// Cosine distance (1 - cosine_similarity).
    Cosine,
}

/// Compression profile for stored vectors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompressionProfile {
    /// No compression — raw fp32 vectors.
    #[default]
    None,
    /// Scalar quantization (int8).
    Scalar,
    /// Product quantization.
    Product,
}

/// Options for creating a new RVF store.
#[derive(Clone, Debug)]
pub struct RvfOptions {
    /// Vector dimensionality (required).
    pub dimension: u16,
    /// Distance metric for similarity search.
    pub metric: DistanceMetric,
    /// Hardware profile identifier (0=Generic, 1=Core, 2=Hot, 3=Full).
    pub profile: u8,
    /// Domain profile for the file (determines canonical extension).
    pub domain_profile: rvf_types::DomainProfile,
    /// Compression profile for stored vectors.
    pub compression: CompressionProfile,
    /// Whether segment signing is enabled.
    pub signing: bool,
    /// HNSW M parameter: max edges per node per layer.
    pub m: u16,
    /// HNSW ef_construction: beam width during index build.
    pub ef_construction: u16,
}

impl Default for RvfOptions {
    fn default() -> Self {
        Self {
            dimension: 0,
            metric: DistanceMetric::L2,
            profile: 0,
            domain_profile: rvf_types::DomainProfile::Generic,
            compression: CompressionProfile::None,
            signing: false,
            m: 16,
            ef_construction: 200,
        }
    }
}

/// Options controlling a query operation.
#[derive(Clone, Debug)]
pub struct QueryOptions {
    /// HNSW ef_search parameter (beam width during search).
    pub ef_search: u16,
    /// Optional metadata filter expression.
    pub filter: Option<FilterExpr>,
    /// Query timeout in milliseconds (0 = no timeout).
    pub timeout_ms: u32,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            ef_search: 100,
            filter: None,
            timeout_ms: 0,
        }
    }
}

/// A single search result: vector ID and distance.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    /// The vector's unique identifier.
    pub id: u64,
    /// Distance from the query vector (lower = more similar).
    pub distance: f32,
}

/// Result of a batch ingest operation.
#[derive(Clone, Debug)]
pub struct IngestResult {
    /// Number of vectors successfully ingested.
    pub accepted: u64,
    /// Number of vectors rejected.
    pub rejected: u64,
    /// Manifest epoch after the ingest commit.
    pub epoch: u32,
}

/// Result of a delete operation.
#[derive(Clone, Debug)]
pub struct DeleteResult {
    /// Number of vectors soft-deleted.
    pub deleted: u64,
    /// Manifest epoch after the delete commit.
    pub epoch: u32,
}

/// Result of a compaction operation.
#[derive(Clone, Debug)]
pub struct CompactionResult {
    /// Number of segments compacted.
    pub segments_compacted: u32,
    /// Bytes of dead space reclaimed.
    pub bytes_reclaimed: u64,
    /// Manifest epoch after compaction commit.
    pub epoch: u32,
}

/// A single metadata entry for a vector.
#[derive(Clone, Debug)]
pub struct MetadataEntry {
    /// Metadata field identifier.
    pub field_id: u16,
    /// The metadata value.
    pub value: MetadataValue,
}

/// Metadata value types matching the spec.
#[derive(Clone, Debug)]
pub enum MetadataValue {
    U64(u64),
    I64(i64),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
}
