#![forbid(unsafe_code)]
//! Product-facing qualification adapter semantics for sensitive authority seams.
//!
//! QUAL-EVID-006 closes the last raw current-head boolean in the #208
//! composition-commitment path.  The original QUAL-EVID-002 reference remains
//! compiled as a private module so its receipt/profile/ledger checks are reused,
//! but its `VerifiedLineageHead::from_checkpoint_adapter(..., verified: bool)`
//! constructor is not part of this crate's public API.
//!
//! This layer does not independently verify #217 checkpoint signatures.  It
//! consumes the exact typed artifact emitted by that independently qualified
//! checkpoint verifier and proves that the artifact matches the governed
//! qualification, adapter profile, local qualification ledger, expected #214
//! profile-match commitment, and current consumption time.

mod legacy {
    include!("lib.rs");
}

use core::fmt;
use mycelix_qualification_receipt_core as qual;

pub use legacy::{
    AdapterProfileDigest, AdapterProfileDigestError, AdapterSeamId, LineageCheckpointDigest,
    LineageCheckpointDigestError, Profile208CompositionCommitment, ProfileDefinitionError,
};

pub const VERIFIED_HEAD_SCHEMA_V1: u16 = 1;
pub const VERIFIED_HEAD_REPORT_KIND_V1: &str =
    "mycelix-health-verified-qualification-head-checkpoint-v1";

macro_rules! digest_type {
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

digest_type!(LineageBindingDigest, LineageBindingDigestError);
digest_type!(CheckpointBundleDigest, CheckpointBundleDigestError);
digest_type!(ProfileMatchCommitmentDigest, ProfileMatchCommitmentDigestError);
digest_type!(LineageStateCommitmentDigest, LineageStateCommitmentDigestError);
digest_type!(VerifiedHeadDigest, VerifiedHeadDigestError);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifiedHeadArtifactError {
    UnsupportedSchemaVersion,
    UnsupportedReportKind,
    InvalidHeadSequence,
    InvalidCheckpointEpoch,
    InvalidVerificationWindow,
}

/// Exact typed identity emitted by QUAL-EVID-004/#217 `emit_verified_head()`.
///
/// This type contains no caller-controlled `verified` bit.  A production parser
/// should construct it only after #217 has independently verified/admitted the
/// governed signed checkpoint and emitted the strict head report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedQualificationHeadCheckpoint {
    binding: qual::QualificationLineageBinding,
    lineage_binding_digest: LineageBindingDigest,
    head_receipt_digest: qual::ReceiptDigest,
    head_sequence: u64,
    checkpoint_digest: LineageCheckpointDigest,
    checkpoint_bundle_digest: CheckpointBundleDigest,
    checkpoint_epoch: u64,
    profile_match_commitment: ProfileMatchCommitmentDigest,
    lineage_state_commitment: LineageStateCommitmentDigest,
    policy_digest: qual::GovernancePolicyDigest,
    trust_store_digest: qual::TrustStoreDigest,
    deployment_evidence: qual::DeploymentEvidenceDigest,
    verified_at_micros: i64,
    expires_at_micros: i64,
    verified_head_digest: VerifiedHeadDigest,
}

impl VerifiedQualificationHeadCheckpoint {
    #[allow(clippy::too_many_arguments)]
    pub fn from_qualified_checkpoint_report(
        schema_version: u16,
        report_kind: &str,
        binding: qual::QualificationLineageBinding,
        lineage_binding_digest: LineageBindingDigest,
        head_receipt_digest: qual::ReceiptDigest,
        head_sequence: u64,
        checkpoint_digest: LineageCheckpointDigest,
        checkpoint_bundle_digest: CheckpointBundleDigest,
        checkpoint_epoch: u64,
        profile_match_commitment: ProfileMatchCommitmentDigest,
        lineage_state_commitment: LineageStateCommitmentDigest,
        policy_digest: qual::GovernancePolicyDigest,
        trust_store_digest: qual::TrustStoreDigest,
        deployment_evidence: qual::DeploymentEvidenceDigest,
        verified_at_micros: i64,
        expires_at_micros: i64,
        verified_head_digest: VerifiedHeadDigest,
    ) -> Result<Self, VerifiedHeadArtifactError> {
        if schema_version != VERIFIED_HEAD_SCHEMA_V1 {
            return Err(VerifiedHeadArtifactError::UnsupportedSchemaVersion);
        }
        if report_kind != VERIFIED_HEAD_REPORT_KIND_V1 {
            return Err(VerifiedHeadArtifactError::UnsupportedReportKind);
        }
        if head_sequence == 0 {
            return Err(VerifiedHeadArtifactError::InvalidHeadSequence);
        }
        if checkpoint_epoch == 0 {
            return Err(VerifiedHeadArtifactError::InvalidCheckpointEpoch);
        }
        if expires_at_micros <= verified_at_micros {
            return Err(VerifiedHeadArtifactError::InvalidVerificationWindow);
        }
        Ok(Self {
            binding,
            lineage_binding_digest,
            head_receipt_digest,
            head_sequence,
            checkpoint_digest,
            checkpoint_bundle_digest,
            checkpoint_epoch,
            profile_match_commitment,
            lineage_state_commitment,
            policy_digest,
            trust_store_digest,
            deployment_evidence,
            verified_at_micros,
            expires_at_micros,
            verified_head_digest,
        })
    }

    pub fn binding(&self) -> qual::QualificationLineageBinding {
        self.binding
    }

    pub fn lineage_binding_digest(&self) -> LineageBindingDigest {
        self.lineage_binding_digest
    }

    pub fn head_receipt_digest(&self) -> qual::ReceiptDigest {
        self.head_receipt_digest
    }

    pub fn head_sequence(&self) -> u64 {
        self.head_sequence
    }

    pub fn checkpoint_digest(&self) -> LineageCheckpointDigest {
        self.checkpoint_digest
    }

    pub fn checkpoint_bundle_digest(&self) -> CheckpointBundleDigest {
        self.checkpoint_bundle_digest
    }

    pub fn checkpoint_epoch(&self) -> u64 {
        self.checkpoint_epoch
    }

    pub fn profile_match_commitment(&self) -> ProfileMatchCommitmentDigest {
        self.profile_match_commitment
    }

    pub fn lineage_state_commitment(&self) -> LineageStateCommitmentDigest {
        self.lineage_state_commitment
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

    pub fn verified_at_micros(&self) -> i64 {
        self.verified_at_micros
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.expires_at_micros
    }

    pub fn verified_head_digest(&self) -> VerifiedHeadDigest {
        self.verified_head_digest
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
    HeadBindingMismatch,
    HeadPolicyMismatch,
    HeadTrustStoreMismatch,
    HeadDeploymentEvidenceMismatch,
    ReceiptNotCurrentHead,
    ProfileMatchCommitmentMismatch,
    HeadNotYetVerified,
    HeadExpired,
    ReceiptNotAdmissible,
    ReceiptNotYetValid,
    ReceiptExpired,
    ReceiptAgeOverflow,
    ReceiptTooOld,
    LegacyInvariant,
}

impl From<legacy::AdapterConversionError> for AdapterConversionError {
    fn from(value: legacy::AdapterConversionError) -> Self {
        match value {
            legacy::AdapterConversionError::ClockRollback => Self::ClockRollback,
            legacy::AdapterConversionError::WrongReceiptKind => Self::WrongReceiptKind,
            legacy::AdapterConversionError::LineageBindingMismatch => Self::LineageBindingMismatch,
            legacy::AdapterConversionError::LedgerBindingMismatch => Self::LedgerBindingMismatch,
            legacy::AdapterConversionError::PolicyMismatch => Self::PolicyMismatch,
            legacy::AdapterConversionError::TrustStoreMismatch => Self::TrustStoreMismatch,
            legacy::AdapterConversionError::DeploymentEvidenceMismatch => {
                Self::DeploymentEvidenceMismatch
            }
            legacy::AdapterConversionError::HeadBindingMismatch => Self::HeadBindingMismatch,
            legacy::AdapterConversionError::ReceiptNotCurrentHead => Self::ReceiptNotCurrentHead,
            legacy::AdapterConversionError::ReceiptNotAdmissible => Self::ReceiptNotAdmissible,
            legacy::AdapterConversionError::ReceiptNotYetValid => Self::ReceiptNotYetValid,
            legacy::AdapterConversionError::ReceiptExpired => Self::ReceiptExpired,
            legacy::AdapterConversionError::ReceiptAgeOverflow => Self::ReceiptAgeOverflow,
            legacy::AdapterConversionError::ReceiptTooOld => Self::ReceiptTooOld,
            legacy::AdapterConversionError::HeadNotVerified => Self::LegacyInvariant,
        }
    }
}

/// Non-cloneable exact #208 authority token with #217 checkpoint provenance.
pub struct Profile208CompositionCommitmentToken {
    inner: legacy::Profile208CompositionCommitmentToken,
    lineage_binding_digest: LineageBindingDigest,
    head_sequence: u64,
    checkpoint_bundle_digest: CheckpointBundleDigest,
    checkpoint_epoch: u64,
    profile_match_commitment: ProfileMatchCommitmentDigest,
    lineage_state_commitment: LineageStateCommitmentDigest,
    verified_head_digest: VerifiedHeadDigest,
    head_verified_at_micros: i64,
    head_expires_at_micros: i64,
}

impl fmt::Debug for Profile208CompositionCommitmentToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile208CompositionCommitmentToken")
            .field("seam", &self.seam())
            .field("receipt_digest", &self.receipt_digest())
            .field("profile_digest", &self.profile_digest())
            .field("binding", &self.binding())
            .field("checkpoint_digest", &self.checkpoint_digest())
            .field("checkpoint_epoch", &self.checkpoint_epoch)
            .field("head_sequence", &self.head_sequence)
            .field("head_verified_at_micros", &self.head_verified_at_micros)
            .field("head_expires_at_micros", &self.head_expires_at_micros)
            .finish_non_exhaustive()
    }
}

impl Profile208CompositionCommitmentToken {
    pub fn seam(&self) -> AdapterSeamId {
        self.inner.seam()
    }

    pub fn receipt_digest(&self) -> qual::ReceiptDigest {
        self.inner.receipt_digest()
    }

    pub fn profile_digest(&self) -> AdapterProfileDigest {
        self.inner.profile_digest()
    }

    pub fn binding(&self) -> qual::QualificationLineageBinding {
        self.inner.binding()
    }

    pub fn policy_digest(&self) -> qual::GovernancePolicyDigest {
        self.inner.policy_digest()
    }

    pub fn trust_store_digest(&self) -> qual::TrustStoreDigest {
        self.inner.trust_store_digest()
    }

    pub fn deployment_evidence(&self) -> qual::DeploymentEvidenceDigest {
        self.inner.deployment_evidence()
    }

    pub fn checkpoint_digest(&self) -> LineageCheckpointDigest {
        self.inner.lineage_checkpoint()
    }

    pub fn lineage_binding_digest(&self) -> LineageBindingDigest {
        self.lineage_binding_digest
    }

    pub fn head_sequence(&self) -> u64 {
        self.head_sequence
    }

    pub fn checkpoint_bundle_digest(&self) -> CheckpointBundleDigest {
        self.checkpoint_bundle_digest
    }

    pub fn checkpoint_epoch(&self) -> u64 {
        self.checkpoint_epoch
    }

    pub fn profile_match_commitment(&self) -> ProfileMatchCommitmentDigest {
        self.profile_match_commitment
    }

    pub fn lineage_state_commitment(&self) -> LineageStateCommitmentDigest {
        self.lineage_state_commitment
    }

    pub fn verified_head_digest(&self) -> VerifiedHeadDigest {
        self.verified_head_digest
    }

    pub fn head_verified_at_micros(&self) -> i64 {
        self.head_verified_at_micros
    }

    pub fn head_expires_at_micros(&self) -> i64 {
        self.head_expires_at_micros
    }

    pub fn issued_at_micros(&self) -> i64 {
        self.inner.issued_at_micros()
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.inner.expires_at_micros()
    }
}

/// Stateful exact #208 product-purpose consumption boundary.
///
/// Every attempt observes time before any semantic check.  This preserves the
/// original #212 anti-time-rollback theorem while adding exact #217 head checks.
pub struct Profile208CompositionCommitmentGate {
    inner: legacy::Profile208CompositionCommitmentGate,
    latest_time_micros: Option<i64>,
}

impl Profile208CompositionCommitmentGate {
    pub fn new(profile: Profile208CompositionCommitment) -> Self {
        Self {
            inner: legacy::Profile208CompositionCommitmentGate::new(profile),
            latest_time_micros: None,
        }
    }

    pub fn profile(&self) -> Profile208CompositionCommitment {
        self.inner.profile()
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

    pub fn convert(
        &mut self,
        qualification: &qual::VerifiedQualification,
        ledger: &qual::QualificationLedger,
        current_head: VerifiedQualificationHeadCheckpoint,
        expected_profile_match: ProfileMatchCommitmentDigest,
        now_micros: i64,
    ) -> Result<Profile208CompositionCommitmentToken, AdapterConversionError> {
        self.observe_time(now_micros)?;

        if qualification.kind() != qual::ReceiptKind::CompositionCommitmentQualification {
            return Err(AdapterConversionError::WrongReceiptKind);
        }
        if current_head.binding != qualification.binding() {
            return Err(AdapterConversionError::HeadBindingMismatch);
        }
        if current_head.policy_digest != qualification.policy_digest() {
            return Err(AdapterConversionError::HeadPolicyMismatch);
        }
        if current_head.trust_store_digest != qualification.trust_store_digest() {
            return Err(AdapterConversionError::HeadTrustStoreMismatch);
        }
        if qualification.deployment_evidence() != Some(current_head.deployment_evidence) {
            return Err(AdapterConversionError::HeadDeploymentEvidenceMismatch);
        }
        if current_head.head_receipt_digest != qualification.receipt_digest() {
            return Err(AdapterConversionError::ReceiptNotCurrentHead);
        }
        if current_head.profile_match_commitment != expected_profile_match {
            return Err(AdapterConversionError::ProfileMatchCommitmentMismatch);
        }
        if now_micros < current_head.verified_at_micros {
            return Err(AdapterConversionError::HeadNotYetVerified);
        }
        if now_micros >= current_head.expires_at_micros {
            return Err(AdapterConversionError::HeadExpired);
        }

        // The legacy QUAL-EVID-002 gate remains private.  Its boolean adapter is
        // invoked only after this stronger QUAL-EVID-006 artifact has passed all
        // exact #217 cross-checks, so no external caller can mint current-head
        // authority from a naked boolean.
        let legacy_head = legacy::VerifiedLineageHead::from_checkpoint_adapter(
            current_head.binding,
            current_head.head_receipt_digest,
            current_head.checkpoint_digest,
            true,
        );
        let inner = self
            .inner
            .convert(qualification, ledger, legacy_head, now_micros)
            .map_err(AdapterConversionError::from)?;

        Ok(Profile208CompositionCommitmentToken {
            inner,
            lineage_binding_digest: current_head.lineage_binding_digest,
            head_sequence: current_head.head_sequence,
            checkpoint_bundle_digest: current_head.checkpoint_bundle_digest,
            checkpoint_epoch: current_head.checkpoint_epoch,
            profile_match_commitment: current_head.profile_match_commitment,
            lineage_state_commitment: current_head.lineage_state_commitment,
            verified_head_digest: current_head.verified_head_digest,
            head_verified_at_micros: current_head.verified_at_micros,
            head_expires_at_micros: current_head.expires_at_micros,
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

    fn verified(subject: u8, issued_at: i64, expires_at: i64) -> qual::VerifiedQualification {
        let bind = binding(subject);
        let kind = qual::ReceiptKind::CompositionCommitmentQualification;
        let policy_digest = oid!(GovernancePolicyDigest, 7);
        let trust_store = oid!(TrustStoreDigest, 8);
        let deployment = oid!(DeploymentEvidenceDigest, 5);
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
        let receipt = oid!(ReceiptDigest, 40);
        let statement = qual::ScopedQualificationStatement::new(
            kind,
            bind,
            oid!(DependencySetDigest, 4),
            Some(deployment),
            ReceiptOutcome::Qualified,
            issued_at,
            expires_at,
            oid!(Nonce, 60),
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

    fn head(
        qualification: &qual::VerifiedQualification,
        profile_match: u8,
        verified_at: i64,
        expires_at: i64,
    ) -> VerifiedQualificationHeadCheckpoint {
        VerifiedQualificationHeadCheckpoint::from_qualified_checkpoint_report(
            VERIFIED_HEAD_SCHEMA_V1,
            VERIFIED_HEAD_REPORT_KIND_V1,
            qualification.binding(),
            LineageBindingDigest::new(bytes(91)).unwrap(),
            qualification.receipt_digest(),
            1,
            LineageCheckpointDigest::new(bytes(92)).unwrap(),
            CheckpointBundleDigest::new(bytes(93)).unwrap(),
            1,
            ProfileMatchCommitmentDigest::new(bytes(profile_match)).unwrap(),
            LineageStateCommitmentDigest::new(bytes(95)).unwrap(),
            qualification.policy_digest(),
            qualification.trust_store_digest(),
            qualification.deployment_evidence().unwrap(),
            verified_at,
            expires_at,
            VerifiedHeadDigest::new(bytes(96)).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn exact_qualified_head_emits_one_provenance_rich_token() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let expected = ProfileMatchCommitmentDigest::new(bytes(94)).unwrap();
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        let token = gate
            .convert(
                &qualification,
                &ledger,
                head(&qualification, 94, 150, 900),
                expected,
                200,
            )
            .unwrap();

        assert_eq!(token.seam(), AdapterSeamId::Patient208CompositionCommitment);
        assert_eq!(token.receipt_digest(), qualification.receipt_digest());
        assert_eq!(token.checkpoint_epoch(), 1);
        assert_eq!(token.head_sequence(), 1);
        assert_eq!(token.profile_match_commitment(), expected);
        assert_eq!(token.checkpoint_bundle_digest(), CheckpointBundleDigest::new(bytes(93)).unwrap());
        assert_eq!(token.verified_head_digest(), VerifiedHeadDigest::new(bytes(96)).unwrap());
    }

    #[test]
    fn wrong_profile_match_is_rejected() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                head(&qualification, 94, 150, 900),
                ProfileMatchCommitmentDigest::new(bytes(99)).unwrap(),
                200,
            )
            .map(|_| ()),
            Err(AdapterConversionError::ProfileMatchCommitmentMismatch)
        );
    }

    #[test]
    fn head_policy_trust_and_deployment_are_rechecked() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let expected = ProfileMatchCommitmentDigest::new(bytes(94)).unwrap();

        let mut wrong_policy = head(&qualification, 94, 150, 900);
        wrong_policy.policy_digest = oid!(GovernancePolicyDigest, 70);
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, wrong_policy, expected, 200)
                .map(|_| ()),
            Err(AdapterConversionError::HeadPolicyMismatch)
        );

        let mut wrong_trust = head(&qualification, 94, 150, 900);
        wrong_trust.trust_store_digest = oid!(TrustStoreDigest, 80);
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, wrong_trust, expected, 200)
                .map(|_| ()),
            Err(AdapterConversionError::HeadTrustStoreMismatch)
        );

        let mut wrong_deployment = head(&qualification, 94, 150, 900);
        wrong_deployment.deployment_evidence = oid!(DeploymentEvidenceDigest, 50);
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, wrong_deployment, expected, 200)
                .map(|_| ()),
            Err(AdapterConversionError::HeadDeploymentEvidenceMismatch)
        );
    }

    #[test]
    fn head_time_window_and_gate_clock_are_fail_closed() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let expected = ProfileMatchCommitmentDigest::new(bytes(94)).unwrap();

        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                head(&qualification, 94, 300, 900),
                expected,
                200,
            )
            .map(|_| ()),
            Err(AdapterConversionError::HeadNotYetVerified)
        );
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                head(&qualification, 94, 150, 900),
                expected,
                100,
            )
            .map(|_| ()),
            Err(AdapterConversionError::ClockRollback)
        );

        let mut expired_gate = Profile208CompositionCommitmentGate::new(profile(3, 5_000));
        assert_eq!(
            expired_gate
                .convert(
                    &qualification,
                    &ledger,
                    head(&qualification, 94, 150, 200),
                    expected,
                    200,
                )
                .map(|_| ()),
            Err(AdapterConversionError::HeadExpired)
        );
    }

    #[test]
    fn invalid_checkpoint_report_identity_is_rejected_before_conversion() {
        let qualification = verified(3, 10, 1_000);
        let build = |schema, kind, sequence, epoch| {
            VerifiedQualificationHeadCheckpoint::from_qualified_checkpoint_report(
                schema,
                kind,
                qualification.binding(),
                LineageBindingDigest::new(bytes(91)).unwrap(),
                qualification.receipt_digest(),
                sequence,
                LineageCheckpointDigest::new(bytes(92)).unwrap(),
                CheckpointBundleDigest::new(bytes(93)).unwrap(),
                epoch,
                ProfileMatchCommitmentDigest::new(bytes(94)).unwrap(),
                LineageStateCommitmentDigest::new(bytes(95)).unwrap(),
                qualification.policy_digest(),
                qualification.trust_store_digest(),
                qualification.deployment_evidence().unwrap(),
                150,
                900,
                VerifiedHeadDigest::new(bytes(96)).unwrap(),
            )
        };
        assert_eq!(
            build(2, VERIFIED_HEAD_REPORT_KIND_V1, 1, 1),
            Err(VerifiedHeadArtifactError::UnsupportedSchemaVersion)
        );
        assert_eq!(
            build(1, "other-head", 1, 1),
            Err(VerifiedHeadArtifactError::UnsupportedReportKind)
        );
        assert_eq!(
            build(1, VERIFIED_HEAD_REPORT_KIND_V1, 0, 1),
            Err(VerifiedHeadArtifactError::InvalidHeadSequence)
        );
        assert_eq!(
            build(1, VERIFIED_HEAD_REPORT_KIND_V1, 1, 0),
            Err(VerifiedHeadArtifactError::InvalidCheckpointEpoch)
        );
    }

    #[test]
    fn superseded_local_receipt_remains_denied() {
        let qualification = verified(3, 10, 1_000);
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
        let expected = ProfileMatchCommitmentDigest::new(bytes(94)).unwrap();
        let mut gate = Profile208CompositionCommitmentGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                head(&qualification, 94, 150, 900),
                expected,
                200,
            )
            .map(|_| ()),
            Err(AdapterConversionError::ReceiptNotAdmissible)
        );
    }

    #[test]
    fn legacy_boolean_head_constructor_is_not_reexported() {
        // This test is intentionally behavioral: the public integrated root has
        // no `VerifiedLineageHead` type at all.  The only path exercised here is
        // the exact #217 artifact conversion above.  The CI workflow also guards
        // the integrated root/Cargo path against future re-export drift.
        assert_eq!(VERIFIED_HEAD_SCHEMA_V1, 1);
        assert_eq!(
            VERIFIED_HEAD_REPORT_KIND_V1,
            "mycelix-health-verified-qualification-head-checkpoint-v1"
        );
    }
}
