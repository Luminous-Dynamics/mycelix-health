//! HDC Core - Hyperdimensional Computing Library
//!
//! Pure Rust implementation of hyperdimensional computing primitives
//! for genetic data encoding and privacy-preserving similarity search.
//!
//! # Features
//!
//! - Binary/bipolar hypervectors (10,000 dimensions)
//! - K-mer based DNA sequence encoding
//! - HLA typing for transplant matching
//! - SNP panel encoding
//! - Multiple similarity metrics (Hamming, Cosine, Jaccard)
//!
//! # Example
//!
//! ```rust
//! use hdc_core::{DnaEncoder, Seed};
//!
//! let seed = Seed::from_string("my-experiment-v1");
//! let encoder = DnaEncoder::new(seed, 6); // k=6
//!
//! let seq1 = "ACGTACGTACGTACGT";
//! let seq2 = "ACGTACGTACGTACGT";
//!
//! let enc1 = encoder.encode_sequence(seq1).unwrap();
//! let enc2 = encoder.encode_sequence(seq2).unwrap();
//!
//! let similarity = enc1.vector.normalized_cosine_similarity(&enc2.vector);
//! println!("Similarity: {:.3}", similarity);
//! ```

pub mod ops;
pub mod encoding;
pub mod similarity;

// Re-export commonly used types for convenience
pub use encoding::{
    DnaEncoder, HlaEncoder, SnpEncoder, EncodedSequence, HlaMatch,
    LocusWeightedHlaEncoder, LocusEncodedHla,
    AlleleHlaEncoder, AlleleEncodedHla,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Standard hypervector dimension
pub const HYPERVECTOR_DIM: usize = 10_000;

/// Number of bytes needed to store HYPERVECTOR_DIM bits
pub const HYPERVECTOR_BYTES: usize = (HYPERVECTOR_DIM + 7) / 8;

/// Default k-mer length for DNA encoding
pub const DEFAULT_KMER_LENGTH: u8 = 6;

/// A 32-byte seed for reproducible hypervector generation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seed(pub [u8; 32]);

impl Seed {
    /// Create a seed from a string (hashed to 32 bytes)
    pub fn from_string(s: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&result);
        Seed(seed)
    }

    /// Create a seed from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Seed(bytes)
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Default for Seed {
    fn default() -> Self {
        Seed([0u8; 32])
    }
}

/// A hypervector - binary representation of high-dimensional data
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hypervector {
    /// Packed binary data (10,000 bits = 1,250 bytes)
    data: Vec<u8>,
}

impl Hypervector {
    /// Create a new hypervector from raw bytes
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, HdcError> {
        if data.len() != HYPERVECTOR_BYTES {
            return Err(HdcError::InvalidDimension {
                expected: HYPERVECTOR_BYTES,
                got: data.len(),
            });
        }
        Ok(Hypervector { data })
    }

    /// Create a zero hypervector
    pub fn zero() -> Self {
        Hypervector {
            data: vec![0u8; HYPERVECTOR_BYTES],
        }
    }

    /// Create a random hypervector from a seed and identifier
    pub fn random(seed: &Seed, identifier: &str) -> Self {
        let data = ops::generate_item_vector(seed.as_bytes(), identifier);
        Hypervector { data }
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the raw bytes
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Bind (XOR) two hypervectors
    pub fn bind(&self, other: &Hypervector) -> Hypervector {
        let data = ops::bind(&self.data, &other.data);
        Hypervector { data }
    }

    /// Permute (rotate) the hypervector
    pub fn permute(&self, shift: usize) -> Hypervector {
        let data = ops::permute(&self.data, shift);
        Hypervector { data }
    }

    /// Get a single bit value
    pub fn get_bit(&self, index: usize) -> bool {
        if index >= HYPERVECTOR_DIM {
            return false;
        }
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        (self.data[byte_idx] >> bit_idx) & 1 == 1
    }

    /// Set a single bit value
    pub fn set_bit(&mut self, index: usize, value: bool) {
        if index >= HYPERVECTOR_DIM {
            return;
        }
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        if value {
            self.data[byte_idx] |= 1 << bit_idx;
        } else {
            self.data[byte_idx] &= !(1 << bit_idx);
        }
    }

    /// Count the number of 1 bits
    pub fn popcount(&self) -> usize {
        self.data.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// Hamming similarity (fraction of matching bits)
    pub fn hamming_similarity(&self, other: &Hypervector) -> f64 {
        ops::hamming_similarity(&self.data, &other.data)
    }

    /// Cosine similarity (bipolar interpretation: -1 to +1)
    pub fn cosine_similarity(&self, other: &Hypervector) -> f64 {
        ops::cosine_similarity(&self.data, &other.data)
    }

    /// Normalized cosine similarity (0 to 1)
    pub fn normalized_cosine_similarity(&self, other: &Hypervector) -> f64 {
        ops::normalized_cosine_similarity(&self.data, &other.data)
    }

    /// Jaccard similarity (intersection over union)
    pub fn jaccard_similarity(&self, other: &Hypervector) -> f64 {
        let intersection: usize = self.data.iter()
            .zip(other.data.iter())
            .map(|(a, b)| (a & b).count_ones() as usize)
            .sum();

        let union: usize = self.data.iter()
            .zip(other.data.iter())
            .map(|(a, b)| (a | b).count_ones() as usize)
            .sum();

        if union == 0 {
            1.0
        } else {
            intersection as f64 / union as f64
        }
    }
}

/// Bundle multiple hypervectors using majority voting
pub fn bundle(vectors: &[&Hypervector]) -> Hypervector {
    let refs: Vec<&[u8]> = vectors.iter().map(|v| v.as_bytes()).collect();
    let data = ops::bundle(&refs);
    Hypervector { data }
}

/// Weighted bundle using thresholded sums
pub fn weighted_bundle(vectors: &[(&Hypervector, f64)]) -> Hypervector {
    let weighted: Vec<(&[u8], f64)> = vectors.iter()
        .map(|(v, w)| (v.as_bytes(), *w))
        .collect();
    let data = ops::weighted_bundle(&weighted);
    Hypervector { data }
}

/// Errors that can occur in HDC operations
#[derive(Debug, Clone, PartialEq)]
pub enum HdcError {
    /// Invalid vector dimension
    InvalidDimension { expected: usize, got: usize },
    /// Sequence is too short for the k-mer length
    SequenceTooShort { length: usize, kmer_length: u8 },
    /// Invalid nucleotide in sequence
    InvalidNucleotide(char),
    /// Empty input
    EmptyInput,
    /// Other error
    Other(String),
}

impl std::fmt::Display for HdcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HdcError::InvalidDimension { expected, got } => {
                write!(f, "Invalid dimension: expected {} bytes, got {}", expected, got)
            }
            HdcError::SequenceTooShort { length, kmer_length } => {
                write!(f, "Sequence too short: {} < k-mer length {}", length, kmer_length)
            }
            HdcError::InvalidNucleotide(c) => {
                write!(f, "Invalid nucleotide: '{}'", c)
            }
            HdcError::EmptyInput => {
                write!(f, "Empty input")
            }
            HdcError::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for HdcError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_from_string() {
        let seed1 = Seed::from_string("test");
        let seed2 = Seed::from_string("test");
        let seed3 = Seed::from_string("different");

        assert_eq!(seed1, seed2);
        assert_ne!(seed1, seed3);
    }

    #[test]
    fn test_hypervector_size() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");
        assert_eq!(hv.as_bytes().len(), HYPERVECTOR_BYTES);
    }

    #[test]
    fn test_bind_is_self_inverse() {
        let seed = Seed::from_string("test");
        let a = Hypervector::random(&seed, "a");
        let b = Hypervector::random(&seed, "b");

        let bound = a.bind(&b);
        let unbound = bound.bind(&b);

        assert_eq!(a, unbound);
    }

    #[test]
    fn test_self_similarity() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");

        assert!((hv.hamming_similarity(&hv) - 1.0).abs() < 0.001);
        assert!((hv.cosine_similarity(&hv) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bundle_majority() {
        let seed = Seed::from_string("test");
        let a = Hypervector::random(&seed, "a");
        let b = Hypervector::random(&seed, "b");
        let c = Hypervector::random(&seed, "c");

        let bundled = bundle(&[&a, &b, &c]);

        // Bundled vector should exist and have correct size
        assert_eq!(bundled.as_bytes().len(), HYPERVECTOR_BYTES);
    }
}
