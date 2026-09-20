use crate::model;

pub use crate::model::{
    AdmissionScope, ApproverId, ApproverRole, ClaimProfileDigest, ContractDigest, ContractRef,
    ContractRefError, ContextDigest, DependencySetDigest, DeploymentEvidenceDigest,
    DispositionEvidenceDigest, GovernancePolicyDigest, GovernanceRule, GovernanceRuleError,
    LedgerAdmissionOutcome, LedgerError, Nonce, OpaqueIdError, OrganizationId,
    QualificationLineageId, ReceiptDigest, ReceiptDisposition, ReceiptKind, ReceiptOutcome,
    ReceiptStatus, RoleSet, SignerKeyId, StatementError, SubjectDigest, TrustStoreDigest,
    VerificationError, QUALIFICATION_RECEIPT_SCHEMA_V1,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovernedStatement {
    inner: model::QualificationStatement,
    contract: ContractRef,
    admission_scope: AdmissionScope,
    claim_profile: ClaimProfileDigest,
}

impl GovernedStatement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ReceiptKind,
        contract: ContractRef,
        lineage: QualificationLineageId,
        subject: SubjectDigest,
        dependencies: DependencySetDigest,
        deployment_evidence: Option<DeploymentEvidenceDigest>,
        context: Option<ContextDigest>,
        outcome: ReceiptOutcome,
        issued_at_micros: i64,
        expires_at_micros: i64,
        nonce: Nonce,
        sequence: u64,
        previous_receipt: Option<ReceiptDigest>,
        policy: GovernancePolicyDigest,
        trust_store: TrustStoreDigest,
        admission_scope: AdmissionScope,
        claim_profile: ClaimProfileDigest,
    ) -> Result<Self, StatementError> {
        let inner = model::QualificationStatement::new(
            kind,
            contract,
            lineage,
            subject,
            dependencies,
            deployment_evidence,
            context,
            outcome,
            issued_at_micros,
            expires_at_micros,
            nonce,
            sequence,
            previous_receipt,
            policy,
            trust_store,
            admission_scope,
            claim_profile,
        )?;
        Ok(Self {
            inner,
            contract,
            admission_scope,
            claim_profile,
        })
    }

    pub fn transcript_bytes(&self) -> Vec<u8> {
        self.inner.transcript_bytes()
    }

    pub fn kind(&self) -> ReceiptKind {
        self.inner.kind()
    }

    pub fn lineage(&self) -> QualificationLineageId {
        self.inner.lineage()
    }

    pub fn sequence(&self) -> u64 {
        self.inner.sequence()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GovernedAdmissionPolicy {
    inner: model::QualificationPolicy,
    expected_contract: ContractRef,
    expected_scope: AdmissionScope,
    expected_claim_profile: ClaimProfileDigest,
}

impl GovernedAdmissionPolicy {
    pub fn new(
        policy_digest: GovernancePolicyDigest,
        trust_store_digest: TrustStoreDigest,
        expected_deployment_evidence: Option<DeploymentEvidenceDigest>,
        rule: GovernanceRule,
        expected_contract: ContractRef,
        expected_scope: AdmissionScope,
        expected_claim_profile: ClaimProfileDigest,
    ) -> Self {
        Self {
            inner: model::QualificationPolicy::new(
                policy_digest,
                trust_store_digest,
                expected_deployment_evidence,
                rule,
            ),
            expected_contract,
            expected_scope,
            expected_claim_profile,
        }
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApproverAdapterError {
    EmptyRoles,
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
    ) -> Result<Self, ApproverAdapterError> {
        if roles.is_empty() {
            return Err(ApproverAdapterError::EmptyRoles);
        }
        Ok(Self {
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
        })
    }

    pub fn receipt_digest(&self) -> ReceiptDigest {
        self.receipt_digest
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovernedQualification {
    inner: model::VerifiedQualification,
}

impl GovernedQualification {
    pub fn receipt_digest(&self) -> ReceiptDigest {
        self.inner.receipt_digest()
    }

    pub fn policy_digest(&self) -> GovernancePolicyDigest {
        self.inner.policy_digest()
    }

    pub fn trust_store_digest(&self) -> TrustStoreDigest {
        self.inner.trust_store_digest()
    }

    pub fn admitted_approvers(&self) -> &[ApproverId] {
        self.inner.admitted_approvers()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GovernedVerificationError {
    ContractMismatch,
    AdmissionScopeMismatch,
    ClaimProfileMismatch,
    AttestationReceiptMismatch,
    Verification(VerificationError),
}

impl From<VerificationError> for GovernedVerificationError {
    fn from(value: VerificationError) -> Self {
        Self::Verification(value)
    }
}

pub fn verify_governed_qualification(
    statement: GovernedStatement,
    commitment: BoundReceiptCommitment,
    policy: GovernedAdmissionPolicy,
    attestations: &[BoundApproverVerification],
    now_micros: i64,
) -> Result<GovernedQualification, GovernedVerificationError> {
    if statement.contract != policy.expected_contract {
        return Err(GovernedVerificationError::ContractMismatch);
    }
    if statement.admission_scope != policy.expected_scope {
        return Err(GovernedVerificationError::AdmissionScopeMismatch);
    }
    if statement.claim_profile != policy.expected_claim_profile {
        return Err(GovernedVerificationError::ClaimProfileMismatch);
    }
    if attestations
        .iter()
        .any(|attestation| attestation.receipt_digest != commitment.receipt_digest)
    {
        return Err(GovernedVerificationError::AttestationReceiptMismatch);
    }

    let inner: Vec<model::ApproverVerification> =
        attestations.iter().map(|value| value.inner).collect();
    let verified = model::verify_qualification_bundle(
        statement.inner,
        commitment.inner,
        policy.inner,
        &inner,
        now_micros,
    )?;
    Ok(GovernedQualification { inner: verified })
}

#[derive(Clone, Debug)]
pub struct GovernedQualificationLedger {
    inner: model::QualificationLedger,
}

impl GovernedQualificationLedger {
    pub fn new(lineage: QualificationLineageId) -> Self {
        Self {
            inner: model::QualificationLedger::new(lineage),
        }
    }

    pub fn admit(
        &mut self,
        qualification: &GovernedQualification,
        now_micros: i64,
    ) -> Result<LedgerAdmissionOutcome, LedgerError> {
        self.inner.admit(&qualification.inner, now_micros)
    }

    pub fn apply_disposition(
        &mut self,
        evidence: GovernedDispositionEvidence,
    ) -> Result<LedgerAdmissionOutcome, LedgerError> {
        self.inner.apply_disposition(evidence.inner)
    }

    pub fn status(&self, digest: ReceiptDigest) -> Option<ReceiptStatus> {
        self.inner.status(digest)
    }

    pub fn is_admissible(&self, digest: ReceiptDigest) -> bool {
        self.inner.is_admissible(digest)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GovernedDispositionEvidence {
    inner: model::DispositionEvidence,
}

impl GovernedDispositionEvidence {
    pub fn from_governance_adapter(
        lineage: QualificationLineageId,
        target: ReceiptDigest,
        disposition: ReceiptDisposition,
        governance_evidence: DispositionEvidenceDigest,
        disposition_epoch: u64,
        verified: bool,
    ) -> Self {
        Self {
            inner: model::DispositionEvidence::from_governance_adapter(
                lineage,
                target,
                disposition,
                governance_evidence,
                disposition_epoch,
                verified,
            ),
        }
    }
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

    fn contract() -> ContractRef {
        ContractRef::new(oid!(ContractDigest, 1), 1).unwrap()
    }

    fn statement(
        sequence: u64,
        previous: Option<ReceiptDigest>,
        nonce_value: u8,
    ) -> GovernedStatement {
        GovernedStatement::new(
            ReceiptKind::P0ClosureQualification,
            contract(),
            oid!(QualificationLineageId, 2),
            oid!(SubjectDigest, 3),
            oid!(DependencySetDigest, 4),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            Some(oid!(ContextDigest, 6)),
            ReceiptOutcome::Qualified,
            10,
            1_000,
            oid!(Nonce, nonce_value),
            sequence,
            previous,
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            AdmissionScope::ProductActivation,
            oid!(ClaimProfileDigest, 9),
        )
        .unwrap()
    }

    fn policy() -> GovernedAdmissionPolicy {
        let rule = GovernanceRule::new(
            ReceiptKind::P0ClosureQualification,
            2,
            2,
            RoleSet::from_roles(&[ApproverRole::Privacy, ApproverRole::Safety]),
            500,
            2_000,
        )
        .unwrap();
        GovernedAdmissionPolicy::new(
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            rule,
            contract(),
            AdmissionScope::ProductActivation,
            oid!(ClaimProfileDigest, 9),
        )
    }

    fn attestation(
        receipt: ReceiptDigest,
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
        digest_value: u8,
        sequence: u64,
        previous: Option<ReceiptDigest>,
        nonce_value: u8,
    ) -> GovernedQualification {
        let digest = oid!(ReceiptDigest, digest_value);
        verify_governed_qualification(
            statement(sequence, previous, nonce_value),
            BoundReceiptCommitment::from_hash_adapter(digest, true),
            policy(),
            &[
                attestation(digest, 20, ApproverRole::Privacy),
                attestation(digest, 30, ApproverRole::Safety),
            ],
            200,
        )
        .unwrap()
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
                statement(1, None, 10),
                BoundReceiptCommitment::from_hash_adapter(digest, true),
                policy(),
                &attestations,
                200,
            ),
            Err(GovernedVerificationError::AttestationReceiptMismatch)
        );
    }

    #[test]
    fn empty_approver_roles_are_rejected_at_the_public_adapter_boundary() {
        assert_eq!(
            BoundApproverVerification::from_verifier_adapter(
                oid!(ReceiptDigest, 40),
                oid!(ApproverId, 20),
                oid!(OrganizationId, 21),
                oid!(SignerKeyId, 22),
                RoleSet::EMPTY,
                100,
                true,
                true,
                true,
                false,
                false,
            ),
            Err(ApproverAdapterError::EmptyRoles)
        );
    }

    #[test]
    fn policy_binds_contract_scope_and_claim_profile() {
        let digest = oid!(ReceiptDigest, 40);
        let attestations = [
            attestation(digest, 20, ApproverRole::Privacy),
            attestation(digest, 30, ApproverRole::Safety),
        ];
        let rule = GovernanceRule::new(
            ReceiptKind::P0ClosureQualification,
            2,
            2,
            RoleSet::from_roles(&[ApproverRole::Privacy, ApproverRole::Safety]),
            500,
            2_000,
        )
        .unwrap();
        let wrong_scope = GovernedAdmissionPolicy::new(
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            rule,
            contract(),
            AdmissionScope::Reference,
            oid!(ClaimProfileDigest, 9),
        );
        assert_eq!(
            verify_governed_qualification(
                statement(1, None, 10),
                BoundReceiptCommitment::from_hash_adapter(digest, true),
                wrong_scope,
                &attestations,
                200,
            ),
            Err(GovernedVerificationError::AdmissionScopeMismatch)
        );
    }

    #[test]
    fn exact_bound_multi_party_bundle_emits_only_governed_token() {
        let digest = oid!(ReceiptDigest, 40);
        let verified = verify_governed_qualification(
            statement(1, None, 10),
            BoundReceiptCommitment::from_hash_adapter(digest, true),
            policy(),
            &[
                attestation(digest, 20, ApproverRole::Privacy),
                attestation(digest, 30, ApproverRole::Safety),
            ],
            200,
        )
        .unwrap();
        assert_eq!(verified.receipt_digest(), digest);
        assert_eq!(verified.admitted_approvers().len(), 2);
    }

    #[test]
    fn governed_ledger_preserves_nonce_sequence_and_disposition_semantics() {
        let first = verified(40, 1, None, 10);
        let second = verified(41, 2, Some(first.receipt_digest()), 11);
        let mut ledger = GovernedQualificationLedger::new(oid!(QualificationLineageId, 2));
        assert_eq!(ledger.admit(&first, 200), Ok(LedgerAdmissionOutcome::Added));
        assert_eq!(ledger.admit(&second, 201), Ok(LedgerAdmissionOutcome::Added));

        let disposition = GovernedDispositionEvidence::from_governance_adapter(
            oid!(QualificationLineageId, 2),
            first.receipt_digest(),
            ReceiptDisposition::Supersede,
            oid!(DispositionEvidenceDigest, 60),
            1,
            true,
        );
        assert_eq!(
            ledger.apply_disposition(disposition),
            Ok(LedgerAdmissionOutcome::Added)
        );
        assert_eq!(
            ledger.status(first.receipt_digest()),
            Some(ReceiptStatus::Superseded)
        );
        assert!(!ledger.is_admissible(first.receipt_digest()));
        assert!(ledger.is_admissible(second.receipt_digest()));
    }
}
