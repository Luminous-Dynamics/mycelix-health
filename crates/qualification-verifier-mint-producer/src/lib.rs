#![forbid(unsafe_code)]
//! Isolated producer-side engine for Health qualification-verifier mint receipts.
//!
//! This crate composes the portable 009A contract with the real 009B hybrid
//! verifier and adds one-time challenge consumption plus verifier-owned signing
//! state. It deliberately has no HTTP/Unix-socket transport and no product-side
//! authority mint.
//!
//! The producer theorem is intentionally narrow:
//!
//! ```text
//! consume one-time challenge
//! -> validate exact request-bound verifier-owned evidence
//! -> construct exact 009A statement
//! -> Ed25519 AND ML-DSA-65 sign the same bytes
//! -> self-verify through 009B + current enrollment
//! -> hybrid-sign a producer envelope binding freshness/policy evidence
//! ```
//!
//! `ProviderAuthenticatedMintReceiptV1` is portable evidence. It is not product
//! authority. A separate consumer-side verifier must authenticate it before any
//! private product token can exist.

use std::collections::HashMap;

use ed25519_dalek::{Signer as _, SigningKey as Ed25519SigningKey};
use ml_dsa::{
    B32, MlDsa65, Signature as MlDsaSignature, SigningKey as MlDsaSigningKey,
    signature::{Keypair as _, Signer as _},
};
use mycelix_qualification_verifier_mint_contract as contract;
use mycelix_qualification_verifier_mint_hybrid_crypto as crypto;
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest as _, Sha256};

const REQUEST_COMMITMENT_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-REQUEST-COMMITMENT-V1\0";
const CHALLENGE_CONSUMPTION_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-CHALLENGE-CONSUMPTION-V1\0";
const POLICY_SNAPSHOT_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-POLICY-SNAPSHOT-V1\0";
const LOGICAL_VERIFIER_ID_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-LOGICAL-ID-V1\0";
const RECEIPT_COMMITMENT_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-MINT-RECEIPT-COMMITMENT-V1\0";
const PRODUCER_ATTESTATION_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-PRODUCER-ATTESTATION-V1\0";
const PRODUCER_ATTESTATION_SCHEMA_V1: u16 = 1;

pub const ED25519_PUBLIC_KEY_LEN: usize = 32;
pub const ML_DSA_65_PUBLIC_KEY_LEN: usize = crypto::ML_DSA_65_PK_LEN;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IssuedVerifierChallengeV1 {
    challenge: contract::Nonce,
    issued_at_micros: i64,
    expires_at_micros: i64,
}

impl IssuedVerifierChallengeV1 {
    pub fn challenge(&self) -> contract::Nonce {
        self.challenge
    }

    pub fn issued_at_micros(&self) -> i64 {
        self.issued_at_micros
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChallengeRecordV1 {
    issued_at_micros: i64,
    expires_at_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChallengeConsumptionV1 {
    challenge: contract::Nonce,
    issued_at_micros: i64,
    expires_at_micros: i64,
    consumed_at_micros: i64,
}

#[derive(Debug, Default)]
struct InMemoryChallengeStoreV1 {
    active: HashMap<contract::Nonce, ChallengeRecordV1>,
}

impl InMemoryChallengeStoreV1 {
    fn issue(
        &mut self,
        now_micros: i64,
        lifetime_micros: i64,
    ) -> Result<IssuedVerifierChallengeV1, ProducerError> {
        if now_micros < 0 || lifetime_micros <= 0 {
            return Err(ProducerError::InvalidChallengeWindow);
        }
        let expires_at_micros = now_micros
            .checked_add(lifetime_micros)
            .ok_or(ProducerError::TimeOverflow)?;

        for _ in 0..16 {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            let Ok(challenge) = contract::Nonce::new(bytes) else {
                continue;
            };
            if self.active.contains_key(&challenge) {
                continue;
            }
            self.active.insert(
                challenge,
                ChallengeRecordV1 {
                    issued_at_micros: now_micros,
                    expires_at_micros,
                },
            );
            return Ok(IssuedVerifierChallengeV1 {
                challenge,
                issued_at_micros: now_micros,
                expires_at_micros,
            });
        }
        Err(ProducerError::ChallengeGenerationExhausted)
    }

    #[cfg(test)]
    fn issue_exact(
        &mut self,
        challenge: contract::Nonce,
        now_micros: i64,
        lifetime_micros: i64,
    ) -> Result<IssuedVerifierChallengeV1, ProducerError> {
        if self.active.contains_key(&challenge) {
            return Err(ProducerError::ChallengeCollision);
        }
        if now_micros < 0 || lifetime_micros <= 0 {
            return Err(ProducerError::InvalidChallengeWindow);
        }
        let expires_at_micros = now_micros
            .checked_add(lifetime_micros)
            .ok_or(ProducerError::TimeOverflow)?;
        self.active.insert(
            challenge,
            ChallengeRecordV1 {
                issued_at_micros: now_micros,
                expires_at_micros,
            },
        );
        Ok(IssuedVerifierChallengeV1 {
            challenge,
            issued_at_micros: now_micros,
            expires_at_micros,
        })
    }

    fn consume(
        &mut self,
        challenge: contract::Nonce,
        now_micros: i64,
    ) -> Result<ChallengeConsumptionV1, ProducerError> {
        // Removal happens before expiry/subject verification. Once a verifier
        // attempts to use the challenge, later failure never resurrects it.
        let record = self
            .active
            .remove(&challenge)
            .ok_or(ProducerError::ChallengeUnknownOrConsumed)?;
        if now_micros < record.issued_at_micros {
            return Err(ProducerError::ChallengeBeforeIssueTime);
        }
        if now_micros >= record.expires_at_micros {
            return Err(ProducerError::ChallengeExpired);
        }
        Ok(ChallengeConsumptionV1 {
            challenge,
            issued_at_micros: record.issued_at_micros,
            expires_at_micros: record.expires_at_micros,
            consumed_at_micros: now_micros,
        })
    }
}

/// Verifier-owned qualification evidence admitted by the future concrete
/// anchor/time adapters. There is intentionally no public production
/// constructor in 009C.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifierOwnedMintEvidenceV1 {
    request_commitment: contract::Digest,
    health_state: contract::HealthQualificationStateV1,
    generic_anchor: contract::GenericAcceptedAnchorFieldsV1,
    state_derivation_evidence_commitment: contract::Digest,
    security_time: contract::HealthSecurityTimeFieldsV1,
    underlying_evidence_commitment: contract::Digest,
    observed_at_micros: i64,
    expires_at_micros: i64,
}

impl VerifierOwnedMintEvidenceV1 {
    fn validate_for_request(
        &self,
        request: &contract::VerifierMintRequestV1,
        now_micros: i64,
    ) -> Result<(), ProducerError> {
        if self.request_commitment != request_commitment_v1(request)? {
            return Err(ProducerError::EvidenceRequestMismatch);
        }
        if self.health_state != request.health_state() {
            return Err(ProducerError::EvidenceHealthStateMismatch);
        }
        if self.observed_at_micros < 0
            || self.expires_at_micros <= self.observed_at_micros
            || now_micros < self.observed_at_micros
            || now_micros >= self.expires_at_micros
        {
            return Err(ProducerError::EvidenceNotCurrent);
        }
        Ok(())
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn for_test(
        request: &contract::VerifierMintRequestV1,
        generic_anchor: contract::GenericAcceptedAnchorFieldsV1,
        state_derivation_evidence_commitment: contract::Digest,
        security_time: contract::HealthSecurityTimeFieldsV1,
        underlying_evidence_commitment: contract::Digest,
        observed_at_micros: i64,
        expires_at_micros: i64,
    ) -> Self {
        Self {
            request_commitment: request_commitment_v1(request).unwrap(),
            health_state: request.health_state(),
            generic_anchor,
            state_derivation_evidence_commitment,
            security_time,
            underlying_evidence_commitment,
            observed_at_micros,
            expires_at_micros,
        }
    }
}

/// Software signing identity owned by the isolated verifier process.
///
/// 009C does not claim HSM/TPM custody. A production custody adapter may replace
/// this implementation without changing the portable 009A/producer transcripts.
pub struct SoftwareVerifierIdentityV1 {
    logical_verifier_id: String,
    provider_namespace_commitment: contract::Digest,
    verifier_instance_commitment: contract::Digest,
    policy_generation: u64,
    ed25519_signing_key: Ed25519SigningKey,
    ml_dsa_signing_key: MlDsaSigningKey<MlDsa65>,
}

impl SoftwareVerifierIdentityV1 {
    pub fn from_seeds(
        logical_verifier_id: String,
        provider_namespace_commitment: contract::Digest,
        verifier_instance_commitment: contract::Digest,
        policy_generation: u64,
        ed25519_seed: [u8; 32],
        ml_dsa_seed: [u8; 32],
    ) -> Result<Self, ProducerError> {
        if logical_verifier_id.is_empty() {
            return Err(ProducerError::EmptyLogicalVerifierId);
        }
        if policy_generation == 0 {
            return Err(ProducerError::InvalidPolicyGeneration);
        }
        let ed25519_signing_key = Ed25519SigningKey::from_bytes(&ed25519_seed);
        let ml_seed: B32 = ml_dsa_seed.into();
        let ml_dsa_signing_key = MlDsaSigningKey::<MlDsa65>::from_seed(&ml_seed);
        Ok(Self {
            logical_verifier_id,
            provider_namespace_commitment,
            verifier_instance_commitment,
            policy_generation,
            ed25519_signing_key,
            ml_dsa_signing_key,
        })
    }

    pub fn logical_verifier_id(&self) -> &str {
        &self.logical_verifier_id
    }

    pub fn policy_generation(&self) -> u64 {
        self.policy_generation
    }

    pub fn ed25519_public_key(&self) -> [u8; ED25519_PUBLIC_KEY_LEN] {
        self.ed25519_signing_key.verifying_key().to_bytes()
    }

    pub fn ml_dsa_65_public_key(&self) -> [u8; ML_DSA_65_PUBLIC_KEY_LEN] {
        let encoded = self.ml_dsa_signing_key.verifying_key().encode();
        let mut out = [0u8; ML_DSA_65_PUBLIC_KEY_LEN];
        out.copy_from_slice(encoded.as_slice());
        out
    }

    pub fn key_lineage_commitment(&self) -> Result<contract::Digest, ProducerError> {
        Ok(crypto::key_lineage_commitment(
            &self.ed25519_public_key(),
            &self.ml_dsa_65_public_key(),
        )?)
    }

    fn statement_identity(&self) -> Result<contract::VerifierIdentityV1, ProducerError> {
        Ok(contract::VerifierIdentityV1::new(
            self.provider_namespace_commitment,
            self.verifier_instance_commitment,
            self.key_lineage_commitment()?,
        ))
    }

    fn current_enrollment(&self) -> Result<crypto::CurrentVerifierEnrollmentV1, ProducerError> {
        Ok(crypto::CurrentVerifierEnrollmentV1::new(
            self.logical_verifier_id.clone(),
            self.provider_namespace_commitment,
            self.verifier_instance_commitment,
            self.ed25519_public_key(),
            self.ml_dsa_65_public_key(),
        )?)
    }

    fn sign_hybrid(
        &self,
        message: &[u8],
    ) -> Result<contract::HybridVerifierSignatureV1, ProducerError> {
        let ed_signature = self.ed25519_signing_key.sign(message).to_bytes();
        let ml_signature: MlDsaSignature<MlDsa65> = self.ml_dsa_signing_key.sign(message);
        let encoded = ml_signature.encode();
        let mut ml_signature_bytes = [0u8; contract::ML_DSA_65_SIGNATURE_LEN];
        ml_signature_bytes.copy_from_slice(encoded.as_slice());
        Ok(contract::HybridVerifierSignatureV1::new(
            ed_signature,
            ml_signature_bytes,
        )?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderAuthenticatedMintReceiptV1 {
    mint_receipt: contract::VerifierMintReceiptV1,
    ed25519_pubkey: [u8; ED25519_PUBLIC_KEY_LEN],
    ml_dsa_65_pubkey: [u8; ML_DSA_65_PUBLIC_KEY_LEN],
    logical_verifier_id_commitment: contract::Digest,
    challenge_consumption_commitment: contract::Digest,
    policy_snapshot_commitment: contract::Digest,
    receipt_commitment: contract::Digest,
    provider_signatures: contract::HybridVerifierSignatureV1,
    producer_issued_at_micros: i64,
    producer_expires_at_micros: i64,
}

impl ProviderAuthenticatedMintReceiptV1 {
    pub fn mint_receipt(&self) -> &contract::VerifierMintReceiptV1 {
        &self.mint_receipt
    }

    pub fn ed25519_pubkey(&self) -> &[u8; ED25519_PUBLIC_KEY_LEN] {
        &self.ed25519_pubkey
    }

    pub fn ml_dsa_65_pubkey(&self) -> &[u8; ML_DSA_65_PUBLIC_KEY_LEN] {
        &self.ml_dsa_65_pubkey
    }

    pub fn logical_verifier_id_commitment(&self) -> contract::Digest {
        self.logical_verifier_id_commitment
    }

    pub fn challenge_consumption_commitment(&self) -> contract::Digest {
        self.challenge_consumption_commitment
    }

    pub fn policy_snapshot_commitment(&self) -> contract::Digest {
        self.policy_snapshot_commitment
    }

    pub fn receipt_commitment(&self) -> contract::Digest {
        self.receipt_commitment
    }

    pub fn provider_signatures(&self) -> &contract::HybridVerifierSignatureV1 {
        &self.provider_signatures
    }

    pub fn producer_issued_at_micros(&self) -> i64 {
        self.producer_issued_at_micros
    }

    pub fn producer_expires_at_micros(&self) -> i64 {
        self.producer_expires_at_micros
    }

    pub fn producer_attestation_transcript(&self) -> Vec<u8> {
        producer_attestation_transcript_v1(
            self.receipt_commitment,
            self.logical_verifier_id_commitment,
            self.challenge_consumption_commitment,
            self.policy_snapshot_commitment,
            self.producer_issued_at_micros,
            self.producer_expires_at_micros,
        )
    }
}

pub struct QualificationVerifierMintProducerV1 {
    challenges: InMemoryChallengeStoreV1,
    identity: SoftwareVerifierIdentityV1,
    max_challenge_lifetime_micros: i64,
    max_receipt_lifetime_micros: i64,
    last_observed_micros: Option<i64>,
}

impl QualificationVerifierMintProducerV1 {
    pub fn new(
        identity: SoftwareVerifierIdentityV1,
        max_challenge_lifetime_micros: i64,
        max_receipt_lifetime_micros: i64,
    ) -> Result<Self, ProducerError> {
        if max_challenge_lifetime_micros <= 0 || max_receipt_lifetime_micros <= 0 {
            return Err(ProducerError::InvalidLifetimePolicy);
        }
        Ok(Self {
            challenges: InMemoryChallengeStoreV1::default(),
            identity,
            max_challenge_lifetime_micros,
            max_receipt_lifetime_micros,
            last_observed_micros: None,
        })
    }

    pub fn issue_challenge(
        &mut self,
        now_micros: i64,
        requested_lifetime_micros: i64,
    ) -> Result<IssuedVerifierChallengeV1, ProducerError> {
        self.observe_time(now_micros)?;
        if requested_lifetime_micros <= 0
            || requested_lifetime_micros > self.max_challenge_lifetime_micros
        {
            return Err(ProducerError::InvalidChallengeWindow);
        }
        self.challenges.issue(now_micros, requested_lifetime_micros)
    }

    pub fn mint(
        &mut self,
        request: contract::VerifierMintRequestV1,
        evidence: VerifierOwnedMintEvidenceV1,
        now_micros: i64,
    ) -> Result<ProviderAuthenticatedMintReceiptV1, ProducerError> {
        self.observe_time(now_micros)?;

        // Security-critical ordering: consume first. Every later error burns the
        // challenge and cannot be retried against the same nonce.
        let consumption = self
            .challenges
            .consume(request.verifier_challenge(), now_micros)?;

        if now_micros < request.requested_at_micros()
            || now_micros >= request.expires_at_micros()
        {
            return Err(ProducerError::RequestNotCurrent);
        }

        evidence.validate_for_request(&request, now_micros)?;

        let request_commitment = request_commitment_v1(&request)?;
        let max_receipt_expiry = now_micros
            .checked_add(self.max_receipt_lifetime_micros)
            .ok_or(ProducerError::TimeOverflow)?;
        let statement_expiry = request
            .expires_at_micros()
            .min(evidence.expires_at_micros)
            .min(max_receipt_expiry);
        if statement_expiry <= now_micros {
            return Err(ProducerError::NoPositiveReceiptLifetime);
        }

        let statement = contract::VerifierMintStatementV1::new(
            &request,
            request_commitment,
            self.identity.statement_identity()?,
            evidence.generic_anchor,
            evidence.state_derivation_evidence_commitment,
            evidence.security_time,
            evidence.underlying_evidence_commitment,
            contract::SignatureSuiteV1::Ed25519MlDsa65,
            now_micros,
            statement_expiry,
        )?;

        let signatures = self.identity.sign_hybrid(&statement.signing_transcript())?;
        let mint_receipt = contract::VerifierMintReceiptV1::new(statement, signatures);

        // Producer self-check through the separately staged 009B verifier.
        let signed = crypto::SignedVerifierMintV1::new(
            mint_receipt.clone(),
            self.identity.ed25519_public_key(),
            self.identity.ml_dsa_65_public_key(),
        );
        let verified_signatures = crypto::verify_mint_signatures_v1(signed)?;
        let enrollment = self.identity.current_enrollment()?;
        let verified_crypto =
            crypto::bind_verified_mint_to_enrollment_v1(verified_signatures, &enrollment)?;
        if verified_crypto.key_lineage_commitment() != self.identity.key_lineage_commitment()? {
            return Err(ProducerError::InternalEnrollmentMismatch);
        }

        let challenge_consumption_commitment = challenge_consumption_commitment_v1(consumption)?;
        let policy_snapshot_commitment = policy_snapshot_commitment_v1(&self.identity)?;
        let logical_verifier_id_commitment =
            logical_verifier_id_commitment_v1(self.identity.logical_verifier_id())?;
        let receipt_commitment = mint_receipt_commitment_v1(&mint_receipt)?;

        let provider_transcript = producer_attestation_transcript_v1(
            receipt_commitment,
            logical_verifier_id_commitment,
            challenge_consumption_commitment,
            policy_snapshot_commitment,
            now_micros,
            statement_expiry,
        );
        let provider_signatures = self.identity.sign_hybrid(&provider_transcript)?;

        Ok(ProviderAuthenticatedMintReceiptV1 {
            mint_receipt,
            ed25519_pubkey: self.identity.ed25519_public_key(),
            ml_dsa_65_pubkey: self.identity.ml_dsa_65_public_key(),
            logical_verifier_id_commitment,
            challenge_consumption_commitment,
            policy_snapshot_commitment,
            receipt_commitment,
            provider_signatures,
            producer_issued_at_micros: now_micros,
            producer_expires_at_micros: statement_expiry,
        })
    }

    #[cfg(test)]
    fn issue_exact_challenge_for_test(
        &mut self,
        challenge: contract::Nonce,
        now_micros: i64,
        lifetime_micros: i64,
    ) -> Result<IssuedVerifierChallengeV1, ProducerError> {
        self.observe_time(now_micros)?;
        self.challenges
            .issue_exact(challenge, now_micros, lifetime_micros)
    }

    fn observe_time(&mut self, now_micros: i64) -> Result<(), ProducerError> {
        if now_micros < 0 {
            return Err(ProducerError::InvalidObservedTime);
        }
        if let Some(previous) = self.last_observed_micros {
            if now_micros < previous {
                return Err(ProducerError::ClockRollback);
            }
        }
        self.last_observed_micros = Some(now_micros);
        Ok(())
    }
}

pub fn request_commitment_v1(
    request: &contract::VerifierMintRequestV1,
) -> Result<contract::Digest, ProducerError> {
    digest_parts(REQUEST_COMMITMENT_DOMAIN_V1, &[&request.canonical_bytes()])
}

fn logical_verifier_id_commitment_v1(
    logical_verifier_id: &str,
) -> Result<contract::Digest, ProducerError> {
    let len = u32::try_from(logical_verifier_id.len()).map_err(|_| ProducerError::LengthOverflow)?;
    digest_parts(
        LOGICAL_VERIFIER_ID_DOMAIN_V1,
        &[&len.to_be_bytes(), logical_verifier_id.as_bytes()],
    )
}

fn challenge_consumption_commitment_v1(
    consumption: ChallengeConsumptionV1,
) -> Result<contract::Digest, ProducerError> {
    digest_parts(
        CHALLENGE_CONSUMPTION_DOMAIN_V1,
        &[
            consumption.challenge.as_bytes(),
            &consumption.issued_at_micros.to_be_bytes(),
            &consumption.expires_at_micros.to_be_bytes(),
            &consumption.consumed_at_micros.to_be_bytes(),
        ],
    )
}

fn policy_snapshot_commitment_v1(
    identity: &SoftwareVerifierIdentityV1,
) -> Result<contract::Digest, ProducerError> {
    let logical = logical_verifier_id_commitment_v1(identity.logical_verifier_id())?;
    let lineage = identity.key_lineage_commitment()?;
    digest_parts(
        POLICY_SNAPSHOT_DOMAIN_V1,
        &[
            logical.as_bytes(),
            identity.provider_namespace_commitment.as_bytes(),
            identity.verifier_instance_commitment.as_bytes(),
            lineage.as_bytes(),
            &identity.policy_generation.to_be_bytes(),
        ],
    )
}

fn mint_receipt_commitment_v1(
    receipt: &contract::VerifierMintReceiptV1,
) -> Result<contract::Digest, ProducerError> {
    let statement = receipt.statement().canonical_bytes();
    let statement_len = u32::try_from(statement.len()).map_err(|_| ProducerError::LengthOverflow)?;
    digest_parts(
        RECEIPT_COMMITMENT_DOMAIN_V1,
        &[
            &statement_len.to_be_bytes(),
            &statement,
            receipt.signatures().ed25519_signature(),
            receipt.signatures().ml_dsa_65_signature(),
        ],
    )
}

fn producer_attestation_transcript_v1(
    receipt_commitment: contract::Digest,
    logical_verifier_id_commitment: contract::Digest,
    challenge_consumption_commitment: contract::Digest,
    policy_snapshot_commitment: contract::Digest,
    issued_at_micros: i64,
    expires_at_micros: i64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);
    out.extend_from_slice(PRODUCER_ATTESTATION_DOMAIN_V1);
    out.extend_from_slice(&PRODUCER_ATTESTATION_SCHEMA_V1.to_be_bytes());
    out.extend_from_slice(receipt_commitment.as_bytes());
    out.extend_from_slice(logical_verifier_id_commitment.as_bytes());
    out.extend_from_slice(challenge_consumption_commitment.as_bytes());
    out.extend_from_slice(policy_snapshot_commitment.as_bytes());
    out.extend_from_slice(&issued_at_micros.to_be_bytes());
    out.extend_from_slice(&expires_at_micros.to_be_bytes());
    out
}

fn digest_parts(
    domain: &[u8],
    parts: &[&[u8]],
) -> Result<contract::Digest, ProducerError> {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(part);
    }
    let bytes: [u8; 32] = hasher.finalize().into();
    contract::Digest::new(bytes).map_err(|_| ProducerError::ImpossibleZeroDigest)
}

#[derive(Debug, thiserror::Error)]
pub enum ProducerError {
    #[error("invalid challenge window")]
    InvalidChallengeWindow,
    #[error("challenge generation exhausted")]
    ChallengeGenerationExhausted,
    #[error("challenge collision")]
    ChallengeCollision,
    #[error("challenge unknown or already consumed")]
    ChallengeUnknownOrConsumed,
    #[error("challenge used before its issue time")]
    ChallengeBeforeIssueTime,
    #[error("challenge expired")]
    ChallengeExpired,
    #[error("request is not current")]
    RequestNotCurrent,
    #[error("verifier-owned evidence was minted for a different request")]
    EvidenceRequestMismatch,
    #[error("verifier-owned evidence names a different Health state")]
    EvidenceHealthStateMismatch,
    #[error("verifier-owned evidence is not current")]
    EvidenceNotCurrent,
    #[error("no positive receipt lifetime remains")]
    NoPositiveReceiptLifetime,
    #[error("empty logical verifier id")]
    EmptyLogicalVerifierId,
    #[error("invalid verifier policy generation")]
    InvalidPolicyGeneration,
    #[error("invalid producer lifetime policy")]
    InvalidLifetimePolicy,
    #[error("invalid observed time")]
    InvalidObservedTime,
    #[error("security clock rollback")]
    ClockRollback,
    #[error("integer length overflow")]
    LengthOverflow,
    #[error("time overflow")]
    TimeOverflow,
    #[error("unexpected all-zero digest")]
    ImpossibleZeroDigest,
    #[error("producer enrollment self-check mismatch")]
    InternalEnrollmentMismatch,
    #[error("009A contract rejected producer output: {0:?}")]
    Contract(#[from] contract::ContractError),
    #[error("009B hybrid verifier rejected producer output: {0}")]
    HybridVerifier(#[from] crypto::MintVerifierError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature as Ed25519Signature, Verifier as _, VerifyingKey};
    use ml_dsa::{
        EncodedSignature as MlDsaEncodedSignature,
        EncodedVerifyingKey as MlDsaEncodedVerifyingKey,
        Signature as MlDsaSignatureT, VerifyingKey as MlDsaVerifyingKey,
        signature::Verifier as _,
    };

    fn b(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn d(value: u8) -> contract::Digest {
        contract::Digest::new(b(value)).unwrap()
    }

    fn n(value: u8) -> contract::Nonce {
        contract::Nonce::new(b(value)).unwrap()
    }

    fn identity(seed_offset: u8, generation: u64) -> SoftwareVerifierIdentityV1 {
        SoftwareVerifierIdentityV1::from_seeds(
            "health-verifier-1".to_string(),
            d(40),
            d(41),
            generation,
            [seed_offset; 32],
            [seed_offset.wrapping_add(1); 32],
        )
        .unwrap()
    }

    fn producer(seed_offset: u8, generation: u64) -> QualificationVerifierMintProducerV1 {
        QualificationVerifierMintProducerV1::new(identity(seed_offset, generation), 1_000, 500)
            .unwrap()
    }

    fn health_state() -> contract::HealthQualificationStateV1 {
        contract::HealthQualificationStateV1::new(d(1), d(2), 7, 9, d(3), d(4)).unwrap()
    }

    fn request(challenge: contract::Nonce, product: contract::Digest) -> contract::VerifierMintRequestV1 {
        contract::VerifierMintRequestV1::new(
            contract::TokenProfileV1::GenericAnchorAndSecurityTime,
            product,
            challenge,
            n(7),
            health_state(),
            1_000,
            2_000,
        )
        .unwrap()
    }

    fn generic() -> contract::GenericAcceptedAnchorFieldsV1 {
        contract::GenericAcceptedAnchorFieldsV1::new(
            contract::Digest::new(contract::HEALTH_GENERIC_DOMAIN_ID_V1).unwrap(),
            d(1),
            7,
            d(8),
            3,
            Some(d(9)),
            n(10),
            d(11),
            d(12),
            2,
            d(13),
            d(14),
            1_100,
            1_950,
            0,
            None,
        )
        .unwrap()
    }

    fn security_time() -> contract::HealthSecurityTimeFieldsV1 {
        contract::HealthSecurityTimeFieldsV1::new(3, d(11), d(8), d(15), 1_120, 1_130, 1_900)
            .unwrap()
    }

    fn evidence(request: &contract::VerifierMintRequestV1) -> VerifierOwnedMintEvidenceV1 {
        VerifierOwnedMintEvidenceV1::for_test(
            request,
            generic(),
            d(20),
            security_time(),
            d(21),
            1_130,
            1_900,
        )
    }

    fn verify_provider_envelope(receipt: &ProviderAuthenticatedMintReceiptV1) {
        let transcript = receipt.producer_attestation_transcript();
        let ed_vk = VerifyingKey::from_bytes(receipt.ed25519_pubkey()).unwrap();
        let ed_sig = Ed25519Signature::from_bytes(receipt.provider_signatures().ed25519_signature());
        ed_vk.verify(&transcript, &ed_sig).unwrap();

        let encoded_key = MlDsaEncodedVerifyingKey::<MlDsa65>::try_from(
            receipt.ml_dsa_65_pubkey().as_slice(),
        )
        .unwrap();
        let ml_vk = MlDsaVerifyingKey::<MlDsa65>::decode(&encoded_key);
        let encoded_sig = MlDsaEncodedSignature::<MlDsa65>::try_from(
            receipt.provider_signatures().ml_dsa_65_signature().as_slice(),
        )
        .unwrap();
        let ml_sig = MlDsaSignatureT::<MlDsa65>::decode(&encoded_sig).unwrap();
        ml_vk.verify(&transcript, &ml_sig).unwrap();
    }

    #[test]
    fn exact_request_mints_once_and_provider_envelope_has_real_hybrid_signatures() {
        let mut producer = producer(11, 1);
        let issued = producer
            .issue_exact_challenge_for_test(n(6), 900, 500)
            .unwrap();
        let request = request(issued.challenge(), d(5));
        let evidence = evidence(&request);

        let result = producer.mint(request, evidence, 1_200).unwrap();
        verify_provider_envelope(&result);

        let replay = producer.mint(request, evidence, 1_201).unwrap_err();
        assert!(matches!(replay, ProducerError::ChallengeUnknownOrConsumed));
    }

    #[test]
    fn evidence_failure_after_consume_burns_challenge() {
        let mut producer = producer(11, 1);
        producer
            .issue_exact_challenge_for_test(n(6), 900, 500)
            .unwrap();
        let request = request(n(6), d(5));
        let mut wrong = evidence(&request);
        wrong.request_commitment = d(99);

        let first = producer.mint(request, wrong, 1_200).unwrap_err();
        assert!(matches!(first, ProducerError::EvidenceRequestMismatch));

        let second = producer.mint(request, evidence(&request), 1_201).unwrap_err();
        assert!(matches!(second, ProducerError::ChallengeUnknownOrConsumed));
    }

    #[test]
    fn product_request_substitution_is_denied_after_challenge_consumption() {
        let mut producer = producer(11, 1);
        producer
            .issue_exact_challenge_for_test(n(6), 900, 500)
            .unwrap();
        let original = request(n(6), d(5));
        let evidence = evidence(&original);
        let substituted = request(n(6), d(55));

        let error = producer.mint(substituted, evidence, 1_200).unwrap_err();
        assert!(matches!(error, ProducerError::EvidenceRequestMismatch));
        assert!(matches!(
            producer.mint(original, evidence(&original), 1_201),
            Err(ProducerError::ChallengeUnknownOrConsumed)
        ));
    }

    #[test]
    fn health_state_substitution_is_denied() {
        let mut producer = producer(11, 1);
        producer
            .issue_exact_challenge_for_test(n(6), 900, 500)
            .unwrap();
        let request = request(n(6), d(5));
        let mut evidence = evidence(&request);
        evidence.health_state = contract::HealthQualificationStateV1::new(
            d(1), d(77), 7, 9, d(3), d(4),
        )
        .unwrap();
        let error = producer.mint(request, evidence, 1_200).unwrap_err();
        assert!(matches!(error, ProducerError::EvidenceHealthStateMismatch));
    }

    #[test]
    fn expired_challenge_is_removed_and_cannot_be_retried() {
        let mut producer = producer(11, 1);
        producer
            .issue_exact_challenge_for_test(n(6), 900, 100)
            .unwrap();
        let request = request(n(6), d(5));
        let error = producer.mint(request, evidence(&request), 1_000).unwrap_err();
        assert!(matches!(error, ProducerError::ChallengeExpired));
        let retry = producer.mint(request, evidence(&request), 1_001).unwrap_err();
        assert!(matches!(retry, ProducerError::ChallengeUnknownOrConsumed));
    }

    #[test]
    fn evidence_cannot_be_used_before_its_observation_floor() {
        let mut producer = producer(11, 1);
        producer
            .issue_exact_challenge_for_test(n(6), 900, 500)
            .unwrap();
        let request = request(n(6), d(5));
        let error = producer.mint(request, evidence(&request), 1_120).unwrap_err();
        assert!(matches!(error, ProducerError::EvidenceNotCurrent));
    }

    #[test]
    fn producer_clock_rollback_fails_closed() {
        let mut producer = producer(11, 1);
        producer
            .issue_exact_challenge_for_test(n(6), 900, 500)
            .unwrap();
        let request = request(n(6), d(5));
        let _ = producer.mint(request, evidence(&request), 1_200).unwrap();
        let error = producer.issue_challenge(1_199, 100).unwrap_err();
        assert!(matches!(error, ProducerError::ClockRollback));
    }

    #[test]
    fn policy_snapshot_changes_when_hybrid_key_lineage_rotates() {
        let first = identity(11, 1);
        let second = identity(31, 2);
        assert_ne!(first.key_lineage_commitment().unwrap(), second.key_lineage_commitment().unwrap());
        assert_ne!(
            policy_snapshot_commitment_v1(&first).unwrap(),
            policy_snapshot_commitment_v1(&second).unwrap()
        );
    }

    #[test]
    fn provider_attestation_binds_receipt_freshness_and_policy_commitments() {
        let mut producer = producer(11, 1);
        producer
            .issue_exact_challenge_for_test(n(6), 900, 500)
            .unwrap();
        let request = request(n(6), d(5));
        let result = producer.mint(request, evidence(&request), 1_200).unwrap();
        let baseline = result.producer_attestation_transcript();

        assert_ne!(
            baseline,
            producer_attestation_transcript_v1(
                d(70),
                result.logical_verifier_id_commitment(),
                result.challenge_consumption_commitment(),
                result.policy_snapshot_commitment(),
                result.producer_issued_at_micros(),
                result.producer_expires_at_micros(),
            )
        );
        assert_ne!(
            baseline,
            producer_attestation_transcript_v1(
                result.receipt_commitment(),
                result.logical_verifier_id_commitment(),
                d(71),
                result.policy_snapshot_commitment(),
                result.producer_issued_at_micros(),
                result.producer_expires_at_micros(),
            )
        );
        assert_ne!(
            baseline,
            producer_attestation_transcript_v1(
                result.receipt_commitment(),
                result.logical_verifier_id_commitment(),
                result.challenge_consumption_commitment(),
                d(72),
                result.producer_issued_at_micros(),
                result.producer_expires_at_micros(),
            )
        );
    }

    #[test]
    fn no_public_path_claims_product_authority() {
        let name = core::any::type_name::<ProviderAuthenticatedMintReceiptV1>();
        assert!(name.contains("ProviderAuthenticatedMintReceiptV1"));
        // The positive producer artifact is intentionally a receipt, not a
        // ProductAuthorityToken / AnchoredQualificationHead.
        assert!(!name.contains("ProductAuthorityToken"));
        assert!(!name.contains("AnchoredQualificationHead"));
    }
}
