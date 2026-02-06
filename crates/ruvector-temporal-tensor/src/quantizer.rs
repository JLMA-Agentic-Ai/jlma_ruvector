/// Groupwise symmetric quantization with f16 scales.
///
/// For each group of `group_len` values:
///   scale = max(|v_i|) / qmax
///   q_i = round(v_i / scale), clamped to [-qmax, +qmax]
///   u_i = q_i + qmax  (bias to unsigned for packing)

use crate::bitpack::qmax_from_bits;
use crate::f16;

/// Compute f16 group scales for a frame.
pub fn compute_scales(frame: &[f32], group_len: usize, bits: u8) -> Vec<u16> {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 {
        return Vec::new();
    }
    let qmax_f = qmax as f32;

    let num_groups = (frame.len() + group_len - 1) / group_len;
    let mut scales = Vec::with_capacity(num_groups);

    for chunk in frame.chunks(group_len) {
        let mut max_abs = 0.0f32;
        for &v in chunk {
            if v.is_finite() {
                let a = v.abs();
                if a > max_abs {
                    max_abs = a;
                }
            }
        }

        let scale = if max_abs == 0.0 { 0.0 } else { max_abs / qmax_f };
        scales.push(f16::f32_to_f16_bits(scale));
    }

    scales
}

/// Pre-convert f16 scales to f32 for hot-path use.
#[inline]
pub fn scales_to_f32(scales_f16: &[u16]) -> Vec<f32> {
    scales_f16.iter().map(|&s| f16::f16_bits_to_f32(s)).collect()
}

/// Check if a frame fits within existing scales (within drift tolerance).
/// Uses pre-converted f32 scales to avoid repeated f16 conversion.
pub fn frame_fits_scales_f32(
    frame: &[f32],
    scales_f32: &[f32],
    group_len: usize,
    bits: u8,
    drift_factor: f32,
) -> bool {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 || scales_f32.is_empty() {
        return false;
    }
    let qmax_f = qmax as f32;

    for (group_idx, chunk) in frame.chunks(group_len).enumerate() {
        if group_idx >= scales_f32.len() {
            return false;
        }
        let allowed = scales_f32[group_idx] * qmax_f * drift_factor;

        for &v in chunk {
            if v.is_finite() && v.abs() > allowed {
                return false;
            }
        }
    }

    true
}

/// Quantize a frame using pre-computed f32 scales and pack into bitstream.
/// Caller must pre-convert f16 scales to f32 via `scales_to_f32`.
pub fn quantize_and_pack_f32(
    frame: &[f32],
    scales_f32: &[f32],
    group_len: usize,
    bits: u8,
    out: &mut Vec<u8>,
) {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 {
        return;
    }
    let qmax_i = qmax;
    let bias = qmax;
    let bits_u32 = bits as u32;

    // Pre-reserve: each value takes `bits` bits, total = ceil(len * bits / 8)
    let needed_bytes = (frame.len() * bits as usize + 7) / 8;
    out.reserve(needed_bytes);

    let mut acc: u64 = 0;
    let mut acc_bits: u32 = 0;

    for (group_idx, chunk) in frame.chunks(group_len).enumerate() {
        let scale = if group_idx < scales_f32.len() {
            scales_f32[group_idx]
        } else {
            0.0
        };
        let inv_scale = if scale == 0.0 { 0.0 } else { 1.0 / scale };

        for &v in chunk {
            let mut q: i32 = 0;
            if v.is_finite() {
                let scaled = v * inv_scale;
                q = scaled.round() as i32;
                q = q.clamp(-qmax_i, qmax_i);
            }

            let u = (q + bias) as u32;
            acc |= (u as u64) << acc_bits;
            acc_bits += bits_u32;

            while acc_bits >= 8 {
                out.push((acc & 0xFF) as u8);
                acc >>= 8;
                acc_bits -= 8;
            }
        }
    }

    if acc_bits > 0 {
        out.push((acc & 0xFF) as u8);
    }
}

/// Dequantize packed codes using f32 scales, writing f32 values.
///
/// Optimized: iterates by frame then by group to avoid per-value modulo/division
/// and caches the f32 scale per group instead of converting f16 per value.
pub fn dequantize_f32(
    data: &[u8],
    scales_f32: &[f32],
    group_len: usize,
    bits: u8,
    tensor_len: usize,
    frame_count: usize,
    out: &mut Vec<f32>,
) {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 {
        return;
    }
    let bias = qmax;
    let bits_u32 = bits as u32;
    let mask = (1u64 << bits_u32) - 1;

    let total = tensor_len * frame_count;
    out.resize(total, 0.0);

    let mut acc: u64 = 0;
    let mut acc_bits: u32 = 0;
    let mut byte_idx = 0usize;
    let mut out_idx = 0usize;

    for _frame in 0..frame_count {
        let mut pos = 0usize;
        let mut group_idx = 0usize;

        while pos < tensor_len {
            let group_end = (pos + group_len).min(tensor_len);
            let scale = if group_idx < scales_f32.len() {
                scales_f32[group_idx]
            } else {
                0.0
            };

            while pos < group_end {
                // Fill accumulator
                while acc_bits < bits_u32 && byte_idx < data.len() {
                    acc |= (data[byte_idx] as u64) << acc_bits;
                    acc_bits += 8;
                    byte_idx += 1;
                }
                if acc_bits < bits_u32 {
                    return; // Ran out of data
                }

                let u = (acc & mask) as u32;
                acc >>= bits_u32;
                acc_bits -= bits_u32;

                let q = (u as i32) - bias;
                out[out_idx] = (q as f32) * scale;
                out_idx += 1;
                pos += 1;
            }

            group_idx += 1;
        }
    }
}

// --- Legacy API (kept for backward compatibility with segment.rs) ---

/// Check if a frame fits within existing f16 scales (within drift tolerance).
pub fn frame_fits_scales(
    frame: &[f32],
    scales: &[u16],
    group_len: usize,
    bits: u8,
    drift_factor: f32,
) -> bool {
    let scales_f32 = scales_to_f32(scales);
    frame_fits_scales_f32(frame, &scales_f32, group_len, bits, drift_factor)
}

/// Quantize a frame using pre-computed f16 scales and pack into bitstream.
pub fn quantize_and_pack(
    frame: &[f32],
    scales: &[u16],
    group_len: usize,
    bits: u8,
    out: &mut Vec<u8>,
) {
    let scales_f32 = scales_to_f32(scales);
    quantize_and_pack_f32(frame, &scales_f32, group_len, bits, out)
}

/// Dequantize packed codes using f16 scales, writing f32 values.
pub fn dequantize(
    data: &[u8],
    scales: &[u16],
    group_len: usize,
    bits: u8,
    tensor_len: usize,
    frame_count: usize,
    out: &mut Vec<f32>,
) {
    let scales_f32 = scales_to_f32(scales);
    dequantize_f32(data, &scales_f32, group_len, bits, tensor_len, frame_count, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_roundtrip_8bit() {
        let frame: Vec<f32> = (0..128).map(|i| (i as f32 - 64.0) * 0.1).collect();
        let group_len = 64;
        let bits = 8;

        let scales = compute_scales(&frame, group_len, bits);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, group_len, bits, frame.len(), 1, &mut decoded);

        assert_eq!(decoded.len(), frame.len());
        for (i, (&orig, &dec)) in frame.iter().zip(decoded.iter()).enumerate() {
            let err = (orig - dec).abs();
            let max_err = if orig.abs() > 0.01 { orig.abs() * 0.02 } else { 0.1 };
            assert!(err < max_err, "i={i}, orig={orig}, dec={dec}, err={err}");
        }
    }

    #[test]
    fn test_quantize_roundtrip_3bit() {
        let frame: Vec<f32> = (0..64).map(|i| (i as f32 - 32.0) * 0.5).collect();
        let group_len = 64;
        let bits = 3;

        let scales = compute_scales(&frame, group_len, bits);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, group_len, bits, frame.len(), 1, &mut decoded);

        assert_eq!(decoded.len(), frame.len());
        let max_val = frame.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        for (&orig, &dec) in frame.iter().zip(decoded.iter()) {
            let err = (orig - dec).abs();
            assert!(err < max_val * 0.35, "orig={orig}, dec={dec}, err={err}");
        }
    }

    #[test]
    fn test_quantize_roundtrip_5bit() {
        let frame: Vec<f32> = (0..256).map(|i| (i as f32 - 128.0) * 0.05).collect();
        let group_len = 64;
        let bits = 5;

        let scales = compute_scales(&frame, group_len, bits);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, group_len, bits, frame.len(), 1, &mut decoded);

        assert_eq!(decoded.len(), frame.len());
        let max_val = frame.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        for (&orig, &dec) in frame.iter().zip(decoded.iter()) {
            let err = (orig - dec).abs();
            assert!(err < max_val * 0.08, "orig={orig}, dec={dec}, err={err}");
        }
    }

    #[test]
    fn test_quantize_roundtrip_7bit() {
        let frame: Vec<f32> = (0..256).map(|i| (i as f32 - 128.0) * 0.05).collect();
        let group_len = 64;
        let bits = 7;

        let scales = compute_scales(&frame, group_len, bits);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, group_len, bits, frame.len(), 1, &mut decoded);

        assert_eq!(decoded.len(), frame.len());
        for (i, (&orig, &dec)) in frame.iter().zip(decoded.iter()).enumerate() {
            let err = (orig - dec).abs();
            let max_err = if orig.abs() > 0.01 { orig.abs() * 0.02 } else { 0.1 };
            assert!(err < max_err, "i={i}, orig={orig}, dec={dec}, err={err}");
        }
    }

    #[test]
    fn test_drift_detection() {
        let frame1: Vec<f32> = vec![1.0; 64];
        let frame2: Vec<f32> = vec![1.05; 64]; // 5% drift
        let frame3: Vec<f32> = vec![2.0; 64]; // 100% drift

        let scales = compute_scales(&frame1, 64, 8);
        let drift_factor = 1.0 + 26.0 / 256.0; // ~10%

        assert!(frame_fits_scales(&frame2, &scales, 64, 8, drift_factor));
        assert!(!frame_fits_scales(&frame3, &scales, 64, 8, drift_factor));
    }

    #[test]
    fn test_zero_frame() {
        let frame = vec![0.0f32; 128];
        let scales = compute_scales(&frame, 64, 8);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, 64, 8, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, 64, 8, 128, 1, &mut decoded);

        for &v in &decoded {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn test_non_finite_values() {
        let mut frame = vec![1.0f32; 64];
        frame[10] = f32::NAN;
        frame[20] = f32::INFINITY;
        frame[30] = f32::NEG_INFINITY;

        let scales = compute_scales(&frame, 64, 8);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, 64, 8, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, 64, 8, 64, 1, &mut decoded);

        assert_eq!(decoded.len(), 64);
        // Non-finite values should quantize to 0
        assert_eq!(decoded[10], 0.0);
        assert_eq!(decoded[20], 0.0);
        assert_eq!(decoded[30], 0.0);
        // Normal values should be close
        assert!((decoded[0] - 1.0).abs() < 0.02);
    }

    #[test]
    fn test_single_element_group() {
        let frame = vec![3.14f32; 16];
        let group_len = 1; // Extreme: 1 element per group
        let bits = 8;

        let scales = compute_scales(&frame, group_len, bits);
        assert_eq!(scales.len(), 16);

        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, group_len, bits, 16, 1, &mut decoded);

        for (i, &v) in decoded.iter().enumerate() {
            let err = (v - 3.14).abs();
            assert!(err < 0.03, "i={i} v={v} err={err}");
        }
    }

    #[test]
    fn test_compression_ratio() {
        let frame = vec![1.0f32; 512];
        let group_len = 64;

        for &(bits, min_ratio) in &[(8u8, 3.5f32), (7, 4.0), (5, 5.5), (3, 8.5)] {
            let scales = compute_scales(&frame, group_len, bits);
            let mut packed = Vec::new();
            quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

            let raw_bytes = frame.len() * 4;
            let compressed = packed.len() + scales.len() * 2;
            let ratio = raw_bytes as f32 / compressed as f32;

            assert!(
                ratio >= min_ratio,
                "bits={bits}: ratio {ratio:.2}x < expected {min_ratio}x"
            );
        }
    }
}
