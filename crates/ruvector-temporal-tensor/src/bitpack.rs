/// Bitstream packer/unpacker for arbitrary bit widths (1-8).
///
/// Uses a 64-bit accumulator for sub-byte codes with no alignment padding.

/// Pack unsigned codes of `bits` width into a byte stream.
pub fn pack(codes: &[u32], bits: u32, out: &mut Vec<u8>) {
    let mut acc: u64 = 0;
    let mut acc_bits: u32 = 0;

    for &code in codes {
        acc |= (code as u64) << acc_bits;
        acc_bits += bits;
        while acc_bits >= 8 {
            out.push((acc & 0xFF) as u8);
            acc >>= 8;
            acc_bits -= 8;
        }
    }

    if acc_bits > 0 {
        out.push((acc & 0xFF) as u8);
    }
}

/// Unpack `count` unsigned codes of `bits` width from a byte stream.
pub fn unpack(data: &[u8], bits: u32, count: usize, out: &mut Vec<u32>) {
    let mask = (1u64 << bits) - 1;
    let mut acc: u64 = 0;
    let mut acc_bits: u32 = 0;
    let mut byte_idx = 0usize;
    let mut decoded = 0usize;

    while decoded < count {
        // Fill accumulator
        while acc_bits < bits && byte_idx < data.len() {
            acc |= (data[byte_idx] as u64) << acc_bits;
            acc_bits += 8;
            byte_idx += 1;
        }
        if acc_bits < bits {
            break; // Insufficient data
        }

        out.push((acc & mask) as u32);
        acc >>= bits;
        acc_bits -= bits;
        decoded += 1;
    }
}

/// Compute qmax for a given bit width: 2^(bits-1) - 1
#[inline]
pub fn qmax_from_bits(bits: u8) -> i32 {
    if bits == 0 || bits > 8 {
        return 0;
    }
    (1i32 << (bits - 1)) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_8bit() {
        let codes: Vec<u32> = (0..256).collect();
        let mut packed = Vec::new();
        pack(&codes, 8, &mut packed);
        assert_eq!(packed.len(), 256); // 8-bit = 1 byte each

        let mut unpacked = Vec::new();
        unpack(&packed, 8, 256, &mut unpacked);
        assert_eq!(codes, unpacked);
    }

    #[test]
    fn test_roundtrip_3bit() {
        let codes: Vec<u32> = (0..7).collect(); // 3-bit range: 0-6
        let mut packed = Vec::new();
        pack(&codes, 3, &mut packed);

        let mut unpacked = Vec::new();
        unpack(&packed, 3, 7, &mut unpacked);
        assert_eq!(codes, unpacked);
    }

    #[test]
    fn test_roundtrip_5bit() {
        let codes: Vec<u32> = (0..31).collect();
        let mut packed = Vec::new();
        pack(&codes, 5, &mut packed);

        let mut unpacked = Vec::new();
        unpack(&packed, 5, 31, &mut unpacked);
        assert_eq!(codes, unpacked);
    }

    #[test]
    fn test_roundtrip_7bit() {
        let codes: Vec<u32> = (0..127).collect();
        let mut packed = Vec::new();
        pack(&codes, 7, &mut packed);

        let mut unpacked = Vec::new();
        unpack(&packed, 7, 127, &mut unpacked);
        assert_eq!(codes, unpacked);
    }

    #[test]
    fn test_packing_density() {
        // 100 3-bit codes = 300 bits = 38 bytes (ceil(300/8))
        let codes = vec![5u32; 100];
        let mut packed = Vec::new();
        pack(&codes, 3, &mut packed);
        assert_eq!(packed.len(), 38); // ceil(300/8) = 38
    }

    #[test]
    fn test_qmax() {
        assert_eq!(qmax_from_bits(8), 127);
        assert_eq!(qmax_from_bits(7), 63);
        assert_eq!(qmax_from_bits(5), 15);
        assert_eq!(qmax_from_bits(3), 3);
        assert_eq!(qmax_from_bits(1), 0);
        assert_eq!(qmax_from_bits(0), 0);
    }
}
