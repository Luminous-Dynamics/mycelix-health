use crate::strict;

pub use crate::strict::{
    AdmissionScope, ApproverAdapterError, ApproverId, ApproverRole, BoundApproverVerification,
    BoundReceiptCommitment, ClaimProfileDigest, ContractDigest, ContractRef, ContractRefError,
    ContextDigest, DependencySetDigest, DeploymentEvidenceDigest, DispositionEvidenceDigest,
    GovernancePolicyDigest, GovernanceRule, GovernanceRuleError, GovernedAdmissionPolicy,
    GovernedDispositionEvidence, GovernedVerificationError, LedgerAdmissionOutcome, LedgerError,
    Nonce, OpaqueIdError, OrganizationId, QualificationLineageId, ReceiptDigest,
    ReceiptDisposition, ReceiptKind, ReceiptOutcome, ReceiptStatus, RoleSet, SignerKeyId,
    StatementError, SubjectDigest, TrustStoreDigest, VerificationError,
    QUALIFICATION_RECEIPT_SCHEMA_V1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationLineageBinding {
    lineage: QualificationLineageId,
    contract: ContractRef,
    subject: SubjectDigest,
    context: Option<ContextDigest>,
    admission_scope: AdmissionScope,
    claim_profile: ClaimProfileDigest,
}

impl QualificationLineageBinding {
    pub fn new(
        lineage: QualificationLineageId,
        contract: ContractRef,
        subject: SubjectDigest,
        context: Option<ContextDigest>,
        admission_scope: AdmissionScope,
        claim_profile: ClaimProfileDigest,
    ) -> Self {
        Self {
            lineage,
            contract,
            subject,
            context,
            admission_scope,
            claim_profile,
        }
    }

    pub fn lineage(&self) -> QualificationLineageId {
        self.lineage
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedQualificationStatement {
    inner: strict::GovernedStatement,
    binding: QualificationLineageBinding,
}

impl ScopedQualificationStatement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ReceiptKind,
        binding: QualificationLineageBinding,
        dependencies: DependencySetDigest,
        deployment_evidence: Option<DeploymentEvidenceDigest>,
        outcome: ReceiptOutcome,
        issued_at_micros: i64,
        expires_at_micros: i64,
        nonce: Nonce,
        sequence: u64,
        previous_receipt: Option<ReceiptDigest>,
        policy: GovernancePolicyDigest,
        trust_store: TrustStoreDigest,
    ) -> Result<Self, StatementError> {
        let inner = strict::GovernedStatement::new(
            kind,
            binding.contract,
            binding.lineage,
            binding.subject,
            dependencies,
            deployment_evidence,
            binding.context,
            outcome,
            issued_at_micros,
            expires_at_micros,
            nonce,
            sequence,
            previous_receipt,
            policy,
            trust_store,
            binding.admission_scope,
            binding.claim_profile,
        )?;
        Ok(Self { inner, binding })
    }

    pub fn transcript_bytes(&self) -> Vec<u8> {
        self.inner.transcript_bytes()
    }

    pub fn binding(&self) -> QualificationLineageBinding {
        self.binding
    }

    pub fn sequence(&self) -> u64 {
        self.inner.sequence()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedQualification {
    inner: strict::GovernedQualification,
    binding: QualificationLineageBinding,
}

impl VerifiedQualification {
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

    pub fn binding(&self) -> QualificationLineageBinding {
        self.binding
    }
}

pub fn verify_qualification(
    statement: ScopedQualificationStatement,
    commitment: BoundReceiptCommitment,
    policy: GovernedAdmissionPolicy,
    attestations: &[BoundApproverVerification],
    now_micros: i64,
) -> Result<VerifiedQualification, GovernedVerificationError> {
    let binding = statement.binding;
    let inner = strict::verify_governed_qualification(
        statement.inner,
        commitment,
        policy,
        attestations,
        now_micros,
    )?;
    Ok(VerifiedQualification { inner, binding })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopedLedgerError {
    LineageScopeMismatch,
    Ledger(LedgerError),
}

impl From<LedgerError> for ScopedLedgerError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

#[derive(Clone, Debug)]
pub struct QualificationLedger {
    binding: QualificationLineageBinding,
    inner: strict::GovernedQualificationLedger,
}

impl QualificationLedger {
    pub fn new(binding: QualificationLineageBinding) -> Self {
        Self {
            inner: strict::GovernedQualificationLedger::new(binding.lineage),
            binding,
        }
    }

    pub fn binding(&self) -> QualificationLineageBinding {
        self.binding
    }

    pub fn admit(
        &mut self,
        qualification: &VerifiedQualification,
        now_micros: i64,
    ) -> Result<LedgerAdmissionOutcome, ScopedLedgerError> {
        if qualification.binding != self.binding {
            return Err(ScopedLedgerError::LineageScopeMismatch);
        }
        self.inner
            .admit(&qualification.inner, now_micros)
            .map_err(Into::into)
    }

    pub fn apply_disposition(
        &mut self,
        evidence: GovernedDispositionEvidence,
    ) -> Result<LedgerAdmissionOutcome, ScopedLedgerError> {
        self.inner.apply_disposition(evidence).map_err(Into::into)
    }

    pub fn status(&self, digest: ReceiptDigest) -> Option<ReceiptStatus> {
        self.inner.status(digest)
    }

    pub fn is_admissible(&self, digest: ReceiptDigest) -> bool {
        self.inner.is_admissible(digest)
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

    fn binding(subject: u8) -> QualificationLineageBinding {
        QualificationLineageBinding::new(
            oid!(QualificationLineageId, 2),
            contract(),
            oid!(SubjectDigest, subject),
            Some(oid!(ContextDigest, 6)),
            AdmissionScope::ProductActivation,
            oid!(ClaimProfileDigest, 9),
        )
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

    fn verified(subject: u8, digest_value: u8) -> VerifiedQualification {
        let digest = oid!(ReceiptDigest, digest_value);
        let statement = ScopedQualificationStatement::new(
            ReceiptKind::P0ClosureQualification,
            binding(subject),
            oid!(DependencySetDigest, 4),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            ReceiptOutcome::Qualified,
            10,
            1_000,
            oid!(Nonce, 10),
            1,
            None,
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
        )
        .unwrap();
        verify_qualification(
            statement,
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
    fn same_opaque_lineage_id_cannot_cross_subject_scope() {
        let good = verified(3, 40);
        let other_subject = verified(99, 41);
        let mut ledger = QualificationLedger::new(binding(3));
        assert_eq!(ledger.admit(&good, 200), Ok(LedgerAdmissionOutcome::Added));
        assert_eq!(
            ledger.admit(&other_subject, 201),
            Err(ScopedLedgerError::LineageScopeMismatch)
        );
    }

    #[test]
    fn binding_is_exposed_without_exposing_private_verifier_internals() {
        let value = verified(3, 40);
        assert_eq!(value.binding(), binding(3));
        assert_eq!(value.receipt_digest(), oid!(ReceiptDigest, 40));
    }
}
