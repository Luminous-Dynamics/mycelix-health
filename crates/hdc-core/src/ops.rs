//! Core HDC operations
//!
//! Pure functions for hypervector computation.
//! Includes SIMD-optimized paths for AVX2-capable CPUs.

use sha2::{Digest, Sha256};
use crate::HYPERVECTOR_DIM;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Check if AVX2 is available at runtime
#[cfg(target_arch = "x86_64")]
pub fn has_avx2() -> bool {
    is_x86_feature_detected!("avx2")
}

#[cfg(not(target_arch = "x86_64"))]
pub fn has_avx2() -> bool {
    false
}

/// Generate a random hypervector from a seed and item identifier
///
/// Uses SHA-256 based expansion for cryptographically secure randomness.
pub fn generate_item_vector(seed: &[u8; 32], item: &str) -> Vec<u8> {
    let num_bytes = (HYPERVECTOR_DIM + 7) / 8;
    let mut result = Vec::with_capacity(num_bytes);

    let mut counter = 0u64;
    while result.len() < num_bytes {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(item.as_bytes());
        hasher.update(&counter.to_le_bytes());
        let hash = hasher.finalize();

        result.extend_from_slice(&hash);
        counter += 1;
    }

    result.truncate(num_bytes);
    result
}

/// Bind two hypervectors (XOR for binary vectors)
///
/// Uses SIMD acceleration when available.
/// XOR is self-inverse: (a XOR b) XOR b = a
#[inline]
pub fn bind(a: &[u8], b: &[u8]) -> Vec<u8> {
    debug_assert_eq!(a.len(), b.len(), "Vectors must have same length");

    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            return unsafe { bind_avx2(a, b) };
        }
    }

    bind_scalar(a, b)
}

/// Scalar implementation of bind
#[inline]
fn bind_scalar(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
}

/// AVX2-optimized bind (XOR) operation
/// Processes 32 bytes at a time
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn bind_avx2(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len();
    let mut result = vec![0u8; len];

    let chunks = len / 32;
    let remainder = len % 32;

    // Process 32-byte chunks with AVX2
    for i in 0..chunks {
        let offset = i * 32;
        let va = _mm256_loadu_si256(a.as_ptr().add(offset) as *const __m256i);
        let vb = _mm256_loadu_si256(b.as_ptr().add(offset) as *const __m256i);
        let xored = _mm256_xor_si256(va, vb);
        _mm256_storeu_si256(result.as_mut_ptr().add(offset) as *mut __m256i, xored);
    }

    // Handle remaining bytes
    let base = chunks * 32;
    for i in 0..remainder {
        result[base + i] = a[base + i] ^ b[base + i];
    }

    result
}

/// Bundle multiple hypervectors using majority voting
///
/// Each bit position takes the majority value across all input vectors.
/// Ties go to 0 for determinism.
pub fn bundle(vectors: &[&[u8]]) -> Vec<u8> {
    if vectors.is_empty() {
        return vec![0u8; (HYPERVECTOR_DIM + 7) / 8];
    }

    let len = vectors[0].len();
    let mut result = vec![0u8; len];
    let threshold = vectors.len() / 2;

    for byte_idx in 0..len {
        let mut bit_result = 0u8;

        for bit_pos in 0..8 {
            let dim_idx = byte_idx * 8 + bit_pos;
            if dim_idx >= HYPERVECTOR_DIM {
                break;
            }

            // Count 1s across all vectors for this bit position
            let ones: usize = vectors.iter()
                .map(|v| ((v[byte_idx] >> bit_pos) & 1) as usize)
                .sum();

            // Majority voting
            if ones > threshold {
                bit_result |= 1 << bit_pos;
            }
        }

        result[byte_idx] = bit_result;
    }

    result
}

/// Weighted bundle using thresholded sums
pub fn weighted_bundle(vectors: &[(&[u8], f64)]) -> Vec<u8> {
    if vectors.is_empty() {
        return vec![0u8; (HYPERVECTOR_DIM + 7) / 8];
    }

    let len = vectors[0].0.len();
    let total_weight: f64 = vectors.iter().map(|(_, w)| w).sum();
    let threshold = total_weight / 2.0;

    let mut result = vec![0u8; len];

    for byte_idx in 0..len {
        let mut bit_result = 0u8;

        for bit_pos in 0..8 {
            let dim_idx = byte_idx * 8 + bit_pos;
            if dim_idx >= HYPERVECTOR_DIM {
                break;
            }

            let weighted_sum: f64 = vectors.iter()
                .map(|(v, w)| {
                    let bit = ((v[byte_idx] >> bit_pos) & 1) as f64;
                    bit * w
                })
                .sum();

            if weighted_sum > threshold {
                bit_result |= 1 << bit_pos;
            }
        }

        result[byte_idx] = bit_result;
    }

    result
}

/// Permute a hypervector (for positional encoding)
///
/// Rotates all bits by `shift` positions.
pub fn permute(v: &[u8], shift: usize) -> Vec<u8> {
    let total_bits = v.len() * 8;
    let shift = shift % HYPERVECTOR_DIM;

    if shift == 0 {
        return v.to_vec();
    }

    let mut result = vec![0u8; v.len()];

    for i in 0..HYPERVECTOR_DIM.min(total_bits) {
        let new_pos = (i + shift) % HYPERVECTOR_DIM;
        let old_byte = i / 8;
        let old_bit = i % 8;
        let new_byte = new_pos / 8;
        let new_bit = new_pos % 8;

        let bit_value = (v[old_byte] >> old_bit) & 1;
        result[new_byte] |= bit_value << new_bit;
    }

    result
}

/// Count population (number of 1 bits) in a byte slice
/// Uses SIMD acceleration when available
#[inline]
pub fn popcount(data: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() && is_x86_feature_detected!("popcnt") {
            return unsafe { popcount_simd(data) };
        }
    }

    popcount_scalar(data)
}

/// Scalar popcount
#[inline]
fn popcount_scalar(data: &[u8]) -> usize {
    data.iter().map(|b| b.count_ones() as usize).sum()
}

/// SIMD-optimized popcount using POPCNT instruction
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "popcnt")]
unsafe fn popcount_simd(data: &[u8]) -> usize {
    let mut count = 0usize;

    // Process 8 bytes at a time using 64-bit popcnt
    let chunks = data.len() / 8;
    let remainder = data.len() % 8;

    let ptr = data.as_ptr() as *const u64;
    for i in 0..chunks {
        count += _popcnt64(*ptr.add(i) as i64) as usize;
    }

    // Handle remaining bytes
    let base = chunks * 8;
    for i in 0..remainder {
        count += data[base + i].count_ones() as usize;
    }

    count
}

/// Calculate Hamming similarity between two hypervectors
///
/// Uses SIMD acceleration when available.
/// Returns value between 0.0 (completely different) and 1.0 (identical)
#[inline]
pub fn hamming_similarity(a: &[u8], b: &[u8]) -> f64 {
    debug_assert_eq!(a.len(), b.len(), "Vectors must have same length");

    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            return unsafe { hamming_similarity_avx2(a, b) };
        }
    }

    hamming_similarity_scalar(a, b)
}

/// Scalar Hamming similarity
#[inline]
fn hamming_similarity_scalar(a: &[u8], b: &[u8]) -> f64 {
    let matching_bits: usize = a.iter()
        .zip(b.iter())
        .map(|(x, y)| (!(x ^ y)).count_ones() as usize)
        .sum();

    matching_bits as f64 / HYPERVECTOR_DIM as f64
}

/// AVX2-optimized Hamming similarity
/// XORs vectors and counts matching bits in one pass
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "popcnt")]
unsafe fn hamming_similarity_avx2(a: &[u8], b: &[u8]) -> f64 {
    let len = a.len();
    let mut diff_bits = 0usize;

    // Process 32-byte chunks
    let chunks = len / 32;
    let remainder = len % 32;

    for i in 0..chunks {
        let offset = i * 32;
        let va = _mm256_loadu_si256(a.as_ptr().add(offset) as *const __m256i);
        let vb = _mm256_loadu_si256(b.as_ptr().add(offset) as *const __m256i);
        let xored = _mm256_xor_si256(va, vb);

        // Extract and count bits in 64-bit chunks
        let bytes: [u8; 32] = std::mem::transmute(xored);
        let ptr = bytes.as_ptr() as *const u64;
        for j in 0..4 {
            diff_bits += _popcnt64(*ptr.add(j) as i64) as usize;
        }
    }

    // Handle remaining bytes
    let base = chunks * 32;
    for i in 0..remainder {
        diff_bits += (a[base + i] ^ b[base + i]).count_ones() as usize;
    }

    let matching_bits = HYPERVECTOR_DIM - diff_bits;
    matching_bits as f64 / HYPERVECTOR_DIM as f64
}

/// Calculate cosine similarity (interpreting bits as bipolar -1/+1)
///
/// Uses SIMD acceleration when available.
/// For binary vectors: cosine = (2 * matching - total) / total
/// Returns value between -1.0 (opposite) and 1.0 (identical)
#[inline]
pub fn cosine_similarity(a: &[u8], b: &[u8]) -> f64 {
    // Reuse hamming_similarity since it computes the same thing
    let hamming = hamming_similarity(a, b);
    let total = HYPERVECTOR_DIM as f64;
    let matching_bits = hamming * total;
    (2.0 * matching_bits - total) / total
}

/// Normalize cosine similarity to 0-1 range
#[inline]
pub fn normalized_cosine_similarity(a: &[u8], b: &[u8]) -> f64 {
    (cosine_similarity(a, b) + 1.0) / 2.0
}

/// Add noise to a hypervector (for differential privacy)
///
/// Flips each bit with probability `noise_level`
#[cfg(feature = "rand")]
pub fn add_noise(v: &[u8], noise_level: f64, rng: &mut impl rand::Rng) -> Vec<u8> {
    v.iter()
        .map(|byte| {
            let mut result = *byte;
            for bit in 0..8 {
                if rng.gen::<f64>() < noise_level {
                    result ^= 1 << bit;
                }
            }
            result
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_deterministic() {
        let seed = [42u8; 32];
        let v1 = generate_item_vector(&seed, "test");
        let v2 = generate_item_vector(&seed, "test");
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_generate_different_items() {
        let seed = [42u8; 32];
        let v1 = generate_item_vector(&seed, "item1");
        let v2 = generate_item_vector(&seed, "item2");
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_bind_self_inverse() {
        let seed = [42u8; 32];
        let a = generate_item_vector(&seed, "a");
        let b = generate_item_vector(&seed, "b");

        let bound = bind(&a, &b);
        let unbound = bind(&bound, &b);

        assert_eq!(a, unbound);
    }

    #[test]
    fn test_permute_identity() {
        let seed = [42u8; 32];
        let v = generate_item_vector(&seed, "test");
        let permuted = permute(&v, 0);
        assert_eq!(v, permuted);
    }

    #[test]
    fn test_permute_full_rotation() {
        let seed = [42u8; 32];
        let v = generate_item_vector(&seed, "test");
        let permuted = permute(&v, HYPERVECTOR_DIM);
        assert_eq!(v, permuted);
    }

    #[test]
    fn test_similarity_identical() {
        let seed = [42u8; 32];
        let v = generate_item_vector(&seed, "test");

        assert!((hamming_similarity(&v, &v) - 1.0).abs() < 0.001);
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 0.001);
        assert!((normalized_cosine_similarity(&v, &v) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_random_near_half() {
        let seed = [42u8; 32];
        let v1 = generate_item_vector(&seed, "item1");
        let v2 = generate_item_vector(&seed, "item2");

        // Random vectors should have ~50% similarity
        let sim = hamming_similarity(&v1, &v2);
        assert!(sim > 0.4 && sim < 0.6);
    }
}
