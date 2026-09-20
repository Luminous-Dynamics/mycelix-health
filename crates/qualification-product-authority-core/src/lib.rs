#![forbid(unsafe_code)]
//! Final typed product-authority boundary for the first Patient-v2 #208 seam.
//!
//! The public conversion path accepts only two evidence artifacts:
//!
//! - [`VerifiedProfile208Match`], mirroring the exact QUAL-EVID-003/#214
//!   profile-match report; and
//! - [`VerifiedCurrentHead208`], mirroring the exact QUAL-EVID-004/#217
//!   governed current-head report.
//!
//! There is no public `verified: bool` input and no caller-selected bare
//! profile-match digest.  The lower reference crates remain private
//! dependencies used only after this layer has established exact identity
//! coherence across both artifacts and the governed qualification.

use core::fmt;
use mycelix_qualification_checkpoint_adapter_core as checkpoint;
use mycelix_qualification_receipt_core as qual;

pub use checkpoint::{
    AdapterProfileDigest, AdapterProfileDigestError, AdapterSeamId, Profile208CompositionCommitment,
    ProfileDefinitionError,
};

pub const PROFILE_MATCH_SCHEMA_V1: u16 = 1;
pub const PROFILE_MATCH_REPORT_KIND_V1: &str =
    "mycelix-health-qualification-profile-match-v1";
pub const PROFILE208_SEAM_NAME: &str = "Patient208CompositionCommitment";
pub const VERIFIED_HEAD_SCHEMA_V1: u16 = checkpoint::VERIFIED_HEAD_SCHEMA_V1;
pub const VERIFIED_HEAD_REPORT_KIND_V1: &str = checkpoint::VERIFIED_HEAD_REPORT_KIND_V1;

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

digest_type!(QualificationBundleDigest, QualificationBundleDigestError);
digest_type!(ProfileMatchDigest, ProfileMatchDigestError);
digest_type!(LineageBindingDigest, LineageBindingDigestError);
digest_type!(CheckpointDigest, CheckpointDigestError);
digest_type!(CheckpointBundleDigest, CheckpointBundleDigestError);
digest_type!(LineageStateCommitmentDigest, LineageStateCommitmentDigestError);
digest_type!(VerifiedHeadDigest, VerifiedHeadDigestError);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileMatchArtifactError {
    UnsupportedSchemaVersion,
    UnsupportedReportKind,
    UnsupportedSeam,
}

/// Exact typed identity of the QUAL-EVID-003/#214 profile-match report.
///
/// This is an adapter boundary for a report already produced by the governed
/// profile verifier.  It does not itself verify Ed25519 signatures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedProfile208Match {
    profile_digest: AdapterProfileDigest,
    qualification_bundle_digest: QualificationBundleDigest,
    receipt_digest: qual::ReceiptDigest,
    binding: qual::QualificationLineageBinding,
    policy_digest: qual::GovernancePolicyDigest,
    trust_store_digest: qual::TrustStoreDigest,
    deployment_evidence: qual::DeploymentEvidenceDigest,
    matched_at_micros: i64,
    commitment: ProfileMatchDigest,
}

impl VerifiedProfile208Match {
    #[allow(clippy::too_many_arguments)]
    pub fn from_qualified_profile_report(
        schema_version: u16,
        report_kind: &str,
        seam_id: &str,
        profile_digest: AdapterProfileDigest,
        qualification_bundle_digest: QualificationBundleDigest,
        receipt_digest: qual::ReceiptDigest,
        binding: qual::QualificationLineageBinding,
        policy_digest: qual::GovernancePolicyDigest,
        trust_store_digest: qual::TrustStoreDigest,
        deployment_evidence: qual::DeploymentEvidenceDigest,
        matched_at_micros: i64,
        commitment: ProfileMatchDigest,
    ) -> Result<Self, ProfileMatchArtifactError> {
        if schema_version != PROFILE_MATCH_SCHEMA_V1 {
            return Err(ProfileMatchArtifactError::UnsupportedSchemaVersion);
        }
        if report_kind != PROFILE_MATCH_REPORT_KIND_V1 {
            return Err(ProfileMatchArtifactError::UnsupportedReportKind);
        }
        if seam_id != PROFILE208_SEAM_NAME {
            return Err(ProfileMatchArtifactError::UnsupportedSeam);
        }
        Ok(Self {
            profile_digest,
            qualification_bundle_digest,
            receipt_digest,
            binding,
            policy_digest,
            trust_store_digest,
            deployment_evidence,
            matched_at_micros,
            commitment,
        })
    }

    pub fn profile_digest(&self) -> AdapterProfileDigest {
        self.profile_digest
    }

    pub fn qualification_bundle_digest(&self) -> QualificationBundleDigest {
        self.qualification_bundle_digest
    }

    pub fn receipt_digest(&self) -> qual::ReceiptDigest {
        self.receipt_digest
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

    pub fn matched_at_micros(&self) -> i64 {
        self.matched_at_micros
    }

    pub fn commitment(&self) -> ProfileMatchDigest {
        self.commitment
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurrentHeadArtifactError {
    Lower(checkpoint::VerifiedHeadArtifactError),
}

impl From<checkpoint::VerifiedHeadArtifactError> for CurrentHeadArtifactError {
    fn from(value: checkpoint::VerifiedHeadArtifactError) -> Self {
        Self::Lower(value)
    }
}

/// Exact typed identity emitted by QUAL-EVID-004/#217.
///
/// The lower checkpoint artifact is deliberately private.  Downstream product
/// callers receive only this wrapper and cannot feed the lower #218 seam a bare
/// profile-match digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedCurrentHead208 {
    inner: checkpoint::VerifiedQualificationHeadCheckpoint,
    lineage_binding_digest: LineageBindingDigest,
    checkpoint_digest: CheckpointDigest,
    checkpoint_bundle_digest: CheckpointBundleDigest,
    lineage_state_commitment: LineageStateCommitmentDigest,
    verified_head_digest: VerifiedHeadDigest,
}

impl VerifiedCurrentHead208 {
    #[allow(clippy::too_many_arguments)]
    pub fn from_qualified_checkpoint_report(
        schema_version: u16,
        report_kind: &str,
        binding: qual::QualificationLineageBinding,
        lineage_binding_digest: LineageBindingDigest,
        head_receipt_digest: qual::ReceiptDigest,
        head_sequence: u64,
        checkpoint_digest: CheckpointDigest,
        checkpoint_bundle_digest: CheckpointBundleDigest,
        checkpoint_epoch: u64,
        profile_match: ProfileMatchDigest,
        lineage_state_commitment: LineageStateCommitmentDigest,
        policy_digest: qual::GovernancePolicyDigest,
        trust_store_digest: qual::TrustStoreDigest,
        deployment_evidence: qual::DeploymentEvidenceDigest,
        verified_at_micros: i64,
        expires_at_micros: i64,
        verified_head_digest: VerifiedHeadDigest,
    ) -> Result<Self, CurrentHeadArtifactError> {
        let lower = checkpoint::VerifiedQualificationHeadCheckpoint::from_qualified_checkpoint_report(
            schema_version,
            report_kind,
            binding,
            checkpoint::LineageBindingDigest::new(*lineage_binding_digest.as_bytes()).unwrap(),
            head_receipt_digest,
            head_sequence,
            checkpoint::LineageCheckpointDigest::new(*checkpoint_digest.as_bytes()).unwrap(),
            checkpoint::CheckpointBundleDigest::new(*checkpoint_bundle_digest.as_bytes()).unwrap(),
            checkpoint_epoch,
            checkpoint::ProfileMatchCommitmentDigest::new(*profile_match.as_bytes()).unwrap(),
            checkpoint::LineageStateCommitmentDigest::new(*lineage_state_commitment.as_bytes()).unwrap(),
            policy_digest,
            trust_store_digest,
            deployment_evidence,
            verified_at_micros,
            expires_at_micros,
            checkpoint::VerifiedHeadDigest::new(*verified_head_digest.as_bytes()).unwrap(),
        )?;
        Ok(Self {
            inner: lower,
            lineage_binding_digest,
            checkpoint_digest,
            checkpoint_bundle_digest,
            lineage_state_commitment,
            verified_head_digest,
        })
    }

    pub fn binding(&self) -> qual::QualificationLineageBinding {
        self.inner.binding()
    }

    pub fn head_receipt_digest(&self) -> qual::ReceiptDigest {
        self.inner.head_receipt_digest()
    }

    pub fn head_sequence(&self) -> u64 {
        self.inner.head_sequence()
    }

    pub fn checkpoint_epoch(&self) -> u64 {
        self.inner.checkpoint_epoch()
    }

    pub fn profile_match_commitment(&self) -> ProfileMatchDigest {
        ProfileMatchDigest::new(*self.inner.profile_match_commitment().as_bytes()).unwrap()
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

    pub fn verified_at_micros(&self) -> i64 {
        self.inner.verified_at_micros()
    }

    pub fn expires_at_micros(&self) -> i64 {
        self.inner.expires_at_micros()
    }

    pub fn lineage_binding_digest(&self) -> LineageBindingDigest {
        self.lineage_binding_digest
    }

    pub fn checkpoint_digest(&self) -> CheckpointDigest {
        self.checkpoint_digest
    }

    pub fn checkpoint_bundle_digest(&self) -> CheckpointBundleDigest {
        self.checkpoint_bundle_digest
    }

    pub fn lineage_state_commitment(&self) -> LineageStateCommitmentDigest {
        self.lineage_state_commitment
    }

    pub fn verified_head_digest(&self) -> VerifiedHeadDigest {
        self.verified_head_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductAuthorityError {
    ClockRollback,
    ProfileDigestMismatch,
    ProfileBindingMismatch,
    ProfileReceiptMismatch,
    ProfilePolicyMismatch,
    ProfileTrustStoreMismatch,
    ProfileDeploymentMismatch,
    ProfileMatchBeforeQualification,
    ProfileMatchAfterQualificationExpiry,
    ProfileMatchInFuture,
    HeadBindingMismatch,
    HeadReceiptMismatch,
    HeadPolicyMismatch,
    HeadTrustStoreMismatch,
    HeadDeploymentMismatch,
    HeadProfileMatchMismatch,
    HeadVerifiedBeforeProfileMatch,
    Lower(checkpoint::AdapterConversionError),
}

impl From<checkpoint::AdapterConversionError> for ProductAuthorityError {
    fn from(value: checkpoint::AdapterConversionError) -> Self {
        Self::Lower(value)
    }
}

/// Final non-cloneable product token for the exact #208 composition seam.
pub struct Profile208ProductAuthorityToken {
    inner: checkpoint::Profile208CompositionCommitmentToken,
    qualification_bundle_digest: QualificationBundleDigest,
    profile_match_digest: ProfileMatchDigest,
    profile_matched_at_micros: i64,
}

impl fmt::Debug for Profile208ProductAuthorityToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile208ProductAuthorityToken")
            .field("seam", &self.inner.seam())
            .field("receipt_digest", &self.inner.receipt_digest())
            .field("profile_match_digest", &self.profile_match_digest)
            .field("checkpoint_epoch", &self.inner.checkpoint_epoch())
            .field("verified_head_digest", &self.inner.verified_head_digest())
            .finish_non_exhaustive()
    }
}

impl Profile208ProductAuthorityToken {
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

    pub fn qualification_bundle_digest(&self) -> QualificationBundleDigest {
        self.qualification_bundle_digest
    }

    pub fn profile_match_digest(&self) -> ProfileMatchDigest {
        self.profile_match_digest
    }

    pub fn profile_matched_at_micros(&self) -> i64 {
        self.profile_matched_at_micros
    }

    pub fn checkpoint_epoch(&self) -> u64 {
        self.inner.checkpoint_epoch()
    }

    pub fn checkpoint_bundle_digest(&self) -> [u8; 32] {
        *self.inner.checkpoint_bundle_digest().as_bytes()
    }

    pub fn verified_head_digest(&self) -> [u8; 32] {
        *self.inner.verified_head_digest().as_bytes()
    }

    pub fn head_expires_at_micros(&self) -> i64 {
        self.inner.head_expires_at_micros()
    }
}

/// Product-facing stateful authority gate.
///
/// Every attempted conversion observes time before artifact validation.  The
/// lower #218 gate independently observes time again, so a failed attempt cannot
/// later be replayed with an older time at either layer.
pub struct Profile208ProductAuthorityGate {
    profile: Profile208CompositionCommitment,
    inner: checkpoint::Profile208CompositionCommitmentGate,
    latest_time_micros: Option<i64>,
}

impl Profile208ProductAuthorityGate {
    pub fn new(profile: Profile208CompositionCommitment) -> Self {
        Self {
            profile,
            inner: checkpoint::Profile208CompositionCommitmentGate::new(profile),
            latest_time_micros: None,
        }
    }

    pub fn profile(&self) -> Profile208CompositionCommitment {
        self.profile
    }

    pub fn latest_time_micros(&self) -> Option<i64> {
        self.latest_time_micros
    }

    fn observe_time(&mut self, now_micros: i64) -> Result<(), ProductAuthorityError> {
        if self
            .latest_time_micros
            .is_some_and(|previous| now_micros < previous)
        {
            return Err(ProductAuthorityError::ClockRollback);
        }
        self.latest_time_micros = Some(now_micros);
        Ok(())
    }

    pub fn convert(
        &mut self,
        qualification: &qual::VerifiedQualification,
        ledger: &qual::QualificationLedger,
        profile_match: VerifiedProfile208Match,
        current_head: VerifiedCurrentHead208,
        now_micros: i64,
    ) -> Result<Profile208ProductAuthorityToken, ProductAuthorityError> {
        self.observe_time(now_micros)?;

        if profile_match.profile_digest != self.profile.profile_digest() {
            return Err(ProductAuthorityError::ProfileDigestMismatch);
        }
        if profile_match.binding != self.profile.binding()
            || profile_match.binding != qualification.binding()
        {
            return Err(ProductAuthorityError::ProfileBindingMismatch);
        }
        if profile_match.receipt_digest != qualification.receipt_digest() {
            return Err(ProductAuthorityError::ProfileReceiptMismatch);
        }
        if profile_match.policy_digest != qualification.policy_digest() {
            return Err(ProductAuthorityError::ProfilePolicyMismatch);
        }
        if profile_match.trust_store_digest != qualification.trust_store_digest() {
            return Err(ProductAuthorityError::ProfileTrustStoreMismatch);
        }
        if qualification.deployment_evidence() != Some(profile_match.deployment_evidence) {
            return Err(ProductAuthorityError::ProfileDeploymentMismatch);
        }
        if profile_match.matched_at_micros < qualification.issued_at_micros() {
            return Err(ProductAuthorityError::ProfileMatchBeforeQualification);
        }
        if profile_match.matched_at_micros >= qualification.expires_at_micros() {
            return Err(ProductAuthorityError::ProfileMatchAfterQualificationExpiry);
        }
        if profile_match.matched_at_micros > now_micros {
            return Err(ProductAuthorityError::ProfileMatchInFuture);
        }

        if current_head.binding() != profile_match.binding {
            return Err(ProductAuthorityError::HeadBindingMismatch);
        }
        if current_head.head_receipt_digest() != profile_match.receipt_digest {
            return Err(ProductAuthorityError::HeadReceiptMismatch);
        }
        if current_head.policy_digest() != profile_match.policy_digest {
            return Err(ProductAuthorityError::HeadPolicyMismatch);
        }
        if current_head.trust_store_digest() != profile_match.trust_store_digest {
            return Err(ProductAuthorityError::HeadTrustStoreMismatch);
        }
        if current_head.deployment_evidence() != profile_match.deployment_evidence {
            return Err(ProductAuthorityError::HeadDeploymentMismatch);
        }
        if current_head.profile_match_commitment() != profile_match.commitment {
            return Err(ProductAuthorityError::HeadProfileMatchMismatch);
        }
        if current_head.verified_at_micros() < profile_match.matched_at_micros {
            return Err(ProductAuthorityError::HeadVerifiedBeforeProfileMatch);
        }

        let inner = self.inner.convert(
            qualification,
            ledger,
            current_head.inner,
            checkpoint::ProfileMatchCommitmentDigest::new(*profile_match.commitment.as_bytes())
                .unwrap(),
            now_micros,
        )?;

        Ok(Profile208ProductAuthorityToken {
            inner,
            qualification_bundle_digest: profile_match.qualification_bundle_digest,
            profile_match_digest: profile_match.commitment,
            profile_matched_at_micros: profile_match.matched_at_micros,
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

    fn profile_match(
        qualification: &qual::VerifiedQualification,
        matched_at: i64,
    ) -> VerifiedProfile208Match {
        VerifiedProfile208Match::from_qualified_profile_report(
            PROFILE_MATCH_SCHEMA_V1,
            PROFILE_MATCH_REPORT_KIND_V1,
            PROFILE208_SEAM_NAME,
            AdapterProfileDigest::new(bytes(90)).unwrap(),
            QualificationBundleDigest::new(bytes(91)).unwrap(),
            qualification.receipt_digest(),
            qualification.binding(),
            qualification.policy_digest(),
            qualification.trust_store_digest(),
            qualification.deployment_evidence().unwrap(),
            matched_at,
            ProfileMatchDigest::new(bytes(94)).unwrap(),
        )
        .unwrap()
    }

    fn head(
        qualification: &qual::VerifiedQualification,
        profile_match_digest: u8,
        verified_at: i64,
        expires_at: i64,
    ) -> VerifiedCurrentHead208 {
        VerifiedCurrentHead208::from_qualified_checkpoint_report(
            VERIFIED_HEAD_SCHEMA_V1,
            VERIFIED_HEAD_REPORT_KIND_V1,
            qualification.binding(),
            LineageBindingDigest::new(bytes(92)).unwrap(),
            qualification.receipt_digest(),
            1,
            CheckpointDigest::new(bytes(93)).unwrap(),
            CheckpointBundleDigest::new(bytes(95)).unwrap(),
            1,
            ProfileMatchDigest::new(bytes(profile_match_digest)).unwrap(),
            LineageStateCommitmentDigest::new(bytes(96)).unwrap(),
            qualification.policy_digest(),
            qualification.trust_store_digest(),
            qualification.deployment_evidence().unwrap(),
            verified_at,
            expires_at,
            VerifiedHeadDigest::new(bytes(97)).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn exact_profile_and_head_emit_one_final_product_token() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let profile_match = profile_match(&qualification, 150);
        let current_head = head(&qualification, 94, 175, 900);
        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        let token = gate
            .convert(&qualification, &ledger, profile_match, current_head, 200)
            .unwrap();

        assert_eq!(token.seam(), AdapterSeamId::Patient208CompositionCommitment);
        assert_eq!(token.receipt_digest(), qualification.receipt_digest());
        assert_eq!(token.profile_match_digest(), ProfileMatchDigest::new(bytes(94)).unwrap());
        assert_eq!(token.checkpoint_epoch(), 1);
        assert_eq!(token.qualification_bundle_digest(), QualificationBundleDigest::new(bytes(91)).unwrap());
    }

    #[test]
    fn profile_artifact_must_match_exact_profile_and_qualification() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let current_head = head(&qualification, 94, 175, 900);

        let mut wrong_profile = profile_match(&qualification, 150);
        wrong_profile.profile_digest = AdapterProfileDigest::new(bytes(99)).unwrap();
        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, wrong_profile, current_head, 200)
                .map(|_| ()),
            Err(ProductAuthorityError::ProfileDigestMismatch)
        );

        let wrong_subject = verified(4, 10, 1_000);
        let wrong_binding = profile_match(&wrong_subject, 150);
        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, wrong_binding, current_head, 200)
                .map(|_| ()),
            Err(ProductAuthorityError::ProfileBindingMismatch)
        );
    }

    #[test]
    fn head_must_bind_the_exact_profile_artifact() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let profile_match = profile_match(&qualification, 150);
        let wrong_head = head(&qualification, 99, 175, 900);
        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, profile_match, wrong_head, 200)
                .map(|_| ()),
            Err(ProductAuthorityError::HeadProfileMatchMismatch)
        );
    }

    #[test]
    fn profile_match_time_is_bounded_by_qualification_and_now() {
        let qualification = verified(3, 100, 500);
        let ledger = admitted(&qualification);
        let current_head = head(&qualification, 94, 175, 450);

        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                profile_match(&qualification, 99),
                current_head,
                200,
            )
            .map(|_| ()),
            Err(ProductAuthorityError::ProfileMatchBeforeQualification)
        );

        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                profile_match(&qualification, 500),
                current_head,
                600,
            )
            .map(|_| ()),
            Err(ProductAuthorityError::ProfileMatchAfterQualificationExpiry)
        );

        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                profile_match(&qualification, 300),
                current_head,
                200,
            )
            .map(|_| ()),
            Err(ProductAuthorityError::ProfileMatchInFuture)
        );
    }

    #[test]
    fn checkpoint_cannot_pretend_to_precede_the_bound_profile_match() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let profile_match = profile_match(&qualification, 180);
        let current_head = head(&qualification, 94, 175, 900);
        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert_eq!(
            gate.convert(&qualification, &ledger, profile_match, current_head, 200)
                .map(|_| ()),
            Err(ProductAuthorityError::HeadVerifiedBeforeProfileMatch)
        );
    }

    #[test]
    fn product_clock_rollback_is_denied_even_after_failed_attempt() {
        let qualification = verified(3, 10, 1_000);
        let ledger = admitted(&qualification);
        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        let bad = profile_match(&qualification, 250);
        let current_head = head(&qualification, 94, 275, 900);
        assert_eq!(
            gate.convert(&qualification, &ledger, bad, current_head, 300)
                .map(|_| ()),
            Ok(())
        );
        assert_eq!(
            gate.convert(
                &qualification,
                &ledger,
                profile_match(&qualification, 150),
                head(&qualification, 94, 175, 900),
                200,
            )
            .map(|_| ()),
            Err(ProductAuthorityError::ClockRollback)
        );
    }

    #[test]
    fn superseded_receipt_still_fails_in_lower_independent_gate() {
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
        let mut gate = Profile208ProductAuthorityGate::new(profile(3, 500));
        assert!(matches!(
            gate.convert(
                &qualification,
                &ledger,
                profile_match(&qualification, 150),
                head(&qualification, 94, 175, 900),
                200,
            ),
            Err(ProductAuthorityError::Lower(
                checkpoint::AdapterConversionError::ReceiptNotAdmissible
            ))
        ));
    }
}
