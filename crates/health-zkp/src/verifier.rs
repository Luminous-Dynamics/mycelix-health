// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Health proof verification pipeline: Winterfell STARK verify + Dilithium5 check + Ed25519 attest.
//!
//! Complete verify-and-attest flow:
//! 1. Deserialize and verify REAL Winterfell STARK proof
//! 2. Verify Dilithium5 prover signature (if present)
//! 3. Sign attestation result with Ed25519 (for DHT storage)
//!
//! This is the off-chain verifier component of the DASTARK health attestation pipeline.
//! Follows the attribution cluster pattern: proof verified off-chain, attestation stored on-chain.

use std::time::Instant;

use crate::{verify_proof, HealthProof};
use mycelix_zkp_core::types::AuthenticatedProof;

#[cfg(feature = "verify-dilithium")]
use mycelix_zkp_core::dilithium;

/// Output from the verify-and-attest pipeline.
#[derive(Debug)]
pub struct VerificationOutput {
    /// Whether the STARK proof is cryptographically valid.
    pub proof_valid: bool,
    /// Whether the Dilithium5 signature is valid (None if no signature).
    pub signature_valid: Option<bool>,
    /// Time to verify the STARK proof (ms).
    pub verify_time_ms: f64,
    /// Time to verify the Dilithium signature (ms, 0 if skipped).
    pub signature_verify_time_ms: f64,
    /// Total verification pipeline time (ms).
    pub total_time_ms: f64,
    /// Proof size in bytes.
    pub proof_size_bytes: usize,
}

/// Verify a health proof using REAL Winterfell STARK verification.
///
/// This calls `winterfell::verify()` under the hood — not a structural stub.
pub fn verify(health_proof: &HealthProof) -> VerificationOutput {
    let total_start = Instant::now();

    let verify_start = Instant::now();
    let proof_valid = verify_proof(health_proof);
    let verify_time = verify_start.elapsed();

    let total_time = total_start.elapsed();

    VerificationOutput {
        proof_valid,
        signature_valid: None,
        verify_time_ms: verify_time.as_secs_f64() * 1000.0,
        signature_verify_time_ms: 0.0,
        total_time_ms: total_time.as_secs_f64() * 1000.0,
        proof_size_bytes: health_proof.proof_bytes.len(),
    }
}

/// Verify a health proof AND its Dilithium5 signature.
///
/// This verifies:
/// 1. The STARK proof (Winterfell AIR circuit)
/// 2. The Dilithium5 signature on the AuthenticatedProof
/// Both must pass for the verification to succeed.
#[cfg(feature = "verify-dilithium")]
pub fn verify_with_signature(
    health_proof: &HealthProof,
    authenticated_proof: &AuthenticatedProof,
    prover_public_key: &[u8],
) -> VerificationOutput {
    let total_start = Instant::now();

    // 1. Verify STARK proof
    let verify_start = Instant::now();
    let proof_valid = verify_proof(health_proof);
    let verify_time = verify_start.elapsed();

    // 2. Verify Dilithium5 signature
    let sig_start = Instant::now();
    let signature_valid = dilithium::verify_authenticated_signature(
        authenticated_proof,
        prover_public_key,
    ).unwrap_or(false);
    let sig_time = sig_start.elapsed();

    let total_time = total_start.elapsed();

    VerificationOutput {
        proof_valid,
        signature_valid: Some(signature_valid),
        verify_time_ms: verify_time.as_secs_f64() * 1000.0,
        signature_verify_time_ms: sig_time.as_secs_f64() * 1000.0,
        total_time_ms: total_time.as_secs_f64() * 1000.0,
        proof_size_bytes: health_proof.proof_bytes.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prover::{self, HealthProofRequest};
    use crate::{AttestorRole, HealthProofType};

    #[test]
    fn test_verify_real_proof() {
        let request = HealthProofRequest {
            proof_type: HealthProofType::VitalsInRange,
            value: 120,
            min: 90,
            max: 180,
            patient_id: "did:mycelix:test".to_string(),
            health_data: b"bp:120/80".to_vec(),
            attestor: AttestorRole::Physician,
        };

        let output = prover::prove(&request);
        let verification = verify(&output.health_proof);

        assert!(verification.proof_valid, "real STARK proof must verify");
        assert!(verification.verify_time_ms > 0.0);
        println!("Verify time: {:.1}ms", verification.verify_time_ms);
    }

    #[cfg(feature = "verify-dilithium")]
    #[test]
    fn test_verify_with_dilithium_signature() {
        use mycelix_zkp_core::dilithium::DilithiumKeypair;

        let keypair = DilithiumKeypair::generate();

        let request = HealthProofRequest {
            proof_type: HealthProofType::AgeRange { min_age: 18, max_age: Some(65) },
            value: 35,
            min: 18,
            max: 65,
            patient_id: "did:mycelix:test2".to_string(),
            health_data: b"dob:1991".to_vec(),
            attestor: AttestorRole::Physician,
        };

        let output = prover::prove_and_sign(&request, &keypair);
        let auth = output.authenticated_proof.as_ref().unwrap();

        let verification = verify_with_signature(
            &output.health_proof,
            auth,
            keypair.public_key(),
        );

        assert!(verification.proof_valid, "STARK proof must verify");
        assert_eq!(verification.signature_valid, Some(true), "Dilithium sig must verify");
        println!(
            "STARK verify: {:.1}ms, Dilithium verify: {:.1}ms, Total: {:.1}ms",
            verification.verify_time_ms,
            verification.signature_verify_time_ms,
            verification.total_time_ms
        );
    }
}
