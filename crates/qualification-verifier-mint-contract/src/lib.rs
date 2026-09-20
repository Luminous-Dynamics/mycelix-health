#![forbid(unsafe_code)]
//! Portable, crypto-verifier-free request and signed-statement contract for an
//! isolated Mycelix Health qualification verifier.
//!
//! This crate freezes bytes only. Constructing a request, statement, signature
//! envelope, or receipt does **not** prove cryptographic verification, verifier
//! identity, challenge freshness/single-use, trusted time, or product token
//! admission.

use core::fmt;

pub const REQUEST_SCHEMA_V1: u16 = 1;
pub const STATEMENT_SCHEMA_V1: u16 = 1;
pub const REQUEST_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-MINT-REQUEST-V1\0";
pub const STATEMENT_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-MINT-STATEMENT-V1\0";
pub const ML_DSA_65_SIGNATURE_LEN: usize = 3309;

/// Same generic transparency domain frozen by Health QUAL-EVID-007/#222.
pub const HEALTH_GENERIC_DOMAIN_ID_V1: [u8; 32] = [
    0xcd, 0xb6, 0x0b, 0xe3, 0x23, 0x75, 0x35, 0x84, 0xce, 0x36, 0x81, 0xc2, 0xbb, 0x8c,
    0xd5, 0x49, 0x9e, 0xc5, 0x69, 0x98, 0x7a, 0x83, 0x83, 0xbd, 0xe5, 0x62, 0x47, 0x34,
    0xee, 0xca, 0x3c, 0xdf,
];

macro_rules! fixed_32_type {
    ($name:ident, $error:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $error {
            AllZero,
        }

        impl $name {
            pub fn new(bytes: [u8; 32]) -> Result<Self, $error> {
                if bytes == [0; 32] {
                    return Err($error::AllZero);
                }
                Ok(Self(bytes))
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    };
}

fixed_32_type!(Digest, DigestError);
fixed_32_type!(Nonce, NonceError);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenProfileV1 {
    GenericAnchorAndSecurityTime = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SignatureSuiteV1 {
    Ed25519MlDsa65 = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractError {
    InvalidCheckpointEpoch,
    InvalidHeadSequence,
    InvalidRequestWindow,
    InvalidAnchorSequence,
    InvalidPredecessorShape,
    InvalidWitnessSetEpoch,
    InvalidGenericWindow,
    InvalidRecoveryShape,
    InvalidSecurityTimeWindow,
    InvalidSecurityTimeFloor,
    GenericDomainMismatch,
    GenericSubjectMismatch,
    GenericStateEpochMismatch,
    SecurityTimeAnchorSequenceMismatch,
    SecurityTimeAnchorMismatch,
    SecurityTimeStateMismatch,
    StatementIssuedBeforeRequest,
    StatementIssuedAfterRequestExpiry,
    InvalidStatementWindow,
    StatementOutlivesRequest,
    StatementOutlivesGenericAnchor,
    StatementOutlivesSecurityTime,
    ZeroEd25519Signature,
    ZeroMlDsa65Signature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealthQualificationStateV1 {
    lineage_binding_digest: Digest,
    checkpoint_digest: Digest,
    checkpoint_epoch: u64,
    head_sequence: u64,
    lineage_state_commitment: Digest,
    verified_head_digest: Digest,
}

impl HealthQualificationStateV1 {
    pub fn new(
        lineage_binding_digest: Digest,
        checkpoint_digest: Digest,
        checkpoint_epoch: u64,
        head_sequence: u64,
        lineage_state_commitment: Digest,
        verified_head_digest: Digest,
    ) -> Result<Self, ContractError> {
        if checkpoint_epoch == 0 {
            return Err(ContractError::InvalidCheckpointEpoch);
        }
        if head_sequence == 0 {
            return Err(ContractError::InvalidHeadSequence);
        }
        Ok(Self {
            lineage_binding_digest,
            checkpoint_digest,
            checkpoint_epoch,
            head_sequence,
            lineage_state_commitment,
            verified_head_digest,
        })
    }

    pub fn lineage_binding_digest(&self) -> Digest {
        self.lineage_binding_digest
    }

    pub fn checkpoint_epoch(&self) -> u64 {
        self.checkpoint_epoch
    }

    pub fn head_sequence(&self) -> u64 {
        self.head_sequence
    }

    fn push_canonical_fields(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.lineage_binding_digest.as_bytes());
        out.extend_from_slice(self.checkpoint_digest.as_bytes());
        out.extend_from_slice(&self.checkpoint_epoch.to_be_bytes());
        out.extend_from_slice(&self.head_sequence.to_be_bytes());
        out.extend_from_slice(self.lineage_state_commitment.as_bytes());
        out.extend_from_slice(self.verified_head_digest.as_bytes());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifierMintRequestV1 {
    token_profile: TokenProfileV1,
    product_request_digest: Digest,
    verifier_challenge: Nonce,
    request_nonce: Nonce,
    health_state: HealthQualificationStateV1,
    requested_at_micros: i64,
    expires_at_micros: i64,
}

impl VerifierMintRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        token_profile: TokenProfileV1,
        product_request_digest: Digest,
        verifier_challenge: Nonce,
        request_nonce: Nonce,
        health_state: HealthQualificationStateV1,
        requested_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, ContractError> {
        if requested_at_micros < 0 || expires_at_micros <= requested_at_micros {
            return Err(ContractError::InvalidRequestWindow);
        }
        Ok(Self {
            token_profile,
            product_request_digest,
            verifier_challenge,
            request_nonce,
            health_state,
            requested_at_micros,
            expires_at_micros,
        })
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(320);
        out.extend_from_slice(REQUEST_DOMAIN_V1);
        out.extend_from_slice(&REQUEST_SCHEMA_V1.to_be_bytes());
        out.push(self.token_profile as u8);
        out.extend_from_slice(self.product_request_digest.as_bytes());
        out.extend_from_slice(self.verifier_challenge.as_bytes());
        out.extend_from_slice(self.request_nonce.as_bytes());
        self.health_state.push_canonical_fields(&mut out);
        out.extend_from_slice(&self.requested_at_micros.to_be_bytes());
        out.extend_from_slice(&self.expires_at_micros.to_be_bytes());
        out
    }

    pub fn token_profile(&self) -> TokenProfileV1 {
        self.token_profile
    }

    pub fn product_request_digest(&self) -> Digest {
        self.product_request_digest
    }

    pub fn verifier_challenge(&self) -> Nonce {
        self.verifier_challenge
    }

    pub fn request_nonce(&self) -> Nonce {
        self.request_nonce
    }

    pub fn health_state(&self) -> HealthQualificationStateV1 {
        self.health_state
    }

    pub fn requested_at_micros(&self) -> i64 {
        self.requested_at_micros
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifierIdentityV1 {
    provider_namespace_commitment: Digest,
    verifier_instance_commitment: Digest,
    key_lineage_commitment: Digest,
}

impl VerifierIdentityV1 {
    pub fn new(
        provider_namespace_commitment: Digest,
        verifier_instance_commitment: Digest,
        key_lineage_commitment: Digest,
    ) -> Self {
        Self {
            provider_namespace_commitment,
            verifier_instance_commitment,
            key_lineage_commitment,
        }
    }

    fn push_canonical_fields(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.provider_namespace_commitment.as_bytes());
        out.extend_from_slice(self.verifier_instance_commitment.as_bytes());
        out.extend_from_slice(self.key_lineage_commitment.as_bytes());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericAcceptedAnchorFieldsV1 {
    domain_id: Digest,
    subject_id: Digest,
    state_epoch: u64,
    state_commitment: Digest,
    anchor_sequence: u64,
    previous_anchor_digest: Option<Digest>,
    anchor_nonce: Nonce,
    anchor_digest: Digest,
    witness_set_digest: Digest,
    witness_set_epoch: u64,
    witness_quorum_digest: Digest,
    trust_snapshot_digest: Digest,
    observed_at_micros: i64,
    expires_at_micros: i64,
    recovery_authority_epoch: u64,
    resolved_recovery_receipt_digest: Option<Digest>,
}

impl GenericAcceptedAnchorFieldsV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        domain_id: Digest,
        subject_id: Digest,
        state_epoch: u64,
        state_commitment: Digest,
        anchor_sequence: u64,
        previous_anchor_digest: Option<Digest>,
        anchor_nonce: Nonce,
        anchor_digest: Digest,
        witness_set_digest: Digest,
        witness_set_epoch: u64,
        witness_quorum_digest: Digest,
        trust_snapshot_digest: Digest,
        observed_at_micros: i64,
        expires_at_micros: i64,
        recovery_authority_epoch: u64,
        resolved_recovery_receipt_digest: Option<Digest>,
    ) -> Result<Self, ContractError> {
        if state_epoch == 0 {
            return Err(ContractError::InvalidCheckpointEpoch);
        }
        if anchor_sequence == 0 {
            return Err(ContractError::InvalidAnchorSequence);
        }
        if (anchor_sequence == 1 && previous_anchor_digest.is_some())
            || (anchor_sequence > 1 && previous_anchor_digest.is_none())
        {
            return Err(ContractError::InvalidPredecessorShape);
        }
        if witness_set_epoch == 0 {
            return Err(ContractError::InvalidWitnessSetEpoch);
        }
        if observed_at_micros < 0 || expires_at_micros <= observed_at_micros {
            return Err(ContractError::InvalidGenericWindow);
        }
        if (recovery_authority_epoch == 0) != resolved_recovery_receipt_digest.is_none() {
            return Err(ContractError::InvalidRecoveryShape);
        }
        Ok(Self {
            domain_id,
            subject_id,
            state_epoch,
            state_commitment,
            anchor_sequence,
            previous_anchor_digest,
            anchor_nonce,
            anchor_digest,
            witness_set_digest,
            witness_set_epoch,
            witness_quorum_digest,
            trust_snapshot_digest,
            observed_at_micros,
            expires_at_micros,
            recovery_authority_epoch,
            resolved_recovery_receipt_digest,
        })
    }

    pub fn anchor_sequence(&self) -> u64 {
        self.anchor_sequence
    }

    pub fn anchor_digest(&self) -> Digest {
        self.anchor_digest
    }

    pub fn state_commitment(&self) -> Digest {
        self.state_commitment
    }

    fn push_canonical_fields(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.domain_id.as_bytes());
        out.extend_from_slice(self.subject_id.as_bytes());
        out.extend_from_slice(&self.state_epoch.to_be_bytes());
        out.extend_from_slice(self.state_commitment.as_bytes());
        out.extend_from_slice(&self.anchor_sequence.to_be_bytes());
        push_optional_digest(out, self.previous_anchor_digest);
        out.extend_from_slice(self.anchor_nonce.as_bytes());
        out.extend_from_slice(self.anchor_digest.as_bytes());
        out.extend_from_slice(self.witness_set_digest.as_bytes());
        out.extend_from_slice(&self.witness_set_epoch.to_be_bytes());
        out.extend_from_slice(self.witness_quorum_digest.as_bytes());
        out.extend_from_slice(self.trust_snapshot_digest.as_bytes());
        out.extend_from_slice(&self.observed_at_micros.to_be_bytes());
        out.extend_from_slice(&self.expires_at_micros.to_be_bytes());
        out.extend_from_slice(&self.recovery_authority_epoch.to_be_bytes());
        push_optional_digest(out, self.resolved_recovery_receipt_digest);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealthSecurityTimeFieldsV1 {
    anchor_sequence: u64,
    anchor_digest: Digest,
    state_commitment: Digest,
    proof_digest: Digest,
    security_time_floor_micros: i64,
    observed_at_micros: i64,
    expires_at_micros: i64,
}

impl HealthSecurityTimeFieldsV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        anchor_sequence: u64,
        anchor_digest: Digest,
        state_commitment: Digest,
        proof_digest: Digest,
        security_time_floor_micros: i64,
        observed_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, ContractError> {
        if anchor_sequence == 0 {
            return Err(ContractError::InvalidAnchorSequence);
        }
        if observed_at_micros < 0 || expires_at_micros <= observed_at_micros {
            return Err(ContractError::InvalidSecurityTimeWindow);
        }
        if security_time_floor_micros < 0 || security_time_floor_micros > observed_at_micros {
            return Err(ContractError::InvalidSecurityTimeFloor);
        }
        Ok(Self {
            anchor_sequence,
            anchor_digest,
            state_commitment,
            proof_digest,
            security_time_floor_micros,
            observed_at_micros,
            expires_at_micros,
        })
    }

    fn push_canonical_fields(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.anchor_sequence.to_be_bytes());
        out.extend_from_slice(self.anchor_digest.as_bytes());
        out.extend_from_slice(self.state_commitment.as_bytes());
        out.extend_from_slice(self.proof_digest.as_bytes());
        out.extend_from_slice(&self.security_time_floor_micros.to_be_bytes());
        out.extend_from_slice(&self.observed_at_micros.to_be_bytes());
        out.extend_from_slice(&self.expires_at_micros.to_be_bytes());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifierMintStatementV1 {
    signature_suite: SignatureSuiteV1,
    token_profile: TokenProfileV1,
    request_commitment: Digest,
    product_request_digest: Digest,
    verifier_challenge: Nonce,
    request_nonce: Nonce,
    request_requested_at_micros: i64,
    request_expires_at_micros: i64,
    verifier_identity: VerifierIdentityV1,
    health_state: HealthQualificationStateV1,
    generic_anchor: GenericAcceptedAnchorFieldsV1,
    state_derivation_evidence_commitment: Digest,
    security_time: HealthSecurityTimeFieldsV1,
    underlying_evidence_commitment: Digest,
    issued_at_micros: i64,
    expires_at_micros: i64,
}

impl VerifierMintStatementV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request: &VerifierMintRequestV1,
        request_commitment: Digest,
        verifier_identity: VerifierIdentityV1,
        generic_anchor: GenericAcceptedAnchorFieldsV1,
        state_derivation_evidence_commitment: Digest,
        security_time: HealthSecurityTimeFieldsV1,
        underlying_evidence_commitment: Digest,
        signature_suite: SignatureSuiteV1,
        issued_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, ContractError> {
        let health_state = request.health_state;
        if generic_anchor.domain_id.0 != HEALTH_GENERIC_DOMAIN_ID_V1 {
            return Err(ContractError::GenericDomainMismatch);
        }
        if generic_anchor.subject_id != health_state.lineage_binding_digest {
            return Err(ContractError::GenericSubjectMismatch);
        }
        if generic_anchor.state_epoch != health_state.checkpoint_epoch {
            return Err(ContractError::GenericStateEpochMismatch);
        }
        if security_time.anchor_sequence != generic_anchor.anchor_sequence {
            return Err(ContractError::SecurityTimeAnchorSequenceMismatch);
        }
        if security_time.anchor_digest != generic_anchor.anchor_digest {
            return Err(ContractError::SecurityTimeAnchorMismatch);
        }
        if security_time.state_commitment != generic_anchor.state_commitment {
            return Err(ContractError::SecurityTimeStateMismatch);
        }
        if issued_at_micros < request.requested_at_micros {
            return Err(ContractError::StatementIssuedBeforeRequest);
        }
        if issued_at_micros >= request.expires_at_micros {
            return Err(ContractError::StatementIssuedAfterRequestExpiry);
        }
        if expires_at_micros <= issued_at_micros {
            return Err(ContractError::InvalidStatementWindow);
        }
        if expires_at_micros > request.expires_at_micros {
            return Err(ContractError::StatementOutlivesRequest);
        }
        if expires_at_micros > generic_anchor.expires_at_micros {
            return Err(ContractError::StatementOutlivesGenericAnchor);
        }
        if expires_at_micros > security_time.expires_at_micros {
            return Err(ContractError::StatementOutlivesSecurityTime);
        }
        Ok(Self {
            signature_suite,
            token_profile: request.token_profile,
            request_commitment,
            product_request_digest: request.product_request_digest,
            verifier_challenge: request.verifier_challenge,
            request_nonce: request.request_nonce,
            request_requested_at_micros: request.requested_at_micros,
            request_expires_at_micros: request.expires_at_micros,
            verifier_identity,
            health_state,
            generic_anchor,
            state_derivation_evidence_commitment,
            security_time,
            underlying_evidence_commitment,
            issued_at_micros,
            expires_at_micros,
        })
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(900);
        out.extend_from_slice(STATEMENT_DOMAIN_V1);
        out.extend_from_slice(&STATEMENT_SCHEMA_V1.to_be_bytes());
        out.push(self.signature_suite as u8);
        out.push(self.token_profile as u8);
        out.extend_from_slice(self.request_commitment.as_bytes());
        out.extend_from_slice(self.product_request_digest.as_bytes());
        out.extend_from_slice(self.verifier_challenge.as_bytes());
        out.extend_from_slice(self.request_nonce.as_bytes());
        out.extend_from_slice(&self.request_requested_at_micros.to_be_bytes());
        out.extend_from_slice(&self.request_expires_at_micros.to_be_bytes());
        self.verifier_identity.push_canonical_fields(&mut out);
        self.health_state.push_canonical_fields(&mut out);
        self.generic_anchor.push_canonical_fields(&mut out);
        out.extend_from_slice(self.state_derivation_evidence_commitment.as_bytes());
        self.security_time.push_canonical_fields(&mut out);
        out.extend_from_slice(self.underlying_evidence_commitment.as_bytes());
        out.extend_from_slice(&self.issued_at_micros.to_be_bytes());
        out.extend_from_slice(&self.expires_at_micros.to_be_bytes());
        out
    }

    pub fn signing_transcript(&self) -> Vec<u8> {
        self.canonical_bytes()
    }

    pub fn request_commitment(&self) -> Digest {
        self.request_commitment
    }

    pub fn generic_anchor(&self) -> GenericAcceptedAnchorFieldsV1 {
        self.generic_anchor
    }

    pub fn security_time(&self) -> HealthSecurityTimeFieldsV1 {
        self.security_time
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct HybridVerifierSignatureV1 {
    ed25519_signature: [u8; 64],
    ml_dsa_65_signature: [u8; ML_DSA_65_SIGNATURE_LEN],
}

impl fmt::Debug for HybridVerifierSignatureV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HybridVerifierSignatureV1")
            .field("ed25519_signature", &"[redacted]")
            .field("ml_dsa_65_signature", &"[redacted]")
            .finish()
    }
}

impl HybridVerifierSignatureV1 {
    pub fn new(
        ed25519_signature: [u8; 64],
        ml_dsa_65_signature: [u8; ML_DSA_65_SIGNATURE_LEN],
    ) -> Result<Self, ContractError> {
        if ed25519_signature == [0; 64] {
            return Err(ContractError::ZeroEd25519Signature);
        }
        if ml_dsa_65_signature == [0; ML_DSA_65_SIGNATURE_LEN] {
            return Err(ContractError::ZeroMlDsa65Signature);
        }
        Ok(Self {
            ed25519_signature,
            ml_dsa_65_signature,
        })
    }

    pub fn ed25519_signature(&self) -> &[u8; 64] {
        &self.ed25519_signature
    }

    pub fn ml_dsa_65_signature(&self) -> &[u8; ML_DSA_65_SIGNATURE_LEN] {
        &self.ml_dsa_65_signature
    }
}

/// Portable signed receipt container.  This type is intentionally **not** named
/// `Verified*`; signature bytes have not been authenticated by this contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifierMintReceiptV1 {
    statement: VerifierMintStatementV1,
    signatures: HybridVerifierSignatureV1,
}

impl VerifierMintReceiptV1 {
    pub fn new(
        statement: VerifierMintStatementV1,
        signatures: HybridVerifierSignatureV1,
    ) -> Self {
        Self {
            statement,
            signatures,
        }
    }

    pub fn statement(&self) -> &VerifierMintStatementV1 {
        &self.statement
    }

    pub fn signatures(&self) -> &HybridVerifierSignatureV1 {
        &self.signatures
    }
}

fn push_optional_digest(out: &mut Vec<u8>, value: Option<Digest>) {
    match value {
        None => out.push(0),
        Some(value) => {
            out.push(1);
            out.extend_from_slice(value.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn d(value: u8) -> Digest {
        Digest::new(b(value)).unwrap()
    }

    fn n(value: u8) -> Nonce {
        Nonce::new(b(value)).unwrap()
    }

    fn health_state() -> HealthQualificationStateV1 {
        HealthQualificationStateV1::new(d(1), d(2), 7, 9, d(3), d(4)).unwrap()
    }

    fn request() -> VerifierMintRequestV1 {
        VerifierMintRequestV1::new(
            TokenProfileV1::GenericAnchorAndSecurityTime,
            d(5),
            n(6),
            n(7),
            health_state(),
            1_000,
            2_000,
        )
        .unwrap()
    }

    fn generic() -> GenericAcceptedAnchorFieldsV1 {
        GenericAcceptedAnchorFieldsV1::new(
            Digest::new(HEALTH_GENERIC_DOMAIN_ID_V1).unwrap(),
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

    fn security_time() -> HealthSecurityTimeFieldsV1 {
        HealthSecurityTimeFieldsV1::new(3, d(11), d(8), d(15), 1_120, 1_130, 1_900).unwrap()
    }

    fn statement() -> VerifierMintStatementV1 {
        VerifierMintStatementV1::new(
            &request(),
            d(16),
            VerifierIdentityV1::new(d(17), d(18), d(19)),
            generic(),
            d(20),
            security_time(),
            d(21),
            SignatureSuiteV1::Ed25519MlDsa65,
            1_200,
            1_800,
        )
        .unwrap()
    }

    #[test]
    fn canonical_request_is_domain_separated_and_fixed_width_after_prefix() {
        let bytes = request().canonical_bytes();
        assert!(bytes.starts_with(REQUEST_DOMAIN_V1));
        assert_eq!(&bytes[REQUEST_DOMAIN_V1.len()..REQUEST_DOMAIN_V1.len() + 2], &[0, 1]);
        assert_eq!(bytes.len(), REQUEST_DOMAIN_V1.len() + 2 + 1 + 32 * 7 + 8 * 4);
    }

    #[test]
    fn canonical_statement_is_domain_separated_and_signing_bytes_are_identical() {
        let statement = statement();
        let bytes = statement.canonical_bytes();
        assert!(bytes.starts_with(STATEMENT_DOMAIN_V1));
        assert_eq!(statement.signing_transcript(), bytes);
    }

    #[test]
    fn authority_field_mutations_change_signed_bytes() {
        let baseline = statement();
        let baseline_bytes = baseline.canonical_bytes();

        let mut cases = Vec::new();

        let mut changed = baseline;
        changed.product_request_digest = d(55);
        cases.push(changed);

        let mut changed = baseline;
        changed.verifier_challenge = n(56);
        cases.push(changed);

        let mut changed = baseline;
        changed.health_state.checkpoint_epoch = 8;
        cases.push(changed);

        let mut changed = baseline;
        changed.verifier_identity.key_lineage_commitment = d(57);
        cases.push(changed);

        let mut changed = baseline;
        changed.generic_anchor.anchor_digest = d(58);
        cases.push(changed);

        let mut changed = baseline;
        changed.security_time.security_time_floor_micros += 1;
        cases.push(changed);

        for candidate in cases {
            assert_ne!(candidate.canonical_bytes(), baseline_bytes);
        }
    }

    #[test]
    fn predecessor_and_recovery_shapes_are_unambiguous() {
        assert_eq!(
            GenericAcceptedAnchorFieldsV1::new(
                Digest::new(HEALTH_GENERIC_DOMAIN_ID_V1).unwrap(),
                d(1),
                1,
                d(8),
                1,
                Some(d(9)),
                n(10),
                d(11),
                d(12),
                1,
                d(13),
                d(14),
                100,
                200,
                0,
                None,
            ),
            Err(ContractError::InvalidPredecessorShape)
        );

        let mut recovered = generic();
        recovered.recovery_authority_epoch = 1;
        assert!(recovered.resolved_recovery_receipt_digest.is_none());
        assert_eq!(
            GenericAcceptedAnchorFieldsV1::new(
                recovered.domain_id,
                recovered.subject_id,
                recovered.state_epoch,
                recovered.state_commitment,
                recovered.anchor_sequence,
                recovered.previous_anchor_digest,
                recovered.anchor_nonce,
                recovered.anchor_digest,
                recovered.witness_set_digest,
                recovered.witness_set_epoch,
                recovered.witness_quorum_digest,
                recovered.trust_snapshot_digest,
                recovered.observed_at_micros,
                recovered.expires_at_micros,
                1,
                None,
            ),
            Err(ContractError::InvalidRecoveryShape)
        );
    }

    #[test]
    fn statement_requires_exact_health_generic_and_time_binding() {
        let request = request();
        let identity = VerifierIdentityV1::new(d(17), d(18), d(19));
        let base_generic = generic();
        let base_time = security_time();

        let wrong_subject = GenericAcceptedAnchorFieldsV1 { subject_id: d(99), ..base_generic };
        assert_eq!(
            VerifierMintStatementV1::new(
                &request, d(16), identity, wrong_subject, d(20), base_time, d(21),
                SignatureSuiteV1::Ed25519MlDsa65, 1_200, 1_800,
            ),
            Err(ContractError::GenericSubjectMismatch)
        );

        let wrong_time = HealthSecurityTimeFieldsV1 { anchor_digest: d(99), ..base_time };
        assert_eq!(
            VerifierMintStatementV1::new(
                &request, d(16), identity, base_generic, d(20), wrong_time, d(21),
                SignatureSuiteV1::Ed25519MlDsa65, 1_200, 1_800,
            ),
            Err(ContractError::SecurityTimeAnchorMismatch)
        );
    }

    #[test]
    fn statement_lifetime_is_intersection_bounded() {
        assert_eq!(
            VerifierMintStatementV1::new(
                &request(), d(16), VerifierIdentityV1::new(d(17), d(18), d(19)),
                generic(), d(20), security_time(), d(21), SignatureSuiteV1::Ed25519MlDsa65,
                1_200, 1_901,
            ),
            Err(ContractError::StatementOutlivesSecurityTime)
        );
    }

    #[test]
    fn receipt_construction_is_not_verification() {
        let signatures = HybridVerifierSignatureV1::new([0x31; 64], [0x32; ML_DSA_65_SIGNATURE_LEN]).unwrap();
        let receipt = VerifierMintReceiptV1::new(statement(), signatures);
        assert_eq!(receipt.statement().signing_transcript(), statement().canonical_bytes());
    }
}
