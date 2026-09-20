use core::fmt;
use std::collections::BTreeSet;

pub const QUALIFICATION_RECEIPT_SCHEMA_V1: u16 = 1;
const RECEIPT_DOMAIN: &[u8] = b"MYCELIX-HEALTH-QUALIFICATION-RECEIPT-V1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueIdError {
    AllZero,
}

macro_rules! opaque32 {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            pub fn new(bytes: [u8; 32]) -> Result<Self, OpaqueIdError> {
                if bytes == [0; 32] {
                    return Err(OpaqueIdError::AllZero);
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

opaque32!(ReceiptDigest);
opaque32!(ContractDigest);
opaque32!(SubjectDigest);
opaque32!(DependencySetDigest);
opaque32!(DeploymentEvidenceDigest);
opaque32!(ContextDigest);
opaque32!(Nonce);
opaque32!(GovernancePolicyDigest);
opaque32!(TrustStoreDigest);
opaque32!(ClaimProfileDigest);
opaque32!(QualificationLineageId);
opaque32!(ApproverId);
opaque32!(OrganizationId);
opaque32!(SignerKeyId);
opaque32!(DispositionEvidenceDigest);

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReceiptKind {
    ReferenceQualification = 1,
    DeploymentQualification = 2,
    MigrationResultQualification = 3,
    CompositionCommitmentQualification = 4,
    DurablePrepareQualification = 5,
    DurableActivationQualification = 6,
    AuditCheckpointQualification = 7,
    MonitoringAssessmentQualification = 8,
    P0ClosureQualification = 9,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionScope {
    Reference = 1,
    Deployment = 2,
    Cutover = 3,
    Audit = 4,
    Monitoring = 5,
    ProductActivation = 6,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptOutcome {
    Qualified = 1,
    Failed = 2,
    Indeterminate = 3,
    Superseded = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApproverRole {
    Clinical = 0,
    Privacy = 1,
    Compliance = 2,
    Safety = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleSet(u8);

impl RoleSet {
    pub const EMPTY: Self = Self(0);

    pub fn from_roles(roles: &[ApproverRole]) -> Self {
        let mut bits = 0u8;
        for role in roles {
            bits |= 1u8 << (*role as u8);
        }
        Self(bits)
    }

    pub fn contains(self, role: ApproverRole) -> bool {
        self.0 & (1u8 << (role as u8)) != 0
    }

    pub fn contains_all(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractRef {
    digest: ContractDigest,
    version: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractRefError {
    ZeroVersion,
}

impl ContractRef {
    pub fn new(digest: ContractDigest, version: u16) -> Result<Self, ContractRefError> {
        if version == 0 {
            return Err(ContractRefError::ZeroVersion);
        }
        Ok(Self { digest, version })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualificationStatement {
    schema_version: u16,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementError {
    InvalidTimeWindow,
    ZeroSequence,
    InvalidPredecessorShape,
}

impl QualificationStatement {
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
        if expires_at_micros <= issued_at_micros {
            return Err(StatementError::InvalidTimeWindow);
        }
        if sequence == 0 {
            return Err(StatementError::ZeroSequence);
        }
        if (sequence == 1) != previous_receipt.is_none() {
            return Err(StatementError::InvalidPredecessorShape);
        }
        Ok(Self {
            schema_version: QUALIFICATION_RECEIPT_SCHEMA_V1,
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
        })
    }

    pub fn kind(&self) -> ReceiptKind {
        self.kind
    }

    pub fn lineage(&self) -> QualificationLineageId {
        self.lineage
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn nonce(&self) -> Nonce {
        self.nonce
    }

    pub fn previous_receipt(&self) -> Option<ReceiptDigest> {
        self.previous_receipt
    }

    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(RECEIPT_DOMAIN);
        out.extend_from_slice(&self.schema_version.to_be_bytes());
        out.push(self.kind as u8);
        out.extend_from_slice(self.contract.digest.as_bytes());
        out.extend_from_slice(&self.contract.version.to_be_bytes());
        out.extend_from_slice(self.lineage.as_bytes());
        out.extend_from_slice(self.subject.as_bytes());
        out.extend_from_slice(self.dependencies.as_bytes());
        append_optional_digest(&mut out, self.deployment_evidence.map(|value| *value.as_bytes()));
        append_optional_digest(&mut out, self.context.map(|value| *value.as_bytes()));
        out.push(self.outcome as u8);
        out.extend_from_slice(&self.issued_at_micros.to_be_bytes());
        out.extend_from_slice(&self.expires_at_micros.to_be_bytes());
        out.extend_from_slice(self.nonce.as_bytes());
        out.extend_from_slice(&self.sequence.to_be_bytes());
        append_optional_digest(&mut out, self.previous_receipt.map(|value| *value.as_bytes()));
        out.extend_from_slice(self.policy.as_bytes());
        out.extend_from_slice(self.trust_store.as_bytes());
        out.push(self.admission_scope as u8);
        out.extend_from_slice(self.claim_profile.as_bytes());
        out
    }
}

fn append_optional_digest(out: &mut Vec<u8>, value: Option<[u8; 32]>) {
    match value {
        Some(bytes) => {
            out.push(1);
            out.extend_from_slice(&bytes);
        }
        None => out.push(0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GovernanceRule {
    kind: ReceiptKind,
    minimum_approvers: u8,
    minimum_organizations: u8,
    required_roles: RoleSet,
    max_attestation_age_micros: i64,
    max_statement_lifetime_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GovernanceRuleError {
    InvalidThreshold,
    EmptyRequiredRoles,
    InvalidAge,
    InvalidStatementLifetime,
}

impl GovernanceRule {
    pub fn new(
        kind: ReceiptKind,
        minimum_approvers: u8,
        minimum_organizations: u8,
        required_roles: RoleSet,
        max_attestation_age_micros: i64,
        max_statement_lifetime_micros: i64,
    ) -> Result<Self, GovernanceRuleError> {
        if minimum_approvers == 0
            || minimum_organizations == 0
            || minimum_organizations > minimum_approvers
        {
            return Err(GovernanceRuleError::InvalidThreshold);
        }
        if required_roles.is_empty() {
            return Err(GovernanceRuleError::EmptyRequiredRoles);
        }
        if max_attestation_age_micros <= 0 {
            return Err(GovernanceRuleError::InvalidAge);
        }
        if max_statement_lifetime_micros <= 0 {
            return Err(GovernanceRuleError::InvalidStatementLifetime);
        }
        Ok(Self {
            kind,
            minimum_approvers,
            minimum_organizations,
            required_roles,
            max_attestation_age_micros,
            max_statement_lifetime_micros,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationPolicy {
    policy_digest: GovernancePolicyDigest,
    trust_store_digest: TrustStoreDigest,
    expected_deployment_evidence: Option<DeploymentEvidenceDigest>,
    rule: GovernanceRule,
}

impl QualificationPolicy {
    pub fn new(
        policy_digest: GovernancePolicyDigest,
        trust_store_digest: TrustStoreDigest,
        expected_deployment_evidence: Option<DeploymentEvidenceDigest>,
        rule: GovernanceRule,
    ) -> Self {
        Self {
            policy_digest,
            trust_store_digest,
            expected_deployment_evidence,
            rule,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApproverVerification {
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
}

impl ApproverVerification {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verifier_adapter(
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
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptCommitmentReceipt {
    digest: ReceiptDigest,
    canonical_transcript_verified: bool,
}

impl ReceiptCommitmentReceipt {
    pub fn from_hash_adapter(digest: ReceiptDigest, canonical_transcript_verified: bool) -> Self {
        Self {
            digest,
            canonical_transcript_verified,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedQualification {
    statement: QualificationStatement,
    receipt_digest: ReceiptDigest,
    policy_digest: GovernancePolicyDigest,
    trust_store_digest: TrustStoreDigest,
    admitted_approvers: Vec<ApproverId>,
}

impl VerifiedQualification {
    pub fn statement(&self) -> &QualificationStatement {
        &self.statement
    }

    pub fn receipt_digest(&self) -> ReceiptDigest {
        self.receipt_digest
    }

    pub fn policy_digest(&self) -> GovernancePolicyDigest {
        self.policy_digest
    }

    pub fn trust_store_digest(&self) -> TrustStoreDigest {
        self.trust_store_digest
    }

    pub fn admitted_approvers(&self) -> &[ApproverId] {
        &self.admitted_approvers
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationError {
    CommitmentNotVerified,
    OutcomeNotQualified,
    ReceiptKindMismatch,
    PolicyMismatch,
    TrustStoreMismatch,
    DeploymentEvidenceMismatch,
    StatementNotYetValid,
    StatementExpired,
    StatementLifetimeTooLong,
    NoAttestations,
    DuplicateApprover,
    DuplicateSignerKey,
    SignatureNotVerified,
    ApproverNotTrusted,
    ApproverInactive,
    ApproverRevoked,
    ApproverCompromised,
    AttestationOutsideStatementWindow,
    AttestationInFuture,
    AttestationTooOld,
    InsufficientApprovers,
    InsufficientOrganizations,
    MissingRequiredRoles,
}

pub fn verify_qualification_bundle(
    statement: QualificationStatement,
    commitment: ReceiptCommitmentReceipt,
    policy: QualificationPolicy,
    attestations: &[ApproverVerification],
    now_micros: i64,
) -> Result<VerifiedQualification, VerificationError> {
    if !commitment.canonical_transcript_verified {
        return Err(VerificationError::CommitmentNotVerified);
    }
    if statement.outcome != ReceiptOutcome::Qualified {
        return Err(VerificationError::OutcomeNotQualified);
    }
    if statement.kind != policy.rule.kind {
        return Err(VerificationError::ReceiptKindMismatch);
    }
    if statement.policy != policy.policy_digest {
        return Err(VerificationError::PolicyMismatch);
    }
    if statement.trust_store != policy.trust_store_digest {
        return Err(VerificationError::TrustStoreMismatch);
    }
    if statement.deployment_evidence != policy.expected_deployment_evidence {
        return Err(VerificationError::DeploymentEvidenceMismatch);
    }
    if now_micros < statement.issued_at_micros {
        return Err(VerificationError::StatementNotYetValid);
    }
    if now_micros >= statement.expires_at_micros {
        return Err(VerificationError::StatementExpired);
    }
    if statement.expires_at_micros - statement.issued_at_micros
        > policy.rule.max_statement_lifetime_micros
    {
        return Err(VerificationError::StatementLifetimeTooLong);
    }
    if attestations.is_empty() {
        return Err(VerificationError::NoAttestations);
    }

    let mut approvers = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut organizations = BTreeSet::new();
    let mut roles = RoleSet::EMPTY;
    let mut admitted_approvers = Vec::new();

    for attestation in attestations {
        if !approvers.insert(attestation.approver) {
            return Err(VerificationError::DuplicateApprover);
        }
        if !keys.insert(attestation.signer_key) {
            return Err(VerificationError::DuplicateSignerKey);
        }
        if !attestation.signature_verified {
            return Err(VerificationError::SignatureNotVerified);
        }
        if !attestation.trusted_at_verification {
            return Err(VerificationError::ApproverNotTrusted);
        }
        if !attestation.active_at_verification {
            return Err(VerificationError::ApproverInactive);
        }
        if attestation.revoked_at_verification {
            return Err(VerificationError::ApproverRevoked);
        }
        if attestation.compromised_at_verification {
            return Err(VerificationError::ApproverCompromised);
        }
        if attestation.attested_at_micros < statement.issued_at_micros
            || attestation.attested_at_micros >= statement.expires_at_micros
        {
            return Err(VerificationError::AttestationOutsideStatementWindow);
        }
        if attestation.attested_at_micros > now_micros {
            return Err(VerificationError::AttestationInFuture);
        }
        if now_micros - attestation.attested_at_micros > policy.rule.max_attestation_age_micros {
            return Err(VerificationError::AttestationTooOld);
        }
        organizations.insert(attestation.organization);
        roles = roles.union(attestation.roles);
        admitted_approvers.push(attestation.approver);
    }

    if approvers.len() < usize::from(policy.rule.minimum_approvers) {
        return Err(VerificationError::InsufficientApprovers);
    }
    if organizations.len() < usize::from(policy.rule.minimum_organizations) {
        return Err(VerificationError::InsufficientOrganizations);
    }
    if !roles.contains_all(policy.rule.required_roles) {
        return Err(VerificationError::MissingRequiredRoles);
    }

    admitted_approvers.sort();
    Ok(VerifiedQualification {
        statement,
        receipt_digest: commitment.digest,
        policy_digest: policy.policy_digest,
        trust_store_digest: policy.trust_store_digest,
        admitted_approvers,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptStatus {
    Active,
    Superseded,
    Abandoned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LedgerEntry {
    receipt_digest: ReceiptDigest,
    nonce: Nonce,
    sequence: u64,
    previous_receipt: Option<ReceiptDigest>,
    status: ReceiptStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerAdmissionOutcome {
    Added,
    IdempotentReplay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerError {
    LineageMismatch,
    ClockRollback,
    ReceiptExpired,
    NonceReplay,
    StaleSequence,
    SequenceGap,
    PredecessorMismatch,
    ConflictingReceipt,
    ReceiptNotFound,
    DispositionNotVerified,
    InvalidDispositionEpoch,
    DispositionConflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptDisposition {
    Supersede,
    Abandon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispositionEvidence {
    lineage: QualificationLineageId,
    target: ReceiptDigest,
    disposition: ReceiptDisposition,
    governance_evidence: DispositionEvidenceDigest,
    disposition_epoch: u64,
    verified: bool,
}

impl DispositionEvidence {
    pub fn from_governance_adapter(
        lineage: QualificationLineageId,
        target: ReceiptDigest,
        disposition: ReceiptDisposition,
        governance_evidence: DispositionEvidenceDigest,
        disposition_epoch: u64,
        verified: bool,
    ) -> Self {
        Self {
            lineage,
            target,
            disposition,
            governance_evidence,
            disposition_epoch,
            verified,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DispositionRecord {
    target: ReceiptDigest,
    disposition: ReceiptDisposition,
    governance_evidence: DispositionEvidenceDigest,
    disposition_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct QualificationLedger {
    lineage: QualificationLineageId,
    entries: Vec<LedgerEntry>,
    dispositions: Vec<DispositionRecord>,
    last_disposition_epoch: u64,
    latest_time_micros: Option<i64>,
}

impl QualificationLedger {
    pub fn new(lineage: QualificationLineageId) -> Self {
        Self {
            lineage,
            entries: Vec::new(),
            dispositions: Vec::new(),
            last_disposition_epoch: 0,
            latest_time_micros: None,
        }
    }

    fn observe_time(&mut self, now_micros: i64) -> Result<(), LedgerError> {
        if self
            .latest_time_micros
            .is_some_and(|previous| now_micros < previous)
        {
            return Err(LedgerError::ClockRollback);
        }
        self.latest_time_micros = Some(now_micros);
        Ok(())
    }

    pub fn admit(
        &mut self,
        qualification: &VerifiedQualification,
        now_micros: i64,
    ) -> Result<LedgerAdmissionOutcome, LedgerError> {
        self.observe_time(now_micros)?;
        let statement = qualification.statement();
        if statement.lineage != self.lineage {
            return Err(LedgerError::LineageMismatch);
        }
        if now_micros >= statement.expires_at_micros {
            return Err(LedgerError::ReceiptExpired);
        }

        if let Some(existing) = self
            .entries
            .iter()
            .find(|entry| entry.receipt_digest == qualification.receipt_digest)
        {
            if existing.sequence == statement.sequence
                && existing.nonce == statement.nonce
                && existing.previous_receipt == statement.previous_receipt
            {
                return Ok(LedgerAdmissionOutcome::IdempotentReplay);
            }
            return Err(LedgerError::ConflictingReceipt);
        }

        if self.entries.iter().any(|entry| entry.nonce == statement.nonce) {
            return Err(LedgerError::NonceReplay);
        }

        match self.entries.last() {
            None => {
                if statement.sequence != 1 {
                    return Err(if statement.sequence < 1 {
                        LedgerError::StaleSequence
                    } else {
                        LedgerError::SequenceGap
                    });
                }
                if statement.previous_receipt.is_some() {
                    return Err(LedgerError::PredecessorMismatch);
                }
            }
            Some(head) => {
                let expected = head.sequence + 1;
                if statement.sequence < expected {
                    return Err(LedgerError::StaleSequence);
                }
                if statement.sequence > expected {
                    return Err(LedgerError::SequenceGap);
                }
                if statement.previous_receipt != Some(head.receipt_digest) {
                    return Err(LedgerError::PredecessorMismatch);
                }
            }
        }

        self.entries.push(LedgerEntry {
            receipt_digest: qualification.receipt_digest,
            nonce: statement.nonce,
            sequence: statement.sequence,
            previous_receipt: statement.previous_receipt,
            status: ReceiptStatus::Active,
        });
        Ok(LedgerAdmissionOutcome::Added)
    }

    pub fn apply_disposition(
        &mut self,
        evidence: DispositionEvidence,
    ) -> Result<LedgerAdmissionOutcome, LedgerError> {
        if evidence.lineage != self.lineage {
            return Err(LedgerError::LineageMismatch);
        }
        if !evidence.verified {
            return Err(LedgerError::DispositionNotVerified);
        }
        if let Some(existing) = self.dispositions.iter().find(|record| {
            record.target == evidence.target
                && record.disposition == evidence.disposition
                && record.governance_evidence == evidence.governance_evidence
                && record.disposition_epoch == evidence.disposition_epoch
        }) {
            let _ = existing;
            return Ok(LedgerAdmissionOutcome::IdempotentReplay);
        }
        if evidence.disposition_epoch != self.last_disposition_epoch + 1 {
            return Err(LedgerError::InvalidDispositionEpoch);
        }
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.receipt_digest == evidence.target)
            .ok_or(LedgerError::ReceiptNotFound)?;
        if entry.status != ReceiptStatus::Active {
            return Err(LedgerError::DispositionConflict);
        }
        entry.status = match evidence.disposition {
            ReceiptDisposition::Supersede => ReceiptStatus::Superseded,
            ReceiptDisposition::Abandon => ReceiptStatus::Abandoned,
        };
        self.dispositions.push(DispositionRecord {
            target: evidence.target,
            disposition: evidence.disposition,
            governance_evidence: evidence.governance_evidence,
            disposition_epoch: evidence.disposition_epoch,
        });
        self.last_disposition_epoch = evidence.disposition_epoch;
        Ok(LedgerAdmissionOutcome::Added)
    }

    pub fn status(&self, digest: ReceiptDigest) -> Option<ReceiptStatus> {
        self.entries
            .iter()
            .find(|entry| entry.receipt_digest == digest)
            .map(|entry| entry.status)
    }

    pub fn is_admissible(&self, digest: ReceiptDigest) -> bool {
        self.status(digest) == Some(ReceiptStatus::Active)
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
        kind: ReceiptKind,
        outcome: ReceiptOutcome,
    ) -> QualificationStatement {
        QualificationStatement::new(
            kind,
            contract(),
            oid!(QualificationLineageId, 2),
            oid!(SubjectDigest, 3),
            oid!(DependencySetDigest, 4),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            Some(oid!(ContextDigest, 6)),
            outcome,
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

    fn rule(kind: ReceiptKind) -> GovernanceRule {
        GovernanceRule::new(
            kind,
            2,
            2,
            RoleSet::from_roles(&[ApproverRole::Privacy, ApproverRole::Safety]),
            500,
            2_000,
        )
        .unwrap()
    }

    fn policy(kind: ReceiptKind) -> QualificationPolicy {
        QualificationPolicy::new(
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            rule(kind),
        )
    }

    fn attestations() -> Vec<ApproverVerification> {
        vec![
            ApproverVerification::from_verifier_adapter(
                oid!(ApproverId, 20),
                oid!(OrganizationId, 21),
                oid!(SignerKeyId, 22),
                RoleSet::from_roles(&[ApproverRole::Privacy]),
                100,
                true,
                true,
                true,
                false,
                false,
            ),
            ApproverVerification::from_verifier_adapter(
                oid!(ApproverId, 30),
                oid!(OrganizationId, 31),
                oid!(SignerKeyId, 32),
                RoleSet::from_roles(&[ApproverRole::Safety]),
                100,
                true,
                true,
                true,
                false,
                false,
            ),
        ]
    }

    fn verified(
        digest_value: u8,
        sequence: u64,
        previous: Option<ReceiptDigest>,
        nonce_value: u8,
    ) -> VerifiedQualification {
        verify_qualification_bundle(
            statement(
                sequence,
                previous,
                nonce_value,
                ReceiptKind::DurableActivationQualification,
                ReceiptOutcome::Qualified,
            ),
            ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, digest_value), true),
            policy(ReceiptKind::DurableActivationQualification),
            &attestations(),
            200,
        )
        .unwrap()
    }

    #[test]
    fn canonical_transcript_changes_when_subject_changes() {
        let a = statement(
            1,
            None,
            10,
            ReceiptKind::DurableActivationQualification,
            ReceiptOutcome::Qualified,
        );
        let mut b = a.clone();
        b.subject = oid!(SubjectDigest, 99);
        assert_ne!(a.transcript_bytes(), b.transcript_bytes());
    }

    #[test]
    fn nonqualified_outcome_and_unverified_commitment_deny() {
        let failed = statement(
            1,
            None,
            10,
            ReceiptKind::DurableActivationQualification,
            ReceiptOutcome::Failed,
        );
        assert_eq!(
            verify_qualification_bundle(
                failed,
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                policy(ReceiptKind::DurableActivationQualification),
                &attestations(),
                200,
            ),
            Err(VerificationError::OutcomeNotQualified)
        );

        let qualified = statement(
            1,
            None,
            10,
            ReceiptKind::DurableActivationQualification,
            ReceiptOutcome::Qualified,
        );
        assert_eq!(
            verify_qualification_bundle(
                qualified,
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), false),
                policy(ReceiptKind::DurableActivationQualification),
                &attestations(),
                200,
            ),
            Err(VerificationError::CommitmentNotVerified)
        );
    }

    #[test]
    fn policy_and_deployment_binding_are_exact() {
        let value = statement(
            1,
            None,
            10,
            ReceiptKind::DurableActivationQualification,
            ReceiptOutcome::Qualified,
        );
        let wrong = QualificationPolicy::new(
            oid!(GovernancePolicyDigest, 70),
            oid!(TrustStoreDigest, 8),
            Some(oid!(DeploymentEvidenceDigest, 5)),
            rule(ReceiptKind::DurableActivationQualification),
        );
        assert_eq!(
            verify_qualification_bundle(
                value.clone(),
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                wrong,
                &attestations(),
                200,
            ),
            Err(VerificationError::PolicyMismatch)
        );

        let wrong_deployment = QualificationPolicy::new(
            oid!(GovernancePolicyDigest, 7),
            oid!(TrustStoreDigest, 8),
            Some(oid!(DeploymentEvidenceDigest, 55)),
            rule(ReceiptKind::DurableActivationQualification),
        );
        assert_eq!(
            verify_qualification_bundle(
                value,
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                wrong_deployment,
                &attestations(),
                200,
            ),
            Err(VerificationError::DeploymentEvidenceMismatch)
        );
    }

    #[test]
    fn threshold_organization_and_role_requirements_are_independent() {
        let value = statement(
            1,
            None,
            10,
            ReceiptKind::DurableActivationQualification,
            ReceiptOutcome::Qualified,
        );
        let mut same_org = attestations();
        same_org[1].organization = same_org[0].organization;
        assert_eq!(
            verify_qualification_bundle(
                value.clone(),
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                policy(ReceiptKind::DurableActivationQualification),
                &same_org,
                200,
            ),
            Err(VerificationError::InsufficientOrganizations)
        );

        let mut missing_role = attestations();
        missing_role[1].roles = RoleSet::from_roles(&[ApproverRole::Privacy]);
        assert_eq!(
            verify_qualification_bundle(
                value,
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                policy(ReceiptKind::DurableActivationQualification),
                &missing_role,
                200,
            ),
            Err(VerificationError::MissingRequiredRoles)
        );
    }

    #[test]
    fn duplicate_or_untrusted_approvers_deny() {
        let value = statement(
            1,
            None,
            10,
            ReceiptKind::DurableActivationQualification,
            ReceiptOutcome::Qualified,
        );
        let mut duplicate = attestations();
        duplicate[1].approver = duplicate[0].approver;
        assert_eq!(
            verify_qualification_bundle(
                value.clone(),
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                policy(ReceiptKind::DurableActivationQualification),
                &duplicate,
                200,
            ),
            Err(VerificationError::DuplicateApprover)
        );

        let mut revoked = attestations();
        revoked[0].revoked_at_verification = true;
        assert_eq!(
            verify_qualification_bundle(
                value,
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                policy(ReceiptKind::DurableActivationQualification),
                &revoked,
                200,
            ),
            Err(VerificationError::ApproverRevoked)
        );
    }

    #[test]
    fn statement_and_attestation_freshness_fail_closed() {
        let value = statement(
            1,
            None,
            10,
            ReceiptKind::DurableActivationQualification,
            ReceiptOutcome::Qualified,
        );
        assert_eq!(
            verify_qualification_bundle(
                value.clone(),
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                policy(ReceiptKind::DurableActivationQualification),
                &attestations(),
                1_000,
            ),
            Err(VerificationError::StatementExpired)
        );

        let mut old = attestations();
        old[0].attested_at_micros = 20;
        old[1].attested_at_micros = 20;
        assert_eq!(
            verify_qualification_bundle(
                value,
                ReceiptCommitmentReceipt::from_hash_adapter(oid!(ReceiptDigest, 40), true),
                policy(ReceiptKind::DurableActivationQualification),
                &old,
                600,
            ),
            Err(VerificationError::AttestationTooOld)
        );
    }

    #[test]
    fn ledger_requires_monotonic_sequence_predecessor_and_unique_nonce() {
        let first = verified(40, 1, None, 10);
        let second = verified(41, 2, Some(first.receipt_digest()), 11);
        let mut ledger = QualificationLedger::new(oid!(QualificationLineageId, 2));
        assert_eq!(ledger.admit(&first, 200), Ok(LedgerAdmissionOutcome::Added));
        assert_eq!(ledger.admit(&second, 201), Ok(LedgerAdmissionOutcome::Added));

        let stale = verified(42, 2, Some(first.receipt_digest()), 12);
        assert_eq!(ledger.admit(&stale, 202), Err(LedgerError::StaleSequence));

        let gap = verified(43, 4, Some(second.receipt_digest()), 13);
        assert_eq!(ledger.admit(&gap, 203), Err(LedgerError::SequenceGap));

        let nonce_replay = verified(44, 3, Some(second.receipt_digest()), 10);
        assert_eq!(ledger.admit(&nonce_replay, 204), Err(LedgerError::NonceReplay));
    }

    #[test]
    fn exact_receipt_replay_is_idempotent_but_clock_rollback_denies() {
        let first = verified(40, 1, None, 10);
        let mut ledger = QualificationLedger::new(oid!(QualificationLineageId, 2));
        assert_eq!(ledger.admit(&first, 200), Ok(LedgerAdmissionOutcome::Added));
        assert_eq!(
            ledger.admit(&first, 201),
            Ok(LedgerAdmissionOutcome::IdempotentReplay)
        );
        assert_eq!(ledger.admit(&first, 199), Err(LedgerError::ClockRollback));
    }

    #[test]
    fn governed_disposition_makes_receipt_inadmissible_and_is_idempotent() {
        let first = verified(40, 1, None, 10);
        let mut ledger = QualificationLedger::new(oid!(QualificationLineageId, 2));
        ledger.admit(&first, 200).unwrap();
        let disposition = DispositionEvidence::from_governance_adapter(
            oid!(QualificationLineageId, 2),
            first.receipt_digest(),
            ReceiptDisposition::Abandon,
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
            Some(ReceiptStatus::Abandoned)
        );
        assert!(!ledger.is_admissible(first.receipt_digest()));
        assert_eq!(
            ledger.apply_disposition(disposition),
            Ok(LedgerAdmissionOutcome::IdempotentReplay)
        );
    }

    #[test]
    fn unverified_or_conflicting_disposition_denies() {
        let first = verified(40, 1, None, 10);
        let mut ledger = QualificationLedger::new(oid!(QualificationLineageId, 2));
        ledger.admit(&first, 200).unwrap();
        let unverified = DispositionEvidence::from_governance_adapter(
            oid!(QualificationLineageId, 2),
            first.receipt_digest(),
            ReceiptDisposition::Supersede,
            oid!(DispositionEvidenceDigest, 60),
            1,
            false,
        );
        assert_eq!(
            ledger.apply_disposition(unverified),
            Err(LedgerError::DispositionNotVerified)
        );
    }

    #[test]
    fn zero_identifiers_and_invalid_statement_shape_are_rejected() {
        assert_eq!(ReceiptDigest::new([0; 32]), Err(OpaqueIdError::AllZero));
        assert_eq!(ContractRef::new(oid!(ContractDigest, 1), 0), Err(ContractRefError::ZeroVersion));
        assert_eq!(
            QualificationStatement::new(
                ReceiptKind::ReferenceQualification,
                contract(),
                oid!(QualificationLineageId, 2),
                oid!(SubjectDigest, 3),
                oid!(DependencySetDigest, 4),
                None,
                None,
                ReceiptOutcome::Qualified,
                10,
                20,
                oid!(Nonce, 10),
                2,
                None,
                oid!(GovernancePolicyDigest, 7),
                oid!(TrustStoreDigest, 8),
                AdmissionScope::Reference,
                oid!(ClaimProfileDigest, 9),
            ),
            Err(StatementError::InvalidPredecessorShape)
        );
    }
}
