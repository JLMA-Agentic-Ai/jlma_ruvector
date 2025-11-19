//! Quantization techniques for memory compression

use crate::error::{Result, VectorDbError};
use crate::types::QuantizationType;
use serde::{Deserialize, Serialize};

/// Quantized vector representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantizedVector {
    /// No quantization - full precision float32
    None(Vec<f32>),
    /// Scalar quantization to int8
    Scalar {
        /// Quantized values
        data: Vec<u8>,
        /// Minimum value for dequantization
        min: f32,
        /// Scale factor for dequantization
        scale: f32,
    },
    /// Product quantization
    Product {
        /// Codebook indices
        codes: Vec<u8>,
        /// Number of subspaces
        subspaces: usize,
    },
    /// Binary quantization (1 bit per dimension)
    Binary {
        /// Packed binary data
        data: Vec<u8>,
        /// Threshold value
        threshold: f32,
    },
}

/// Quantize a vector using specified quantization type
pub fn quantize(vector: &[f32], qtype: QuantizationType) -> Result<QuantizedVector> {
    match qtype {
        QuantizationType::None => Ok(QuantizedVector::None(vector.to_vec())),
        QuantizationType::Scalar => Ok(scalar_quantize(vector)),
        QuantizationType::Product { subspaces, k } => {
            product_quantize(vector, subspaces, k)
        }
        QuantizationType::Binary => Ok(binary_quantize(vector)),
    }
}

/// Dequantize a quantized vector back to float32
pub fn dequantize(quantized: &QuantizedVector) -> Vec<f32> {
    match quantized {
        QuantizedVector::None(v) => v.clone(),
        QuantizedVector::Scalar { data, min, scale } => {
            scalar_dequantize(data, *min, *scale)
        }
        QuantizedVector::Product { codes, subspaces } => {
            // Placeholder - would need codebooks stored separately
            vec![0.0; codes.len() * (codes.len() / subspaces)]
        }
        QuantizedVector::Binary { data, threshold } => {
            binary_dequantize(data, *threshold)
        }
    }
}

/// Scalar quantization to int8
fn scalar_quantize(vector: &[f32]) -> QuantizedVector {
    let min = vector.iter().copied().fold(f32::INFINITY, f32::min);
    let max = vector.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    let scale = if max > min {
        255.0 / (max - min)
    } else {
        1.0
    };

    let data: Vec<u8> = vector
        .iter()
        .map(|&v| ((v - min) * scale).clamp(0.0, 255.0) as u8)
        .collect();

    QuantizedVector::Scalar { data, min, scale }
}

/// Dequantize scalar quantized vector
fn scalar_dequantize(data: &[u8], min: f32, scale: f32) -> Vec<f32> {
    data.iter()
        .map(|&v| (v as f32) / scale + min)
        .collect()
}

/// Product quantization (simplified version)
fn product_quantize(
    vector: &[f32],
    subspaces: usize,
    _k: usize,
) -> Result<QuantizedVector> {
    if !vector.len().is_multiple_of(subspaces) {
        return Err(VectorDbError::Quantization(
            "Vector length must be divisible by number of subspaces".to_string(),
        ));
    }

    // Simplified: just store subspace indices
    // In production, this would involve k-means clustering per subspace
    let subspace_dim = vector.len() / subspaces;
    let codes: Vec<u8> = (0..subspaces)
        .map(|i| {
            let start = i * subspace_dim;
            let subvec = &vector[start..start + subspace_dim];
            // Placeholder: hash to a code (0-255)
            (subvec.iter().sum::<f32>() as u32 % 256) as u8
        })
        .collect();

    Ok(QuantizedVector::Product { codes, subspaces })
}

/// Binary quantization (1 bit per dimension)
fn binary_quantize(vector: &[f32]) -> QuantizedVector {
    let threshold = vector.iter().sum::<f32>() / vector.len() as f32;

    let num_bytes = vector.len().div_ceil(8);
    let mut data = vec![0u8; num_bytes];

    for (i, &val) in vector.iter().enumerate() {
        if val > threshold {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            data[byte_idx] |= 1 << bit_idx;
        }
    }

    QuantizedVector::Binary { data, threshold }
}

/// Dequantize binary quantized vector
fn binary_dequantize(data: &[u8], threshold: f32) -> Vec<f32> {
    let mut result = Vec::with_capacity(data.len() * 8);

    for &byte in data {
        for bit_idx in 0..8 {
            let bit = (byte >> bit_idx) & 1;
            result.push(if bit == 1 { threshold + 1.0 } else { threshold - 1.0 });
        }
    }

    result
}

/// Calculate memory savings from quantization
pub fn calculate_compression_ratio(
    original_dims: usize,
    qtype: QuantizationType,
) -> f32 {
    let original_bytes = original_dims * 4; // float32 = 4 bytes
    let quantized_bytes = match qtype {
        QuantizationType::None => original_bytes,
        QuantizationType::Scalar => original_dims + 8, // u8 per dim + min + scale
        QuantizationType::Product { subspaces, .. } => subspaces + 4, // u8 per subspace + overhead
        QuantizationType::Binary => original_dims.div_ceil(8) + 4, // 1 bit per dim + threshold
    };

    original_bytes as f32 / quantized_bytes as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_quantization() {
        let vector = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let quantized = scalar_quantize(&vector);
        let dequantized = dequantize(&quantized);

        // Check approximate equality (quantization loses precision)
        for (orig, deq) in vector.iter().zip(dequantized.iter()) {
            assert!((orig - deq).abs() < 0.1);
        }
    }

    #[test]
    fn test_binary_quantization() {
        let vector = vec![1.0, 5.0, 2.0, 8.0, 3.0];
        let quantized = binary_quantize(&vector);

        match quantized {
            QuantizedVector::Binary { data, .. } => {
                assert!(!data.is_empty());
            }
            _ => panic!("Expected binary quantization"),
        }
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = calculate_compression_ratio(384, QuantizationType::Scalar);
        assert!(ratio > 3.0); // Should be close to 4x

        let ratio = calculate_compression_ratio(384, QuantizationType::Binary);
        assert!(ratio > 20.0); // Should be close to 32x
    }
}
