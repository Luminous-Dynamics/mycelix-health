use crate::model;

pub use crate::model::{
    AdmissionScope, ApproverId, ApproverRole, ClaimProfileDigest, ContractDigest, ContractRef,
    ContractRefError, ContextDigest, DependencySetDigest, DeploymentEvidenceDigest,
    DispositionEvidence, DispositionEvidenceDigest, GovernancePolicyDigest, GovernanceRule,
    GovernanceRuleError, LedgerAdmissionOutcome, LedgerError, Nonce, OpaqueIdError,
    OrganizationId, QualificationLedger, QualificationLineageId, QualificationPolicy,
    QualificationStatement, ReceiptDigest, ReceiptDisposition, ReceiptKind, ReceiptOutcome,
    ReceiptStatus, RoleSet, SignerKeyId, StatementError, SubjectDigest, TrustStoreDigest,
    VerificationError, VerifiedQualification, QUALIFICATION_RECEIPT_SCHEMA_V1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundReceiptCommitment {
    receipt_digest: ReceiptDigest,
    inner: model::ReceiptCommitmentReceipt,
}

impl BoundReceiptCommitment {
    pub fn from_hash_adapter(
        receipt_digest: ReceiptDigest,
        canonical_transcript_verified: bool,
    ) -> Self {
        Self {
            receipt_digest,
            inner: model::ReceiptCommitmentReceipt::from_hash_adapter(
                receipt_digest,
                canonical_transcript_verified,
            ),
        }
    }

    pub fn receipt_digest(&self) -> ReceiptDigest {
        self.receipt_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundApproverVerification {
    receipt_digest: ReceiptDigest,
    inner: model::ApproverVerification,
}

impl BoundApproverVerification {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verifier_adapter(
        receipt_digest: ReceiptDigest,
        approver: ApproverId,
        organization: OrganizationId,
        signer_key: SignerKeyId,
        roles: RoleSet,
        attested_at_micros: i64,
        signature_verified: bool,
        trusted_at_verification: bool,
        active_at_verification: bool,
        revoked_at_verification: bool,
        compromised_at_verification: bool,
    ) -> Self {
        Self {
            receipt_digest,
            inner: model::ApproverVerification::from_verifier_adapter(
                approver,
                organization,
                signer_key,
                roles,
                attested_at_micros,
                signature_verified,
                trusted_at_verification,
                active_at_verification,
                revoked_at_verification,
                compromised_at_verification,
            ),
        }
    }

    pub fn receipt_digest(&self) -> ReceiptDigest {
        self.receipt_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GovernedVerificationError {
    AttestationReceiptMismatch,
    Verification(VerificationError),
}

impl From<VerificationError> for GovernedVerificationError {
    fn from(value: VerificationError) -> Self {
        Self::Verification(value)
    }
}

pub fn verify_governed_qualification(
    statement: QualificationStatement,
    commitment: BoundReceiptCommitment,
    policy: QualificationPolicy,
    attestations: &[BoundApproverVerification],
    now_micros: i64,
) -> Result<VerifiedQualification, GovernedVerificationError> {
    if attestations
        .iter()
        .any(|attestation| attestation.receipt_digest != commitment.receipt_digest)
    {
        return Err(GovernedVerificationError::AttestationReceiptMismatch);
    }

    let inner: Vec<model::ApproverVerification> =
        attestations.iter().map(|value| value.inner).collect();
    model::verify_qualification_bundle(statement, commitment.inner, policy, &inner, now_micros)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(value: u8) -> [u8; 32] {
        [value; 32]
    }

    macro_rules! oid {
        ($ty:ident, $value:expr) => {
            $ty::new(bytes($value)).unwrap()
        };
    }

    fn statement() -> QualificationStatement {
        QualificationStatement::new(
            ReceiptKind::P0ClosureQualification,
            ContractRef::new(oid!(ContractDigest, 1), 1).unwrap(),
            oid!(QualificationLineageId, 2),
            oid!(SubjectDigest, 3),
            oid!(DependencySetDigest, 4),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            Some(oid!(ContextDigest, 6)),
            ReceiptOutcome::Qualified,
            10,
            1_000,
            oid!(Nonce, 10),
            1,
            None,
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            AdmissionScope::ProductActivation,
            oid!(ClaimProfileDigest, 9),
        )
        .unwrap()
    }

    fn policy() -> QualificationPolicy {
        let rule = GovernanceRule::new(
            ReceiptKind::P0ClosureQualification,
            2,
            2,
            RoleSet::from_roles(&[ApproverRole::Privacy, ApproverRole::Safety]),
            500,
            2_000,
        )
        .unwrap();
        QualificationPolicy::new(
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            rule,
        )
    }

    fn attestation(receipt: ReceiptDigest, seed: u8, role: ApproverRole) -> BoundApproverVerification {
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
    }

    #[test]
    fn every_attestation_must_bind_the_exact_receipt_commitment() {
        let digest = oid!(ReceiptDigest, 40);
        let wrong = oid!(ReceiptDigest, 41);
        let attestations = [
            attestation(digest, 20, ApproverRole::Privacy),
            attestation(wrong, 30, ApproverRole::Safety),
        ];
        assert_eq!(
            verify_governed_qualification(
                statement(),
                BoundReceiptCommitment::from_hash_adapter(digest, true),
                policy(),
                &attestations,
                200,
            ),
            Err(GovernedVerificationError::AttestationReceiptMismatch)
        );
    }

    #[test]
    fn exact_bound_multi_party_bundle_emits_verified_qualification() {
        let digest = oid!(ReceiptDigest, 40);
        let attestations = [
            attestation(digest, 20, ApproverRole::Privacy),
            attestation(digest, 30, ApproverRole::Safety),
        ];
        let verified = verify_governed_qualification(
            statement(),
            BoundReceiptCommitment::from_hash_adapter(digest, true),
            policy(),
            &attestations,
            200,
        )
        .unwrap();
        assert_eq!(verified.receipt_digest(), digest);
        assert_eq!(verified.admitted_approvers().len(), 2);
    }
}
