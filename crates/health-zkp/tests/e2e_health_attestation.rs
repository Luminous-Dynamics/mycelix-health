// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end health ZKP attestation tests.
//!
//! These tests exercise the COMPLETE pipeline:
//!   prove_range() → Dilithium5 sign → winterfell::verify() → Dilithium verify
//!
//! Every operation uses REAL cryptographic primitives — not mocked.
//! This is the reproducible benchmark for the paper.

use mycelix_health_zkp::prover::{self, HealthProofRequest};
use mycelix_health_zkp::verifier;
use mycelix_health_zkp::{AttestorRole, HealthProofType, ProofSystem, verify_proof};

#[cfg(feature = "verify-dilithium")]
use mycelix_zkp_core::dilithium::DilithiumKeypair;

// ═══════════════════════════════════════════════════════════════════
// E2E: Prove + Verify (Winterfell STARK only)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn e2e_vitals_in_range() {
    println!("\n=== E2E: Blood Pressure 120 ∈ [90, 180] ===");

    let request = HealthProofRequest {
        proof_type: HealthProofType::VitalsInRange,
        value: 120,
        min: 90,
        max: 180,
        patient_id: "did:mycelix:patient001".to_string(),
        health_data: b"bp_systolic:120".to_vec(),
        attestor: AttestorRole::Physician,
    };

    // Prove (REAL Winterfell STARK)
    let proof_output = prover::prove(&request);
    assert_eq!(proof_output.health_proof.metadata.system, ProofSystem::WinterfellStark);
    assert!(proof_output.health_proof.proof_bytes.len() > 1000,
        "STARK proof must be >1KB, got {}", proof_output.health_proof.proof_bytes.len());

    // Verify (REAL winterfell::verify)
    let verify_output = verifier::verify(&proof_output.health_proof);
    assert!(verify_output.proof_valid, "REAL STARK proof must verify");

    println!("  Prove:  {:.1} ms", proof_output.prove_time_ms);
    println!("  Verify: {:.1} ms", verify_output.verify_time_ms);
    println!("  Proof:  {} bytes ({:.1} KB)", verify_output.proof_size_bytes,
        verify_output.proof_size_bytes as f64 / 1024.0);
    println!("  PASSED");
}

#[test]
fn e2e_age_range() {
    println!("\n=== E2E: Age 35 ∈ [18, 65] ===");

    let request = HealthProofRequest {
        proof_type: HealthProofType::AgeRange { min_age: 18, max_age: Some(65) },
        value: 35,
        min: 18,
        max: 65,
        patient_id: "did:mycelix:patient002".to_string(),
        health_data: b"dob:1991-03-15".to_vec(),
        attestor: AttestorRole::Physician,
    };

    let proof_output = prover::prove(&request);
    assert_eq!(proof_output.health_proof.metadata.system, ProofSystem::WinterfellStark);

    let verify_output = verifier::verify(&proof_output.health_proof);
    assert!(verify_output.proof_valid);

    println!("  Prove:  {:.1} ms", proof_output.prove_time_ms);
    println!("  Verify: {:.1} ms", verify_output.verify_time_ms);
    println!("  PASSED");
}

#[test]
fn e2e_lab_threshold() {
    println!("\n=== E2E: A1C 54 (5.4%) ∈ [0, 70] ===");

    let request = HealthProofRequest {
        proof_type: HealthProofType::LabThreshold {
            loinc_code: "4548-4".to_string(),
            threshold: 7.0,
            direction: mycelix_health_zkp::ThresholdDirection::Below,
        },
        value: 54, // 5.4% in tenths
        min: 0,
        max: 70,  // 7.0% in tenths
        patient_id: "did:mycelix:patient003".to_string(),
        health_data: b"a1c:5.4".to_vec(),
        attestor: AttestorRole::Laboratory,
    };

    let proof_output = prover::prove(&request);
    assert_eq!(proof_output.health_proof.metadata.system, ProofSystem::WinterfellStark);

    let verify_output = verifier::verify(&proof_output.health_proof);
    assert!(verify_output.proof_valid);

    println!("  Prove:  {:.1} ms", proof_output.prove_time_ms);
    println!("  Verify: {:.1} ms", verify_output.verify_time_ms);
    println!("  PASSED");
}

#[test]
fn e2e_tampered_proof_rejected() {
    println!("\n=== E2E: Tampered Proof Rejection ===");

    let request = HealthProofRequest {
        proof_type: HealthProofType::VitalsInRange,
        value: 120,
        min: 90,
        max: 180,
        patient_id: "did:mycelix:attacker".to_string(),
        health_data: b"bp:120".to_vec(),
        attestor: AttestorRole::PatientSelf,
    };

    let mut proof_output = prover::prove(&request);

    // Tamper with proof bytes
    if !proof_output.health_proof.proof_bytes.is_empty() {
        proof_output.health_proof.proof_bytes[0] ^= 0xFF;
    }

    let verify_output = verifier::verify(&proof_output.health_proof);
    assert!(!verify_output.proof_valid, "tampered proof MUST be rejected");

    println!("  Tampered proof correctly rejected");
    println!("  PASSED");
}

// ═══════════════════════════════════════════════════════════════════
// E2E: Prove + Sign + Verify + Signature Check (Full DASTARK pipeline)
// ═══════════════════════════════════════════════════════════════════

#[cfg(feature = "verify-dilithium")]
#[test]
fn e2e_full_dastark_pipeline() {
    println!("\n=== E2E: Full DASTARK Pipeline (STARK + Dilithium5) ===");

    let keypair = DilithiumKeypair::generate();

    let request = HealthProofRequest {
        proof_type: HealthProofType::VitalsInRange,
        value: 120,
        min: 90,
        max: 180,
        patient_id: "did:mycelix:patient001".to_string(),
        health_data: b"bp:120/80,hr:72,temp:98.6".to_vec(),
        attestor: AttestorRole::Physician,
    };

    // 1. Prove + Sign (REAL Winterfell STARK + REAL Dilithium5)
    let proof_output = prover::prove_and_sign(&request, &keypair);
    let auth = proof_output.authenticated_proof.as_ref()
        .expect("authenticated proof should be present");

    assert!(!auth.signature.is_empty(), "Dilithium signature must be non-empty");
    assert_eq!(proof_output.health_proof.metadata.system, ProofSystem::WinterfellStark);

    // 2. Verify STARK + Dilithium (REAL verification of both)
    let verify_output = verifier::verify_with_signature(
        &proof_output.health_proof,
        auth,
        keypair.public_key(),
    );

    assert!(verify_output.proof_valid, "STARK proof must verify");
    assert_eq!(verify_output.signature_valid, Some(true), "Dilithium sig must verify");

    // 3. Print paper-ready metrics
    println!("  --- PAPER METRICS ---");
    println!("  STARK prove:       {:.1} ms", proof_output.prove_time_ms);
    println!("  Dilithium5 sign:   {:.1} ms", proof_output.sign_time_ms);
    println!("  STARK verify:      {:.1} ms", verify_output.verify_time_ms);
    println!("  Dilithium5 verify: {:.1} ms", verify_output.signature_verify_time_ms);
    println!("  Total pipeline:    {:.1} ms", proof_output.total_time_ms + verify_output.total_time_ms);
    println!("  Proof size:        {} bytes ({:.1} KB)", verify_output.proof_size_bytes,
        verify_output.proof_size_bytes as f64 / 1024.0);
    println!("  Signature size:    {} bytes", auth.signature.len());
    println!("  PASSED — All REAL, nothing mocked");
}

#[cfg(feature = "verify-dilithium")]
#[test]
fn e2e_wrong_dilithium_key_rejected() {
    println!("\n=== E2E: Wrong Dilithium Key Rejection ===");

    let prover_key = DilithiumKeypair::generate();
    let wrong_key = DilithiumKeypair::generate();

    let request = HealthProofRequest {
        proof_type: HealthProofType::VitalsInRange,
        value: 120,
        min: 90,
        max: 180,
        patient_id: "did:mycelix:attacker".to_string(),
        health_data: b"bp:120".to_vec(),
        attestor: AttestorRole::PatientSelf,
    };

    let proof_output = prover::prove_and_sign(&request, &prover_key);
    let auth = proof_output.authenticated_proof.as_ref().unwrap();

    // Verify with WRONG key
    let verify_output = verifier::verify_with_signature(
        &proof_output.health_proof,
        auth,
        wrong_key.public_key(), // WRONG KEY
    );

    // STARK proof is still valid (it's the same proof)
    assert!(verify_output.proof_valid, "STARK proof is valid regardless of key");
    // But Dilithium signature check fails
    assert_eq!(verify_output.signature_valid, Some(false),
        "wrong Dilithium key MUST be rejected");

    println!("  STARK valid, Dilithium rejected (correct behavior)");
    println!("  PASSED");
}
