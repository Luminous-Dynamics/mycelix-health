#![forbid(unsafe_code)]
//! Exact qualification adapter-profile semantics for sensitive authority seams.
//!
//! This reference consumes `VerifiedQualification` values from
//! `mycelix-qualification-receipt-core`. It does not verify signatures, compute
//! receipt/profile hashes, load governance policy, or provide trusted time.

use core::fmt;
use mycelix_qualification_receipt_core as qual;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdapterSeamId {
    Patient204ActivationSide = 1,
    Patient206MigrationResult = 2,
    Patient208CompositionCommitment = 3,
    Patient208DurablePrepare = 4,
    Patient208DurableActivation = 5,
    Audit190Checkpoint = 6,
    Monitoring184Assessment = 7,
    SecurityP0Closure = 8,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdapterProfileDigest([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterProfileDigestError {
    AllZero,
}

impl AdapterProfileDigest {
    pub fn new(bytes: [u8; 32]) -> Result<Self, AdapterProfileDigestError> {
        if bytes == [0; 32] {
            return Err(AdapterProfileDigestError::AllZero);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AdapterProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AdapterProfileDigest([redacted])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileDefinitionError {
    InvalidMaximumAge,
}

/// Exact governed profile for the first real QUAL-EVID-002 seam:
/// Patient-v2 #208 composition-commitment qualification.
///
/// The expected binding is constructed internally with `AdmissionScope::Cutover`
/// and a mandatory context, so callers cannot reinterpret the same profile as a
/// reference/deployment/monitoring qualification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile208CompositionCommitment {
    profile_digest: AdapterProfileDigest,
    binding: qual::QualificationLineageBinding,
    policy_digest: qual::GovernancePolicyDigest,
    trust_store_digest: qual::TrustStoreDigest,
    deployment_evidence: qual::DeploymentEvidenceDigest,
    max_consumption_age_micros: i64,
}

#[allow(clippy::too_many_arguments)]
impl Profile208CompositionCommitment {
    pub fn new(
        profile_digest: AdapterProfileDigest,
        lineage: qual::QualificationLineageId,
        contract: qual::ContractRef,
        subject: qual::SubjectDigest,
        context: qual::ContextDigest,
        claim_profile: qual::ClaimProfileDigest,
        policy_digest: qual::GovernancePolicyDigest,
        trust_store_digest: qual::TrustStoreDigest,
        deployment_evidence: qual::DeploymentEvidenceDigest,
        max_consumption_age_micros: i64,
    ) -> Result<Self, ProfileDefinitionError> {
        if max_consumption_age_micros <= 0 {
            return Err(ProfileDefinitionError::InvalidMaximumAge);
        }
        Ok(Self {
            profile_digest,
            binding: qual::QualificationLineageBinding::new(
                lineage,
                contract,
                subject,
                Some(context),
                qual::AdmissionScope::Cutover,
                claim_profile,
            ),
            policy_digest,
            trust_store_digest,
            deployment_evidence,
            max_consumption_age_micros,
        })
    }

    pub fn seam(&self) -> AdapterSeamId {
        AdapterSeamId::Patient208CompositionCommitment
    }

    pub fn profile_digest(&self) -> AdapterProfileDigest {
        self.profile_digest
    }

    pub fn binding(&self) -> qual::QualificationLineageBinding {
        self.binding
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterConversionError {
    WrongReceiptKind,
    LineageBindingMismatch,
    LedgerBindingMismatch,
    PolicyMismatch,
    TrustStoreMismatch,
    DeploymentEvidenceMismatch,
    ReceiptNotAdmissible,
    ReceiptNotYetValid,
    ReceiptExpired,
    ReceiptTooOld,
}

/// Non-cloneable typed authority token for the exact #208 composition seam.
///
/// It preserves qualification provenance but intentionally exposes no generic
/// constructor. Production integration should consume this token (or a strictly
/// stronger successor), not `VerifiedQualification` directly.
pub struct Profile208CompositionCommitmentToken {
    receipt_digest: qual::ReceiptDigest,
    profile_digest: AdapterProfileDigest,
    binding: qual::QualificationLineageBinding,
    policy_digest: qual::GovernancePolicyDigest,
    trust_store_digest: qual::TrustStoreDigest,
    deployment_evidence: qual::DeploymentEvidenceDigest,
    issued_at_micros: i64,
    expires_at_micros: i64,
}

impl fmt::Debug for Profile208CompositionCommitmentToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile208CompositionCommitmentToken")
            .field("seam", &AdapterSeamId::Patient208CompositionCommitment)
            .field("receipt_digest", &self.receipt_digest)
            .field("profile_digest", &self.profile_digest)
            .field("binding", &self.binding)
            .field("issued_at_micros", &self.issued_at_micros)
            .field("expires_at_micros", &self.expires_at_micros)
            .finish_non_exhaustive()
    }
}

impl Profile208CompositionCommitmentToken {
    pub fn seam(&self) -> AdapterSeamId {
        AdapterSeamId::Patient208CompositionCommitment
    }

    pub fn receipt_digest(&self) -> qual::ReceiptDigest {
        self.receipt_digest
    }

    pub fn profile_digest(&self) -> AdapterProfileDigest {
        self.profile_digest
    }

    pub fn binding(&self) -> qual::QualificationLineageBinding {
        self.binding
    }

    pub fn policy_digest(&self) -> qual::GovernancePolicyDigest {
        self.policy_digest
    }

    pub fn trust_store_digest(&self) -> qual::TrustStoreDigest {
        self.trust_store_digest
    }

    pub fn deployment_evidence(&self) -> qual::DeploymentEvidenceDigest {
        self.deployment_evidence
    }

    pub fn issued_at_micros(&self) -> i64 {
        self.issued_at_micros
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }
}

/// Convert one already-governed qualification into the exact #208
/// composition-commitment authority token.
///
/// This conversion performs an independent product-seam check after generic
/// receipt verification. It also requires the receipt to remain active in the
/// exact semantic-lineage ledger at consumption time.
pub fn convert_profile208_composition_commitment(
    qualification: &qual::VerifiedQualification,
    ledger: &qual::QualificationLedger,
    profile: Profile208CompositionCommitment,
    now_micros: i64,
) -> Result<Profile208CompositionCommitmentToken, AdapterConversionError> {
    if qualification.kind() != qual::ReceiptKind::CompositionCommitmentQualification {
        return Err(AdapterConversionError::WrongReceiptKind);
    }
    if qualification.binding() != profile.binding {
        return Err(AdapterConversionError::LineageBindingMismatch);
    }
    if ledger.binding() != profile.binding {
        return Err(AdapterConversionError::LedgerBindingMismatch);
    }
    if qualification.policy_digest() != profile.policy_digest {
        return Err(AdapterConversionError::PolicyMismatch);
    }
    if qualification.trust_store_digest() != profile.trust_store_digest {
        return Err(AdapterConversionError::TrustStoreMismatch);
    }
    if qualification.deployment_evidence() != Some(profile.deployment_evidence) {
        return Err(AdapterConversionError::DeploymentEvidenceMismatch);
    }
    if now_micros < qualification.issued_at_micros() {
        return Err(AdapterConversionError::ReceiptNotYetValid);
    }
    if now_micros >= qualification.expires_at_micros() {
        return Err(AdapterConversionError::ReceiptExpired);
    }
    if now_micros - qualification.issued_at_micros() > profile.max_consumption_age_micros {
        return Err(AdapterConversionError::ReceiptTooOld);
    }
    if !ledger.is_admissible(qualification.receipt_digest()) {
        return Err(AdapterConversionError::ReceiptNotAdmissible);
    }

    Ok(Profile208CompositionCommitmentToken {
        receipt_digest: qualification.receipt_digest(),
        profile_digest: profile.profile_digest,
        binding: qualification.binding(),
        policy_digest: qualification.policy_digest(),
        trust_store_digest: qualification.trust_store_digest(),
        deployment_evidence: profile.deployment_evidence,
        issued_at_micros: qualification.issued_at_micros(),
        expires_at_micros: qualification.expires_at_micros(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qual::{
        ApproverId, ApproverRole, BoundApproverVerification, BoundReceiptCommitment,
        DependencySetDigest, DispositionEvidenceDigest, GovernedAdmissionPolicy,
        GovernedDispositionEvidence, GovernanceRule, Nonce, OrganizationId,
        QualificationLineageBinding, ReceiptDisposition, ReceiptOutcome, RoleSet, SignerKeyId,
    };

    fn bytes(value: u8) -> [u8; 32] {
        [value; 32]
    }

    macro_rules! oid {
        ($ty:ident, $value:expr) => {
            qual::$ty::new(bytes($value)).unwrap()
        };
    }

    fn contract() -> qual::ContractRef {
        qual::ContractRef::new(oid!(ContractDigest, 1), 1).unwrap()
    }

    fn binding(subject: u8) -> QualificationLineageBinding {
        QualificationLineageBinding::new(
            oid!(QualificationLineageId, 2),
            contract(),
            oid!(SubjectDigest, subject),
            Some(oid!(ContextDigest, 6)),
            qual::AdmissionScope::Cutover,
            oid!(ClaimProfileDigest, 9),
        )
    }

    fn profile(subject: u8, max_age: i64) -> Profile208CompositionCommitment {
        Profile208CompositionCommitment::new(
            AdapterProfileDigest::new(bytes(90)).unwrap(),
            oid!(QualificationLineageId, 2),
            contract(),
            oid!(SubjectDigest, subject),
            oid!(ContextDigest, 6),
            oid!(ClaimProfileDigest, 9),
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            oid!(DeploymentEvidenceDigest, 5),
            max_age,
        )
        .unwrap()
    }

    fn attestation(
        receipt: qual::ReceiptDigest,
        seed: u8,
        role: ApproverRole,
    ) -> BoundApproverVerification {
        BoundApproverVerification::from_verifier_adapter(
            receipt,
            oid!(ApproverId, seed),
            oid!(OrganizationId, seed + 1),
            oid!(SignerKeyId, seed + 2),
            RoleSet::from_roles(&[role]),
            100,
            true,
            true,
            true,
            false,
            false,
        )
        .unwrap()
    }

    fn verified(
        kind: qual::ReceiptKind,
        subject: u8,
        receipt_value: u8,
        policy_value: u8,
        trust_value: u8,
        deployment_value: u8,
        issued_at: i64,
        expires_at: i64,
    ) -> qual::VerifiedQualification {
        let bind = binding(subject);
        let policy_digest = oid!(GovernancePolicyDigest, policy_value);
        let trust_store = oid!(TrustStoreDigest, trust_value);
        let deployment = oid!(DeploymentEvidenceDigest, deployment_value);
        let rule = GovernanceRule::new(
            kind,
            2,
            2,
            RoleSet::from_roles(&[ApproverRole::Privacy, ApproverRole::Safety]),
            1_000,
            5_000,
        )
        .unwrap();
        let policy = GovernedAdmissionPolicy::new(
            policy_digest,
            trust_store,
            Some(deployment),
            rule,
            contract(),
            qual::AdmissionScope::Cutover,
            oid!(ClaimProfileDigest, 9),
        );
        let receipt = oid!(ReceiptDigest, receipt_value);
        let statement = qual::ScopedQualificationStatement::new(
            kind,
            bind,
            oid!(DependencySetDigest, 4),
            Some(deployment),
            ReceiptOutcome::Qualified,
            issued_at,
            expires_at,
            oid!(Nonce, receipt_value + 20),
            1,
            None,
            policy_digest,
            trust_store,
        )
        .unwrap();
        qual::verify_qualification(
            statement,
            BoundReceiptCommitment::from_hash_adapter(receipt, true),
            policy,
            &[
                attestation(receipt, 20, ApproverRole::Privacy),
                attestation(receipt, 30, ApproverRole::Safety),
            ],
            200,
        )
        .unwrap()
    }

    fn admitted(qualification: &qual::VerifiedQualification) -> qual::QualificationLedger {
        let mut ledger = qual::QualificationLedger::new(qualification.binding());
        ledger.admit(qualification, 200).unwrap();
        ledger
    }

    #[test]
    fn exact_active_composition_receipt_converts_to_one_typed_token() {
        let qualification = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            3,
            40,
            7,
            8,
            5,
            10,
            1_000,
        );
        let ledger = admitted(&qualification);
        let token = convert_profile208_composition_commitment(
            &qualification,
            &ledger,
            profile(3, 500),
            200,
        )
        .unwrap();
        assert_eq!(token.seam(), AdapterSeamId::Patient208CompositionCommitment);
        assert_eq!(token.receipt_digest(), oid!(ReceiptDigest, 40));
        assert_eq!(token.policy_digest(), oid!(GovernancePolicyDigest, 7));
        assert_eq!(token.deployment_evidence(), oid!(DeploymentEvidenceDigest, 5));
    }

    #[test]
    fn migration_receipt_cannot_become_composition_authority() {
        let qualification = verified(
            qual::ReceiptKind::MigrationResultQualification,
            3,
            40,
            7,
            8,
            5,
            10,
            1_000,
        );
        let ledger = admitted(&qualification);
        assert_eq!(
            convert_profile208_composition_commitment(&qualification, &ledger, profile(3, 500), 200)
                .map(|_| ()),
            Err(AdapterConversionError::WrongReceiptKind)
        );
    }

    #[test]
    fn wrong_semantic_lineage_is_rejected() {
        let qualification = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            4,
            40,
            7,
            8,
            5,
            10,
            1_000,
        );
        let ledger = admitted(&qualification);
        assert_eq!(
            convert_profile208_composition_commitment(&qualification, &ledger, profile(3, 500), 200)
                .map(|_| ()),
            Err(AdapterConversionError::LineageBindingMismatch)
        );
    }

    #[test]
    fn wrong_policy_trust_or_deployment_is_rejected() {
        let wrong_policy = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            3,
            40,
            70,
            8,
            5,
            10,
            1_000,
        );
        let ledger = admitted(&wrong_policy);
        assert_eq!(
            convert_profile208_composition_commitment(&wrong_policy, &ledger, profile(3, 500), 200)
                .map(|_| ()),
            Err(AdapterConversionError::PolicyMismatch)
        );

        let wrong_trust = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            3,
            41,
            7,
            80,
            5,
            10,
            1_000,
        );
        let ledger = admitted(&wrong_trust);
        assert_eq!(
            convert_profile208_composition_commitment(&wrong_trust, &ledger, profile(3, 500), 200)
                .map(|_| ()),
            Err(AdapterConversionError::TrustStoreMismatch)
        );

        let wrong_deployment = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            3,
            42,
            7,
            8,
            50,
            10,
            1_000,
        );
        let ledger = admitted(&wrong_deployment);
        assert_eq!(
            convert_profile208_composition_commitment(
                &wrong_deployment,
                &ledger,
                profile(3, 500),
                200,
            )
            .map(|_| ()),
            Err(AdapterConversionError::DeploymentEvidenceMismatch)
        );
    }

    #[test]
    fn superseded_receipt_is_not_admissible_at_the_seam() {
        let qualification = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            3,
            40,
            7,
            8,
            5,
            10,
            1_000,
        );
        let mut ledger = admitted(&qualification);
        ledger
            .apply_disposition(GovernedDispositionEvidence::from_governance_adapter(
                oid!(QualificationLineageId, 2),
                qualification.receipt_digest(),
                ReceiptDisposition::Supersede,
                oid!(DispositionEvidenceDigest, 60),
                1,
                true,
            ))
            .unwrap();
        assert_eq!(
            convert_profile208_composition_commitment(&qualification, &ledger, profile(3, 500), 200)
                .map(|_| ()),
            Err(AdapterConversionError::ReceiptNotAdmissible)
        );
    }

    #[test]
    fn seam_rechecks_expiry_and_stricter_consumption_age() {
        let qualification = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            3,
            40,
            7,
            8,
            5,
            10,
            1_000,
        );
        let ledger = admitted(&qualification);
        assert_eq!(
            convert_profile208_composition_commitment(&qualification, &ledger, profile(3, 50), 200)
                .map(|_| ()),
            Err(AdapterConversionError::ReceiptTooOld)
        );
        assert_eq!(
            convert_profile208_composition_commitment(&qualification, &ledger, profile(3, 5_000), 1_000)
                .map(|_| ()),
            Err(AdapterConversionError::ReceiptExpired)
        );
    }

    #[test]
    fn ledger_must_be_bound_to_the_same_semantic_lineage() {
        let qualification = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            3,
            40,
            7,
            8,
            5,
            10,
            1_000,
        );
        let wrong = verified(
            qual::ReceiptKind::CompositionCommitmentQualification,
            99,
            41,
            7,
            8,
            5,
            10,
            1_000,
        );
        let ledger = admitted(&wrong);
        assert_eq!(
            convert_profile208_composition_commitment(&qualification, &ledger, profile(3, 500), 200)
                .map(|_| ()),
            Err(AdapterConversionError::LedgerBindingMismatch)
        );
    }

    #[test]
    fn profile_requires_positive_consumption_age() {
        assert_eq!(
            Profile208CompositionCommitment::new(
                AdapterProfileDigest::new(bytes(90)).unwrap(),
                oid!(QualificationLineageId, 2),
                contract(),
                oid!(SubjectDigest, 3),
                oid!(ContextDigest, 6),
                oid!(ClaimProfileDigest, 9),
                oid!(GovernancePolicyDigest, 7),
                oid!(TrustStoreDigest, 8),
                oid!(DeploymentEvidenceDigest, 5),
                0,
            ),
            Err(ProfileDefinitionError::InvalidMaximumAge)
        );
    }
}
