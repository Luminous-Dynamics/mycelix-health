#![forbid(unsafe_code)]
//! Exact qualification adapter-profile semantics for sensitive authority seams.
//!
//! This reference consumes `VerifiedQualification` values from
//! `mycelix-qualification-receipt-core`. It does not verify signatures, compute
//! receipt/profile hashes, load governance policy, provide trusted time, or
//! durably persist the seam-consumption clock.

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
    ClockRollback,
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

/// Stateful consumption boundary for the exact #208 composition seam.
///
/// The gate observes time before any semantic checks. Once it has observed a
/// later time, a retry with an earlier time is rejected even when an earlier
/// timestamp would otherwise make the receipt fresh again.
pub struct Profile208CompositionCommitmentGate {
    profile: Profile208CompositionCommitment,
    latest_time_micros: Option<i64>,
}

impl Profile208CompositionCommitmentGate {
    pub fn new(profile: Profile208CompositionCommitment) -> Self {
        Self {
            profile,
            latest_time_micros: None,
        }
    }

    pub fn profile(&self) -> Profile208CompositionCommitment {
        self.profile
    }

    pub fn latest_time_micros(&self) -> Option<i64> {
        self.latest_time_micros
    }

    fn observe_time(&mut self, now_micros: i64) -> Result<(), AdapterConversionError> {
        if self
            .latest_time_micros
            .is_some_and(|previous| now_micros < previous)
        {
            return Err(AdapterConversionError::ClockRollback);
        }
        self.latest_time_micros = Some(now_micros);
        Ok(())
    }

    /// Convert one already-governed qualification into the exact #208
    /// composition-commitment authority token.
    ///
    /// This performs an independent product-seam check after generic receipt
    /// verification and requires the receipt to remain active in the exact
    /// semantic-lineage ledger at consumption time.
    pub fn convert(
        &mut self,
        qualification: &qual::VerifiedQualification,
        ledger: &qual::QualificationLedger,
        now_micros: i64,
    ) -> Result<Profile208CompositionCommitmentToken, AdapterConversionError> {
        self.observe_time(now_micros)?;

        if qualification.kind() != qual::ReceiptKind::CompositionCommitmentQualification {
            return Err(AdapterConversionError::WrongReceiptKind);
        }
        if qualification.binding() != self.profile.binding {
            return Err(AdapterConversionError::LineageBindingMismatch);
        }
        if ledger.binding() != self.profile.binding {
            return Err(AdapterConversionError::LedgerBindingMismatch);
        }
        if qualification.policy_digest() != self.profile.policy_digest {
            return Err(AdapterConversionError::PolicyMismatch);
        }
        if qualification.trust_store_digest() != self.profile.trust_store_digest {
            return Err(AdapterConversionError::TrustStoreMismatch);
        }
        if qualification.deployment_evidence() != Some(self.profile.deployment_evidence) {
            return Err(AdapterConversionError::DeploymentEvidenceMismatch);
        }
        if now_micros < qualification.issued_at_micros() {
            return Err(AdapterConversionError::ReceiptNotYetValid);
        }
        if now_micros >= qualification.expires_at_micros() {
            return Err(AdapterConversionError::ReceiptExpired);
        }
        if now_micros - qualification.issued_at_micros()
            > self.profile.max_consumption_age_micros
        {
            return Err(AdapterConversionError::ReceiptTooOld);
        }
        if !ledger.is_admissible(qualification.receipt_digest()) {
            return Err(AdapterConversionError::ReceiptNotAdmissible);
        }

        Ok(Profile208CompositionCommitmentToken {
            receipt_digest: qualification.receipt_digest(),
            profile_digest: self.profile.profile_digest,
            binding: qualification.binding(),
            policy_digest: qualification.policy_digest(),
            trust_store_digest: qualification.trust_store_digest(),
            deployment_evidence: self.profile.deployment_evidence,
            issued_at_micros: qualification.issued_at_micros(),
            expires_at_micros: qualification.expires_at_micros(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qual::{
        ApproverRole, BoundApproverVerification, BoundReceiptCommitment,
        GovernedAdmissionPolicy, GovernedDispositionEvidence, GovernanceRule,
        QualificationLineageBinding, ReceiptDisposition, ReceiptOutcome, RoleSet,
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

    #[allow(clippy::too_many_arguments)]
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        let token = gate.convert(&qualification, &ledger, 200).unwrap();
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, 200).map(|_| ()),
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, 200).map(|_| ()),
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&wrong_policy, &ledger, 200).map(|_| ()),
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&wrong_trust, &ledger, 200).map(|_| ()),
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&wrong_deployment, &ledger, 200).map(|_| ()),
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, 200).map(|_| ()),
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

        let mut age_gate = Profile208CompositionCommitmentGate::new(profile(3, 50));
        assert_eq!(
            age_gate.convert(&qualification, &ledger, 200).map(|_| ()),
            Err(AdapterConversionError::ReceiptTooOld)
        );

        let mut expiry_gate = Profile208CompositionCommitmentGate::new(profile(3, 5_000));
        assert_eq!(
            expiry_gate.convert(&qualification, &ledger, 1_000).map(|_| ()),
            Err(AdapterConversionError::ReceiptExpired)
        );
        assert_eq!(
            expiry_gate.convert(&qualification, &ledger, 200).map(|_| ()),
            Err(AdapterConversionError::ClockRollback)
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
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, 200).map(|_| ()),
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
