// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Homomorphic federated learning — Paillier-encrypted gradient aggregation.
//!
//! Patients encrypt gradients locally. The aggregator sums encrypted gradients
//! (Paillier addition is homomorphic: E(a)+E(b)=E(a+b)). Only the aggregate
//! is decrypted via k-of-n threshold. Even a compromised aggregator learns
//! nothing about individual contributions.

use serde::{Deserialize, Serialize};
use crate::{HealthGradient, HEALTH_GRADIENT_DIM};

/// Encrypted health gradient — Paillier-encrypted feature vector.
/// Each feature is independently encrypted as a fixed-point integer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedHealthGradient {
    /// Patient pseudonymous ID.
    pub node_id: String,
    /// Paillier-encrypted features (one ciphertext per feature).
    /// Each is a serialized BigUint.
    pub encrypted_features: Vec<Vec<u8>>,
    /// LOINC family code.
    pub loinc_family: String,
    /// FL round number.
    pub round: u64,
}

/// Fixed-point encoding for gradient features.
/// Maps f32 [0.0, 1.0] → i64 [0, SCALE] for Paillier encryption.
const FIXED_POINT_SCALE: i64 = 1_000_000; // 6 decimal places of precision

/// Encode a float gradient feature as a fixed-point integer.
pub fn encode_feature(value: f32) -> i64 {
    (value.clamp(0.0, 1.0) as f64 * FIXED_POINT_SCALE as f64) as i64
}

/// Decode a fixed-point integer back to float.
pub fn decode_feature(encoded: i64, count: usize) -> f32 {
    (encoded as f64 / (FIXED_POINT_SCALE as f64 * count as f64)) as f32
}

/// Simulated Paillier encryption for a single feature.
/// In production, this uses the real Paillier crate from mycelix-core.
/// Here we demonstrate the protocol flow with a simple additive scheme.
pub fn paillier_encrypt_feature(value: i64, _public_key: &[u8]) -> Vec<u8> {
    // Placeholder: encode as little-endian i64 bytes
    // Real implementation: use PaillierKeyPair::encrypt() from mycelix-core
    value.to_le_bytes().to_vec()
}

/// Simulated Paillier homomorphic addition.
/// E(a) + E(b) = E(a + b) without decryption.
pub fn paillier_add(ct_a: &[u8], ct_b: &[u8]) -> Vec<u8> {
    // Placeholder: decode, add, re-encode
    // Real implementation: BigUint modular multiplication (Paillier addition)
    let a = i64::from_le_bytes(ct_a[..8].try_into().unwrap_or([0; 8]));
    let b = i64::from_le_bytes(ct_b[..8].try_into().unwrap_or([0; 8]));
    (a + b).to_le_bytes().to_vec()
}

/// Simulated Paillier decryption.
pub fn paillier_decrypt(ciphertext: &[u8], _private_key: &[u8]) -> i64 {
    // Placeholder: decode directly
    // Real implementation: use PaillierKeyPair::decrypt() from mycelix-core
    i64::from_le_bytes(ciphertext[..8].try_into().unwrap_or([0; 8]))
}

/// Encrypt a health gradient with Paillier.
/// DP noise should be applied BEFORE this function (encrypt the noised gradient).
pub fn encrypt_gradient(
    gradient: &HealthGradient,
    public_key: &[u8],
) -> EncryptedHealthGradient {
    let encrypted_features = gradient.features.iter()
        .map(|&f| {
            let encoded = encode_feature(f);
            paillier_encrypt_feature(encoded, public_key)
        })
        .collect();

    EncryptedHealthGradient {
        node_id: gradient.node_id.clone(),
        encrypted_features,
        loinc_family: gradient.loinc_family.clone(),
        round: gradient.round,
    }
}

/// Aggregate encrypted gradients homomorphically.
/// Returns the encrypted sum (to be decrypted by threshold key holders).
pub fn aggregate_encrypted(
    gradients: &[EncryptedHealthGradient],
) -> Result<Vec<Vec<u8>>, String> {
    if gradients.is_empty() {
        return Err("No gradients to aggregate".into());
    }

    let dim = gradients[0].encrypted_features.len();
    if dim != HEALTH_GRADIENT_DIM {
        return Err(format!("Expected {} features, got {}", HEALTH_GRADIENT_DIM, dim));
    }

    // Homomorphic summation: for each feature dimension, add all ciphertexts
    let mut aggregate = gradients[0].encrypted_features.clone();
    for gradient in &gradients[1..] {
        for (i, ct) in gradient.encrypted_features.iter().enumerate() {
            aggregate[i] = paillier_add(&aggregate[i], ct);
        }
    }

    Ok(aggregate)
}

/// Decrypt the aggregate and compute the mean.
pub fn decrypt_aggregate(
    encrypted_sum: &[Vec<u8>],
    private_key: &[u8],
    count: usize,
) -> Vec<f32> {
    encrypted_sum.iter()
        .map(|ct| {
            let sum = paillier_decrypt(ct, private_key);
            decode_feature(sum, count)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_gradient;

    #[test]
    fn encrypted_aggregation_matches_plaintext() {
        let fake_pk = vec![0u8; 32];
        let fake_sk = vec![0u8; 32];

        // Create 5 gradients
        let gradients: Vec<_> = (0..5).map(|i| {
            extract_gradient(
                &format!("{}", 80 + i * 5),
                "70-100", false, false, 1.0, true, "2345-7",
                &format!("patient-{}", i), 1,
            )
        }).collect();

        // Plaintext average
        let mut plain_sum = vec![0.0f32; HEALTH_GRADIENT_DIM];
        for g in &gradients {
            for (i, &f) in g.features.iter().enumerate() {
                plain_sum[i] += f;
            }
        }
        let plain_avg: Vec<f32> = plain_sum.iter().map(|s| s / 5.0).collect();

        // Encrypted aggregation
        let encrypted: Vec<_> = gradients.iter()
            .map(|g| encrypt_gradient(g, &fake_pk))
            .collect();

        let agg = aggregate_encrypted(&encrypted).unwrap();
        let decrypted_avg = decrypt_aggregate(&agg, &fake_sk, 5);

        // Compare (allow small fixed-point rounding error)
        for (i, (p, d)) in plain_avg.iter().zip(decrypted_avg.iter()).enumerate() {
            assert!(
                (p - d).abs() < 0.01,
                "Feature {}: plaintext {:.4} vs encrypted {:.4}", i, p, d
            );
        }
    }

    #[test]
    fn fixed_point_roundtrip() {
        for val in [0.0, 0.5, 1.0, 0.123456, 0.999999] {
            let encoded = encode_feature(val);
            let decoded = decode_feature(encoded, 1);
            assert!((val - decoded).abs() < 0.001, "{} → {} → {}", val, encoded, decoded);
        }
    }
}
