// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Health proof generation pipeline: Winterfell STARK + Dilithium5 signing.
//!
//! Complete prove-and-sign flow:
//! 1. Generate REAL Winterfell STARK range proof
//! 2. Wrap in AuthenticatedProof with domain tag
//! 3. Sign with CRYSTALS-Dilithium5 (post-quantum)
//!
//! This is the client-side component of the DASTARK health attestation pipeline.

use std::time::Instant;

use sha2::{Digest, Sha256};

use crate::{
    generate_proof, AttestorRole, HealthProof, HealthProofType,
};
use mycelix_zkp_core::{
    AuthenticatedProof, DomainTag,
    domain::tag_health_attest,
    types::{BackendId, ProofMetadata},
};

#[cfg(feature = "verify-dilithium")]
use mycelix_zkp_core::dilithium::DilithiumKeypair;

/// Request to generate a health ZKP.
#[derive(Debug, Clone)]
pub struct HealthProofRequest {
    /// What to prove.
    pub proof_type: HealthProofType,
    /// The private value (e.g., blood pressure, age, A1C).
    pub value: u64,
    /// Minimum of the valid range (public).
    pub min: u64,
    /// Maximum of the valid range (public).
    pub max: u64,
    /// Patient identifier (hashed before use).
    pub patient_id: String,
    /// Private health data bytes (hashed for commitment).
    pub health_data: Vec<u8>,
    /// Who attested to the underlying data.
    pub attestor: AttestorRole,
}

/// Output from the prove-and-sign pipeline.
#[derive(Debug)]
pub struct HealthProofOutput {
    /// The health proof with REAL Winterfell STARK bytes.
    pub health_proof: HealthProof,
    /// The authenticated proof with Dilithium5 signature (if signing enabled).
    pub authenticated_proof: Option<AuthenticatedProof>,
    /// Time to generate the STARK proof (ms).
    pub prove_time_ms: f64,
    /// Time to sign with Dilithium5 (ms, 0 if not signed).
    pub sign_time_ms: f64,
    /// Total pipeline time (ms).
    pub total_time_ms: f64,
}

/// Generate a REAL Winterfell STARK health proof.
///
/// This calls `prove_range()` with actual Winterfell AIR circuit — not a stub.
pub fn prove(request: &HealthProofRequest) -> HealthProofOutput {
    let total_start = Instant::now();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let expiry = timestamp + 86400; // 24 hours

    // Generate REAL Winterfell STARK proof
    let prove_start = Instant::now();
    let health_proof = generate_proof(
        request.proof_type.clone(),
        &request.health_data,
        request.patient_id.as_bytes(),
        request.attestor.clone(),
        timestamp,
        expiry,
        request.value,
        request.min,
        request.max,
    );
    let prove_time = prove_start.elapsed();

    let total_time = total_start.elapsed();

    HealthProofOutput {
        health_proof,
        authenticated_proof: None,
        prove_time_ms: prove_time.as_secs_f64() * 1000.0,
        sign_time_ms: 0.0,
        total_time_ms: total_time.as_secs_f64() * 1000.0,
    }
}

/// Generate a REAL proof AND sign with Dilithium5.
///
/// Returns both the health proof and an AuthenticatedProof with PQ signature.
#[cfg(feature = "verify-dilithium")]
pub fn prove_and_sign(
    request: &HealthProofRequest,
    keypair: &DilithiumKeypair,
) -> HealthProofOutput {
    let total_start = Instant::now();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let expiry = timestamp + 86400;

    // 1. Generate REAL Winterfell STARK proof
    let prove_start = Instant::now();
    let health_proof = generate_proof(
        request.proof_type.clone(),
        &request.health_data,
        request.patient_id.as_bytes(),
        request.attestor.clone(),
        timestamp,
        expiry,
        request.value,
        request.min,
        request.max,
    );
    let prove_time = prove_start.elapsed();

    // 2. Construct AuthenticatedProof
    let domain_tag = tag_health_attest();
    let public_inputs_hash = {
        let h = Sha256::digest(format!("{}:{}:{}", request.min, request.max, request.value).as_bytes());
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&h);
        hash
    };

    let nonce: [u8; 32] = rand::random();

    let mut authenticated = AuthenticatedProof {
        proof: health_proof.proof_bytes.clone(),
        signature: vec![],
        metadata: ProofMetadata {
            domain_tag,
            protocol_version: 1,
            client_id: *keypair.client_id(),
            timestamp: timestamp as u64,
            nonce,
            backend: BackendId::Winterfell,
        },
        public_inputs_hash,
    };

    // 3. Sign with Dilithium5
    let sign_start = Instant::now();
    keypair.sign_proof(&mut authenticated)
        .expect("Dilithium5 signing failed");
    let sign_time = sign_start.elapsed();

    let total_time = total_start.elapsed();

    HealthProofOutput {
        health_proof,
        authenticated_proof: Some(authenticated),
        prove_time_ms: prove_time.as_secs_f64() * 1000.0,
        sign_time_ms: sign_time.as_secs_f64() * 1000.0,
        total_time_ms: total_time.as_secs_f64() * 1000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prove_vitals() {
        let request = HealthProofRequest {
            proof_type: HealthProofType::VitalsInRange,
            value: 120,
            min: 90,
            max: 180,
            patient_id: "did:mycelix:patient001".to_string(),
            health_data: b"bp:120/80,hr:72".to_vec(),
            attestor: AttestorRole::Physician,
        };

        let output = prove(&request);
        assert!(output.prove_time_ms > 0.0, "prove should take non-zero time");
        assert!(output.health_proof.proof_bytes.len() > 1000,
            "STARK proof should be >1KB, got {}", output.health_proof.proof_bytes.len());
        println!("Prove time: {:.1}ms, Proof size: {} bytes",
            output.prove_time_ms, output.health_proof.proof_bytes.len());
    }

    #[cfg(feature = "verify-dilithium")]
    #[test]
    fn test_prove_and_sign() {
        let keypair = DilithiumKeypair::generate();

        let request = HealthProofRequest {
            proof_type: HealthProofType::AgeRange { min_age: 18, max_age: Some(65) },
            value: 35,
            min: 18,
            max: 65,
            patient_id: "did:mycelix:patient002".to_string(),
            health_data: b"dob:1991-03-15".to_vec(),
            attestor: AttestorRole::Physician,
        };

        let output = prove_and_sign(&request, &keypair);
        assert!(output.authenticated_proof.is_some());
        let auth = output.authenticated_proof.unwrap();
        assert!(!auth.signature.is_empty(), "Dilithium signature should be non-empty");
        assert!(output.sign_time_ms > 0.0);
        println!("Prove: {:.1}ms, Sign: {:.1}ms, Total: {:.1}ms",
            output.prove_time_ms, output.sign_time_ms, output.total_time_ms);
        println!("Proof size: {} bytes, Signature size: {} bytes",
            auth.proof.len(), auth.signature.len());
    }
}
