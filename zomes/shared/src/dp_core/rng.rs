//! Cryptographically Secure Random Number Generation
//!
//! Provides CSPRNG functionality for differential privacy mechanisms.
//! Uses `getrandom` crate which provides OS-level entropy on all platforms
//! including WASM (via Web Crypto API).
//!
//! # Security Properties
//!
//! - Uses OS entropy source (urandom/CryptGenRandom/Web Crypto)
//! - Resistant to prediction attacks
//! - Suitable for cryptographic operations
//!
//! # Why Not sys_time()?
//!
//! The previous implementation used `sys_time()` for randomness:
//! ```ignore
//! let u = (sys_time().unwrap().as_micros() % 1_000_000) as f64 / 1_000_000.0;
//! ```
//!
//! This is INSECURE because:
//! 1. Time is predictable/observable
//! 2. Multiple calls in the same microsecond return identical values
//! 3. An attacker can reconstruct the "random" noise

use serde::{Deserialize, Serialize};

/// Error type for RNG operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RngError {
    /// Failed to generate random bytes
    EntropyError(String),
    /// Invalid parameter
    InvalidParameter(String),
}

impl std::fmt::Display for RngError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RngError::EntropyError(msg) => write!(f, "Entropy error: {}", msg),
            RngError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
        }
    }
}

/// Secure random number generator using OS entropy
pub struct SecureRng;

impl SecureRng {
    /// Fill a buffer with cryptographically secure random bytes
    ///
    /// Uses the OS-provided entropy source via `getrandom`.
    ///
    /// # Arguments
    /// * `buffer` - Mutable byte slice to fill with random data
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(RngError)` if entropy source fails
    pub fn fill_bytes(buffer: &mut [u8]) -> Result<(), RngError> {
        getrandom::fill(buffer)
            .map_err(|e| RngError::EntropyError(format!("Failed to get entropy: {:?}", e)))
    }

    /// Generate a random u64
    pub fn random_u64() -> Result<u64, RngError> {
        let mut bytes = [0u8; 8];
        Self::fill_bytes(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    /// Generate a random f64 uniformly distributed in [0, 1)
    ///
    /// Uses the standard technique of generating 53 random bits
    /// (the mantissa precision of f64) and dividing by 2^53.
    ///
    /// # Returns
    /// A uniformly distributed f64 in the range [0, 1)
    pub fn random_f64_uniform() -> Result<f64, RngError> {
        let mut bytes = [0u8; 8];
        Self::fill_bytes(&mut bytes)?;

        // Use 53 bits (f64 mantissa precision) for uniform [0, 1)
        let value = u64::from_le_bytes(bytes);
        let masked = value >> 11; // Keep top 53 bits
        Ok(masked as f64 / (1u64 << 53) as f64)
    }

    /// Generate a random f64 uniformly distributed in (-0.5, 0.5)
    ///
    /// This range is specifically needed for the Laplace mechanism's
    /// inverse CDF transformation.
    ///
    /// # Returns
    /// A uniformly distributed f64 in the range (-0.5, 0.5), excluding exactly 0
    pub fn random_f64_centered() -> Result<f64, RngError> {
        loop {
            let u = Self::random_f64_uniform()?;
            let centered = u - 0.5;
            // Avoid exactly 0 (probability essentially 0, but handle it)
            if centered.abs() > 1e-15 {
                return Ok(centered);
            }
        }
    }

    /// Generate two independent uniform random values for Box-Muller transform
    ///
    /// # Returns
    /// Tuple of (u1, u2) where both are in (0, 1), avoiding exactly 0
    pub fn random_pair_for_box_muller() -> Result<(f64, f64), RngError> {
        let mut bytes = [0u8; 16];
        Self::fill_bytes(&mut bytes)?;

        // Generate two independent uniform values
        let v1 = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let v2 = u64::from_le_bytes(bytes[8..16].try_into().unwrap());

        let u1 = (v1 >> 11) as f64 / (1u64 << 53) as f64;
        let u2 = (v2 >> 11) as f64 / (1u64 << 53) as f64;

        // Ensure neither is exactly 0 (for ln() in Box-Muller)
        let u1 = if u1 < 1e-15 { 1e-15 } else { u1 };
        let u2 = if u2 < 1e-15 { 1e-15 } else { u2 };

        Ok((u1, u2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_bytes() {
        let mut buffer1 = [0u8; 32];
        let mut buffer2 = [0u8; 32];

        SecureRng::fill_bytes(&mut buffer1).unwrap();
        SecureRng::fill_bytes(&mut buffer2).unwrap();

        // Buffers should be different (with overwhelming probability)
        assert_ne!(buffer1, buffer2);
        // Buffers should not be all zeros
        assert_ne!(buffer1, [0u8; 32]);
    }

    #[test]
    fn test_random_f64_uniform_range() {
        for _ in 0..1000 {
            let value = SecureRng::random_f64_uniform().unwrap();
            assert!(value >= 0.0);
            assert!(value < 1.0);
        }
    }

    #[test]
    fn test_random_f64_centered_range() {
        for _ in 0..1000 {
            let value = SecureRng::random_f64_centered().unwrap();
            assert!(value > -0.5);
            assert!(value < 0.5);
            assert!(value.abs() > 1e-15); // Not exactly 0
        }
    }

    #[test]
    fn test_random_pair_positive() {
        for _ in 0..1000 {
            let (u1, u2) = SecureRng::random_pair_for_box_muller().unwrap();
            assert!(u1 > 0.0);
            assert!(u1 <= 1.0);
            assert!(u2 > 0.0);
            assert!(u2 <= 1.0);
        }
    }
}
