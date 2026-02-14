//! Segment type discriminator for the RVF format.

/// Identifies the kind of data stored in a segment.
///
/// Values `0x00` and `0xF0..=0xFF` are reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum SegmentType {
    /// Not a valid segment (uninitialized / zeroed region).
    Invalid = 0x00,
    /// Raw vector payloads (the actual embeddings).
    Vec = 0x01,
    /// HNSW adjacency lists, entry points, routing tables.
    Index = 0x02,
    /// Graph overlay deltas, partition updates, min-cut witnesses.
    Overlay = 0x03,
    /// Metadata mutations (label changes, deletions, moves).
    Journal = 0x04,
    /// Segment directory, hotset pointers, epoch state.
    Manifest = 0x05,
    /// Quantization dictionaries and codebooks.
    Quant = 0x06,
    /// Arbitrary key-value metadata (tags, provenance, lineage).
    Meta = 0x07,
    /// Temperature-promoted hot data (vectors + neighbors).
    Hot = 0x08,
    /// Access counter sketches for temperature decisions.
    Sketch = 0x09,
    /// Capability manifests, proof of computation, audit trails.
    Witness = 0x0A,
    /// Domain profile declarations (RVDNA, RVText, etc.).
    Profile = 0x0B,
    /// Key material, signature chains, certificate anchors.
    Crypto = 0x0C,
    /// Metadata inverted indexes for filtered search.
    MetaIdx = 0x0D,
    /// Embedded kernel / unikernel image for self-booting.
    Kernel = 0x0E,
    /// Embedded eBPF program for kernel fast path.
    Ebpf = 0x0F,
    /// COW cluster mapping.
    CowMap = 0x20,
    /// Cluster reference counts.
    Refcount = 0x21,
    /// Vector membership filter.
    Membership = 0x22,
    /// Sparse delta patches.
    Delta = 0x23,
}

impl TryFrom<u8> for SegmentType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Invalid),
            0x01 => Ok(Self::Vec),
            0x02 => Ok(Self::Index),
            0x03 => Ok(Self::Overlay),
            0x04 => Ok(Self::Journal),
            0x05 => Ok(Self::Manifest),
            0x06 => Ok(Self::Quant),
            0x07 => Ok(Self::Meta),
            0x08 => Ok(Self::Hot),
            0x09 => Ok(Self::Sketch),
            0x0A => Ok(Self::Witness),
            0x0B => Ok(Self::Profile),
            0x0C => Ok(Self::Crypto),
            0x0D => Ok(Self::MetaIdx),
            0x0E => Ok(Self::Kernel),
            0x0F => Ok(Self::Ebpf),
            0x20 => Ok(Self::CowMap),
            0x21 => Ok(Self::Refcount),
            0x22 => Ok(Self::Membership),
            0x23 => Ok(Self::Delta),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_variants() {
        let variants = [
            SegmentType::Invalid,
            SegmentType::Vec,
            SegmentType::Index,
            SegmentType::Overlay,
            SegmentType::Journal,
            SegmentType::Manifest,
            SegmentType::Quant,
            SegmentType::Meta,
            SegmentType::Hot,
            SegmentType::Sketch,
            SegmentType::Witness,
            SegmentType::Profile,
            SegmentType::Crypto,
            SegmentType::MetaIdx,
            SegmentType::Kernel,
            SegmentType::Ebpf,
            SegmentType::CowMap,
            SegmentType::Refcount,
            SegmentType::Membership,
            SegmentType::Delta,
        ];
        for v in variants {
            let raw = v as u8;
            assert_eq!(SegmentType::try_from(raw), Ok(v));
        }
    }

    #[test]
    fn invalid_value_returns_err() {
        assert_eq!(SegmentType::try_from(0x24), Err(0x24));
        assert_eq!(SegmentType::try_from(0xF0), Err(0xF0));
        assert_eq!(SegmentType::try_from(0xFF), Err(0xFF));
    }

    #[test]
    fn kernel_and_ebpf_discriminants() {
        assert_eq!(SegmentType::Kernel as u8, 0x0E);
        assert_eq!(SegmentType::Ebpf as u8, 0x0F);
    }
}
