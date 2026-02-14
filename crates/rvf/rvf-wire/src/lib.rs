//! RVF wire format reader/writer.
//!
//! This crate implements the binary encoding and decoding for the RuVector
//! Format (RVF): segment headers, varint encoding, delta coding, hash
//! computation, tail scanning, and per-segment-type codecs.

pub mod varint;
pub mod delta;
pub mod hash;
pub mod reader;
pub mod writer;
pub mod tail_scan;
pub mod manifest_codec;
pub mod vec_seg_codec;
pub mod hot_seg_codec;
pub mod index_seg_codec;

pub use reader::{read_segment, read_segment_header, validate_segment};
pub use writer::{write_segment, calculate_padded_size};
pub use tail_scan::find_latest_manifest;
