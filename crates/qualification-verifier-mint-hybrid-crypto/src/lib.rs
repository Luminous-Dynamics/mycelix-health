#![forbid(unsafe_code)]
//! Real hybrid cryptographic verification for the exact isolated-verifier mint
//! statement frozen by QUAL-EVID-009A.
//!
//! This crate proves only:
//! - Ed25519 verifies over the exact V1 statement bytes;
//! - ML-DSA-65 verifies over those same exact bytes;
//! - the exact presented key pair equals one supplied current verifier
//!   enrollment; and
//! - the verifier identity commitments embedded in the signed statement equal
//!   that enrollment.
//!
//! It does **not** own a challenge store, daemon endpoint, trusted clock, or
//! product token mint and therefore makes no freshness/single-use or product
//! authority claim.

use ed25519_dalek::{Signature as Ed25519Signature, Verifier as _, VerifyingKey};
use ml_dsa::{
    EncodedSignature as MlDsaEncodedSignature,
    EncodedVerifyingKey as MlDsaEncodedVerifyingKey, MlDsa65,
    Signature as MlDsaSignature, VerifyingKey as MlDsaVerifyingKey,
    signature::Verifier as _,
};
use mycelix_qualification_verifier_mint_contract as contract;
use sha2::{Digest as _, Sha256};

pub const ML_DSA_65_PK_LEN: usize = 1952;
pub const ML_DSA_65_SIG_LEN: usize = contract::ML_DSA_65_SIGNATURE_LEN;

const KEY_LINEAGE_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-KEY-LINEAGE-V1\0";
const CRYPTO_EVIDENCE_DOMAIN_V1: &[u8] =
    b"MYCELIX-HEALTH-QUALIFICATION-VERIFIER-CRYPTO-EVIDENCE-V1\0";

// Frozen by the 009A V1 statement layout:
// domain(56) + schema(2) + suite(1) + profile(1) + four digests(128)
// + request times(16) = 204.
const STATEMENT_VERIFIER_IDENTITY_OFFSET_V1: usize = 204;
const STATEMENT_VERIFIER_IDENTITY_LEN_V1: usize = 96;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedVerifierMintV1 {
    receipt: contract::VerifierMintReceiptV1,
    ed25519_pubkey: [u8; 32],
    ml_dsa_65_pubkey: [u8; ML_DSA_65_PK_LEN],
}

impl SignedVerifierMintV1 {
    pub fn new(
        receipt: contract::VerifierMintReceiptV1,
        ed25519_pubkey: [u8; 32],
        ml_dsa_65_pubkey: [u8; ML_DSA_65_PK_LEN],
    ) -> Self {
        Self {
            receipt,
            ed25519_pubkey,
            ml_dsa_65_pubkey,
        }
    }

    pub fn receipt(&self) -> &contract::VerifierMintReceiptV1 {
        &self.receipt
    }

    pub fn ed25519_pubkey(&self) -> &[u8; 32] {
        &self.ed25519_pubkey
    }

    pub fn ml_dsa_65_pubkey(&self) -> &[u8; ML_DSA_65_PK_LEN] {
        &self.ml_dsa_65_pubkey
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentVerifierEnrollmentV1 {
    logical_verifier_id: String,
    provider_namespace_commitment: contract::Digest,
    verifier_instance_commitment: contract::Digest,
    key_lineage_commitment: contract::Digest,
    ed25519_pubkey: [u8; 32],
    ml_dsa_65_pubkey: [u8; ML_DSA_65_PK_LEN],
}

impl CurrentVerifierEnrollmentV1 {
    pub fn new(
        logical_verifier_id: String,
        provider_namespace_commitment: contract::Digest,
        verifier_instance_commitment: contract::Digest,
        ed25519_pubkey: [u8; 32],
        ml_dsa_65_pubkey: [u8; ML_DSA_65_PK_LEN],
    ) -> Result<Self, MintVerifierError> {
        if logical_verifier_id.is_empty() {
            return Err(MintVerifierError::EmptyLogicalVerifierId);
        }
        let key_lineage_commitment =
            key_lineage_commitment(&ed25519_pubkey, &ml_dsa_65_pubkey)?;
        Ok(Self {
            logical_verifier_id,
            provider_namespace_commitment,
            verifier_instance_commitment,
            key_lineage_commitment,
            ed25519_pubkey,
            ml_dsa_65_pubkey,
        })
    }

    pub fn logical_verifier_id(&self) -> &str {
        &self.logical_verifier_id
    }

    pub fn provider_namespace_commitment(&self) -> contract::Digest {
        self.provider_namespace_commitment
    }

    pub fn verifier_instance_commitment(&self) -> contract::Digest {
        self.verifier_instance_commitment
    }

    pub fn key_lineage_commitment(&self) -> contract::Digest {
        self.key_lineage_commitment
    }

    pub fn statement_identity(&self) -> contract::VerifierIdentityV1 {
        contract::VerifierIdentityV1::new(
            self.provider_namespace_commitment,
            self.verifier_instance_commitment,
            self.key_lineage_commitment,
        )
    }
}

/// Positive result proving only that both signatures verified over one exact
/// 009A statement with one exact presented hybrid key pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedVerifierMintSignaturesV1 {
    signed: SignedVerifierMintV1,
    statement_digest: contract::Digest,
}

impl VerifiedVerifierMintSignaturesV1 {
    pub fn signed(&self) -> &SignedVerifierMintV1 {
        &self.signed
    }

    pub fn statement_digest(&self) -> contract::Digest {
        self.statement_digest
    }
}

/// Positive result additionally proving the exact verified pair equals one
/// supplied current verifier enrollment and that the signed verifier identity
/// commitments equal that enrollment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedVerifierMintCryptographyV1 {
    logical_verifier_id: String,
    statement_digest: contract::Digest,
    provider_namespace_commitment: contract::Digest,
    verifier_instance_commitment: contract::Digest,
    key_lineage_commitment: contract::Digest,
    cryptographic_evidence_commitment: contract::Digest,
}

impl VerifiedVerifierMintCryptographyV1 {
    pub fn logical_verifier_id(&self) -> &str {
        &self.logical_verifier_id
    }

    pub fn statement_digest(&self) -> contract::Digest {
        self.statement_digest
    }

    pub fn provider_namespace_commitment(&self) -> contract::Digest {
        self.provider_namespace_commitment
    }

    pub fn verifier_instance_commitment(&self) -> contract::Digest {
        self.verifier_instance_commitment
    }

    pub fn key_lineage_commitment(&self) -> contract::Digest {
        self.key_lineage_commitment
    }

    pub fn cryptographic_evidence_commitment(&self) -> contract::Digest {
        self.cryptographic_evidence_commitment
    }
}

pub fn verify_mint_signatures_v1(
    signed: SignedVerifierMintV1,
) -> Result<VerifiedVerifierMintSignaturesV1, MintVerifierError> {
    let transcript = signed.receipt.statement().signing_transcript();
    let signatures = signed.receipt.signatures();

    let ed_vk = VerifyingKey::from_bytes(&signed.ed25519_pubkey)
        .map_err(|_| MintVerifierError::MalformedEd25519Key)?;
    let ed_sig = Ed25519Signature::from_bytes(signatures.ed25519_signature());
    ed_vk
        .verify(&transcript, &ed_sig)
        .map_err(|_| MintVerifierError::Ed25519VerifyFailed)?;

    let encoded_key = MlDsaEncodedVerifyingKey::<MlDsa65>::try_from(
        signed.ml_dsa_65_pubkey.as_slice(),
    )
    .map_err(|_| MintVerifierError::MalformedMlDsaKey)?;
    let ml_vk = MlDsaVerifyingKey::<MlDsa65>::decode(&encoded_key);
    let encoded_sig = MlDsaEncodedSignature::<MlDsa65>::try_from(
        signatures.ml_dsa_65_signature().as_slice(),
    )
    .map_err(|_| MintVerifierError::MalformedMlDsaSignature)?;
    let ml_sig = MlDsaSignature::<MlDsa65>::decode(&encoded_sig)
        .ok_or(MintVerifierError::MalformedMlDsaSignature)?;
    ml_vk
        .verify(&transcript, &ml_sig)
        .map_err(|_| MintVerifierError::MlDsaVerifyFailed)?;

    let statement_digest = sha256_digest(&transcript)?;
    Ok(VerifiedVerifierMintSignaturesV1 {
        signed,
        statement_digest,
    })
}

pub fn bind_verified_mint_to_enrollment_v1(
    verified: VerifiedVerifierMintSignaturesV1,
    enrollment: &CurrentVerifierEnrollmentV1,
) -> Result<VerifiedVerifierMintCryptographyV1, MintVerifierError> {
    let signed = verified.signed();
    if signed.ed25519_pubkey != enrollment.ed25519_pubkey
        || signed.ml_dsa_65_pubkey != enrollment.ml_dsa_65_pubkey
    {
        return Err(MintVerifierError::EnrollmentKeyMismatch);
    }

    let (provider, instance, lineage) =
        statement_verifier_identity_commitments(signed.receipt.statement())?;
    if provider != enrollment.provider_namespace_commitment
        || instance != enrollment.verifier_instance_commitment
        || lineage != enrollment.key_lineage_commitment
    {
        return Err(MintVerifierError::EnrollmentIdentityMismatch);
    }

    let cryptographic_evidence_commitment = cryptographic_evidence_commitment(&verified)?;
    Ok(VerifiedVerifierMintCryptographyV1 {
        logical_verifier_id: enrollment.logical_verifier_id.clone(),
        statement_digest: verified.statement_digest,
        provider_namespace_commitment: enrollment.provider_namespace_commitment,
        verifier_instance_commitment: enrollment.verifier_instance_commitment,
        key_lineage_commitment: enrollment.key_lineage_commitment,
        cryptographic_evidence_commitment,
    })
}

pub fn key_lineage_commitment(
    ed25519_pubkey: &[u8; 32],
    ml_dsa_65_pubkey: &[u8; ML_DSA_65_PK_LEN],
) -> Result<contract::Digest, MintVerifierError> {
    let mut hasher = Sha256::new();
    hasher.update(KEY_LINEAGE_DOMAIN_V1);
    hasher.update(ed25519_pubkey);
    hasher.update(ml_dsa_65_pubkey);
    digest_from_sha256(hasher.finalize().into())
}

fn statement_verifier_identity_commitments(
    statement: &contract::VerifierMintStatementV1,
) -> Result<(contract::Digest, contract::Digest, contract::Digest), MintVerifierError> {
    let bytes = statement.canonical_bytes();
    let end = STATEMENT_VERIFIER_IDENTITY_OFFSET_V1 + STATEMENT_VERIFIER_IDENTITY_LEN_V1;
    if bytes.len() < end {
        return Err(MintVerifierError::MalformedStatementLayout);
    }
    let provider = digest_from_slice(
        &bytes[STATEMENT_VERIFIER_IDENTITY_OFFSET_V1..STATEMENT_VERIFIER_IDENTITY_OFFSET_V1 + 32],
    )?;
    let instance = digest_from_slice(
        &bytes[STATEMENT_VERIFIER_IDENTITY_OFFSET_V1 + 32
            ..STATEMENT_VERIFIER_IDENTITY_OFFSET_V1 + 64],
    )?;
    let lineage = digest_from_slice(
        &bytes[STATEMENT_VERIFIER_IDENTITY_OFFSET_V1 + 64..end],
    )?;
    Ok((provider, instance, lineage))
}

fn cryptographic_evidence_commitment(
    verified: &VerifiedVerifierMintSignaturesV1,
) -> Result<contract::Digest, MintVerifierError> {
    let transcript = verified.signed.receipt.statement().signing_transcript();
    let signatures = verified.signed.receipt.signatures();
    let mut hasher = Sha256::new();
    hasher.update(CRYPTO_EVIDENCE_DOMAIN_V1);
    hasher.update((transcript.len() as u32).to_be_bytes());
    hasher.update(&transcript);
    hasher.update(signatures.ed25519_signature());
    hasher.update(signatures.ml_dsa_65_signature());
    digest_from_sha256(hasher.finalize().into())
}

fn sha256_digest(bytes: &[u8]) -> Result<contract::Digest, MintVerifierError> {
    digest_from_sha256(Sha256::digest(bytes).into())
}

fn digest_from_slice(bytes: &[u8]) -> Result<contract::Digest, MintVerifierError> {
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| MintVerifierError::MalformedStatementLayout)?;
    contract::Digest::new(array).map_err(|_| MintVerifierError::ZeroDerivedDigest)
}

fn digest_from_sha256(bytes: [u8; 32]) -> Result<contract::Digest, MintVerifierError> {
    contract::Digest::new(bytes).map_err(|_| MintVerifierError::ZeroDerivedDigest)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MintVerifierError {
    EmptyLogicalVerifierId,
    MalformedEd25519Key,
    Ed25519VerifyFailed,
    MalformedMlDsaKey,
    MalformedMlDsaSignature,
    MlDsaVerifyFailed,
    EnrollmentKeyMismatch,
    EnrollmentIdentityMismatch,
    MalformedStatementLayout,
    ZeroDerivedDigest,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer as _, SigningKey};
    use ml_dsa::{
        B32, Signature as MlDsaSignatureT, SigningKey as MlDsaSigningKey,
        signature::{Keypair as _, Signer as _},
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

    struct TestIdentity {
        ed: SigningKey,
        ml: MlDsaSigningKey<MlDsa65>,
    }

    impl TestIdentity {
        fn from_seeds(ed: [u8; 32], ml: [u8; 32]) -> Self {
            let seed: B32 = ml.into();
            Self {
                ed: SigningKey::from_bytes(&ed),
                ml: MlDsaSigningKey::<MlDsa65>::from_seed(&seed),
            }
        }

        fn ed_pk(&self) -> [u8; 32] {
            self.ed.verifying_key().to_bytes()
        }

        fn ml_pk(&self) -> [u8; ML_DSA_65_PK_LEN] {
            let encoded = self.ml.verifying_key().encode();
            let mut out = [0_u8; ML_DSA_65_PK_LEN];
            out.copy_from_slice(encoded.as_slice());
            out
        }

        fn signatures(&self, transcript: &[u8]) -> contract::HybridVerifierSignatureV1 {
            let ed = self.ed.sign(transcript).to_bytes();
            let ml_signature: MlDsaSignatureT<MlDsa65> = self.ml.sign(transcript);
            let encoded = ml_signature.encode();
            let mut ml = [0_u8; ML_DSA_65_SIG_LEN];
            ml.copy_from_slice(encoded.as_slice());
            contract::HybridVerifierSignatureV1::new(ed, ml).unwrap()
        }
    }

    fn enrollment(identity: &TestIdentity, instance_byte: u8) -> CurrentVerifierEnrollmentV1 {
        CurrentVerifierEnrollmentV1::new(
            format!("health-verifier:{instance_byte}"),
            d(40),
            d(instance_byte),
            identity.ed_pk(),
            identity.ml_pk(),
        )
        .unwrap()
    }

    fn request(product_byte: u8) -> contract::VerifierMintRequestV1 {
        let state = contract::HealthQualificationStateV1::new(d(1), d(2), 7, 9, d(3), d(4))
            .unwrap();
        contract::VerifierMintRequestV1::new(
            contract::TokenProfileV1::GenericAnchorAndSecurityTime,
            d(product_byte),
            n(6),
            n(7),
            state,
            1_000,
            2_000,
        )
        .unwrap()
    }

    fn statement(
        request: &contract::VerifierMintRequestV1,
        enrollment: &CurrentVerifierEnrollmentV1,
    ) -> contract::VerifierMintStatementV1 {
        let generic = contract::GenericAcceptedAnchorFieldsV1::new(
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
        .unwrap();
        let security = contract::HealthSecurityTimeFieldsV1::new(
            3,
            d(11),
            d(8),
            d(15),
            1_120,
            1_130,
            1_900,
        )
        .unwrap();
        contract::VerifierMintStatementV1::new(
            request,
            d(16),
            enrollment.statement_identity(),
            generic,
            d(20),
            security,
            d(21),
            contract::SignatureSuiteV1::Ed25519MlDsa65,
            1_200,
            1_800,
        )
        .unwrap()
    }

    fn signed(
        statement: contract::VerifierMintStatementV1,
        ed_signer: &TestIdentity,
        ml_signer: &TestIdentity,
    ) -> SignedVerifierMintV1 {
        let transcript = statement.signing_transcript();
        let ed = ed_signer.ed.sign(&transcript).to_bytes();
        let ml_signature: MlDsaSignatureT<MlDsa65> = ml_signer.ml.sign(&transcript);
        let encoded = ml_signature.encode();
        let mut ml = [0_u8; ML_DSA_65_SIG_LEN];
        ml.copy_from_slice(encoded.as_slice());
        let signatures = contract::HybridVerifierSignatureV1::new(ed, ml).unwrap();
        SignedVerifierMintV1::new(
            contract::VerifierMintReceiptV1::new(statement, signatures),
            ed_signer.ed_pk(),
            ml_signer.ml_pk(),
        )
    }

    #[test]
    fn exact_hybrid_signatures_bind_to_current_enrollment() {
        let signer = TestIdentity::from_seeds(b(0x11), b(0x12));
        let enrollment = enrollment(&signer, 41);
        let statement = statement(&request(5), &enrollment);
        let verified = verify_mint_signatures_v1(signed(statement, &signer, &signer)).unwrap();
        let bound = bind_verified_mint_to_enrollment_v1(verified, &enrollment).unwrap();
        assert_eq!(bound.logical_verifier_id(), "health-verifier:41");
        assert_eq!(bound.key_lineage_commitment(), enrollment.key_lineage_commitment());
    }

    #[test]
    fn statement_substitution_breaks_signature_verification() {
        let signer = TestIdentity::from_seeds(b(0x21), b(0x22));
        let enrollment = enrollment(&signer, 42);
        let original = statement(&request(5), &enrollment);
        let signatures = signer.signatures(&original.signing_transcript());
        let substituted = statement(&request(55), &enrollment);
        let proof = SignedVerifierMintV1::new(
            contract::VerifierMintReceiptV1::new(substituted, signatures),
            signer.ed_pk(),
            signer.ml_pk(),
        );
        assert_eq!(
            verify_mint_signatures_v1(proof).unwrap_err(),
            MintVerifierError::Ed25519VerifyFailed
        );
    }

    #[test]
    fn both_signatures_can_be_valid_while_pair_is_not_jointly_enrolled() {
        let classical = TestIdentity::from_seeds(b(0x31), b(0x32));
        let pq = TestIdentity::from_seeds(b(0x33), b(0x34));
        let enrollment = enrollment(&classical, 43);
        let statement = statement(&request(5), &enrollment);
        let verified = verify_mint_signatures_v1(signed(statement, &classical, &pq)).unwrap();
        assert_eq!(
            bind_verified_mint_to_enrollment_v1(verified, &enrollment).unwrap_err(),
            MintVerifierError::EnrollmentKeyMismatch
        );
    }

    #[test]
    fn verifier_key_rotation_invalidates_old_pair_binding() {
        let old = TestIdentity::from_seeds(b(0x41), b(0x42));
        let replacement = TestIdentity::from_seeds(b(0x43), b(0x44));
        let old_enrollment = enrollment(&old, 44);
        let replacement_enrollment = enrollment(&replacement, 44);
        let statement = statement(&request(5), &old_enrollment);
        let verified = verify_mint_signatures_v1(signed(statement, &old, &old)).unwrap();
        assert_eq!(
            bind_verified_mint_to_enrollment_v1(verified, &replacement_enrollment).unwrap_err(),
            MintVerifierError::EnrollmentKeyMismatch
        );
    }

    #[test]
    fn signed_verifier_identity_must_equal_current_enrollment() {
        let signer = TestIdentity::from_seeds(b(0x51), b(0x52));
        let statement_enrollment = enrollment(&signer, 45);
        let current = CurrentVerifierEnrollmentV1::new(
            "health-verifier:45".into(),
            d(99),
            statement_enrollment.verifier_instance_commitment(),
            signer.ed_pk(),
            signer.ml_pk(),
        )
        .unwrap();
        let statement = statement(&request(5), &statement_enrollment);
        let verified = verify_mint_signatures_v1(signed(statement, &signer, &signer)).unwrap();
        assert_eq!(
            bind_verified_mint_to_enrollment_v1(verified, &current).unwrap_err(),
            MintVerifierError::EnrollmentIdentityMismatch
        );
    }

    #[test]
    fn malformed_ml_dsa_key_never_verifies() {
        let signer = TestIdentity::from_seeds(b(0x61), b(0x62));
        let enrollment = enrollment(&signer, 46);
        let statement = statement(&request(5), &enrollment);
        let transcript = statement.signing_transcript();
        let signatures = signer.signatures(&transcript);
        let proof = SignedVerifierMintV1::new(
            contract::VerifierMintReceiptV1::new(statement, signatures),
            signer.ed_pk(),
            [0xff; ML_DSA_65_PK_LEN],
        );
        assert!(matches!(
            verify_mint_signatures_v1(proof),
            Err(MintVerifierError::MalformedMlDsaKey)
                | Err(MintVerifierError::MlDsaVerifyFailed)
        ));
    }

    #[test]
    fn standalone_crypto_type_carries_no_freshness_or_mint_claim() {
        let signer = TestIdentity::from_seeds(b(0x71), b(0x72));
        let enrollment = enrollment(&signer, 47);
        let statement = statement(&request(5), &enrollment);
        let verified = verify_mint_signatures_v1(signed(statement, &signer, &signer)).unwrap();
        let bound = bind_verified_mint_to_enrollment_v1(verified, &enrollment).unwrap();
        assert_ne!(*bound.cryptographic_evidence_commitment().as_bytes(), [0; 32]);
        // There is intentionally no challenge-consumed, daemon-authenticated,
        // trusted-time, or token-minted field/method on this positive type.
    }
}
