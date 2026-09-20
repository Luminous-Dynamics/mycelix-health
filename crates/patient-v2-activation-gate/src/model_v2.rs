use core::fmt;

pub const ACTIVATION_CONTRACT_V2: u16 = 2;
const EVIDENCE_TRANSCRIPT_DOMAIN: &[u8] = b"mycelix-health/patient-v2-evidence-set/v2\0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceCommit([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DnaManifestDigest([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntegrityWasmSetDigest([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoordinatorWasmSetDigest([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolchainDigest([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorityRef([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LegacyStateDigest([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueIdError {
    AllZero,
}

macro_rules! opaque32 {
    ($ty:ident) => {
        impl $ty {
            pub fn new(bytes: [u8; 32]) -> Result<Self, OpaqueIdError> {
                if bytes == [0u8; 32] {
                    return Err(OpaqueIdError::AllZero);
                }
                Ok(Self(bytes))
            }

            fn bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($ty), "([redacted])"))
            }
        }
    };
}

opaque32!(EvidenceId);
opaque32!(SourceCommit);
opaque32!(DnaManifestDigest);
opaque32!(IntegrityWasmSetDigest);
opaque32!(CoordinatorWasmSetDigest);
opaque32!(ToolchainDigest);
opaque32!(AuthorityRef);
opaque32!(LegacyStateDigest);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeploymentIdentity {
    source_commit: SourceCommit,
    dna_manifest: DnaManifestDigest,
    integrity_wasm_set: IntegrityWasmSetDigest,
    coordinator_wasm_set: CoordinatorWasmSetDigest,
    toolchain: ToolchainDigest,
    privacy_contract_epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentIdentityError {
    ZeroPrivacyContractEpoch,
}

impl DeploymentIdentity {
    pub fn new(
        source_commit: SourceCommit,
        dna_manifest: DnaManifestDigest,
        integrity_wasm_set: IntegrityWasmSetDigest,
        coordinator_wasm_set: CoordinatorWasmSetDigest,
        toolchain: ToolchainDigest,
        privacy_contract_epoch: u64,
    ) -> Result<Self, DeploymentIdentityError> {
        if privacy_contract_epoch == 0 {
            return Err(DeploymentIdentityError::ZeroPrivacyContractEpoch);
        }
        Ok(Self {
            source_commit,
            dna_manifest,
            integrity_wasm_set,
            coordinator_wasm_set,
            toolchain,
            privacy_contract_epoch,
        })
    }

    pub fn privacy_contract_epoch(&self) -> u64 {
        self.privacy_contract_epoch
    }

    fn append_transcript(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.source_commit.bytes());
        out.extend_from_slice(self.dna_manifest.bytes());
        out.extend_from_slice(self.integrity_wasm_set.bytes());
        out.extend_from_slice(self.coordinator_wasm_set.bytes());
        out.extend_from_slice(self.toolchain.bytes());
        out.extend_from_slice(&self.privacy_contract_epoch.to_be_bytes());
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractId {
    PatientProfileMigration = 1,
    ProtectedEnvelope = 2,
    VaultSession = 3,
    CareCapability = 4,
    CareDisclosure = 5,
    LinkTopologyInventory = 6,
    OpaqueLocator = 7,
    AuditChain = 8,
    SecurityMonitoring = 9,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractRef {
    pub id: ContractId,
    pub version: u16,
}

pub const PATIENT_PROFILE_MIGRATION_V2: ContractRef = ContractRef { id: ContractId::PatientProfileMigration, version: 2 };
pub const PROTECTED_ENVELOPE_V2: ContractRef = ContractRef { id: ContractId::ProtectedEnvelope, version: 2 };
pub const VAULT_SESSION_V1: ContractRef = ContractRef { id: ContractId::VaultSession, version: 1 };
pub const CARE_CAPABILITY_V1: ContractRef = ContractRef { id: ContractId::CareCapability, version: 1 };
pub const CARE_DISCLOSURE_V1: ContractRef = ContractRef { id: ContractId::CareDisclosure, version: 1 };
pub const LINK_TOPOLOGY_INVENTORY_V1: ContractRef = ContractRef { id: ContractId::LinkTopologyInventory, version: 1 };
pub const OPAQUE_LOCATOR_V1: ContractRef = ContractRef { id: ContractId::OpaqueLocator, version: 1 };
pub const AUDIT_CHAIN_V1: ContractRef = ContractRef { id: ContractId::AuditChain, version: 1 };
pub const SECURITY_MONITORING_V1: ContractRef = ContractRef { id: ContractId::SecurityMonitoring, version: 1 };

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProofLine {
    PatientRoutingReference = 1,
    EnvelopeReference = 2,
    EnvelopeProductionAdapter = 3,
    VaultSessionReference = 4,
    VaultSessionIntegration = 5,
    CareCapabilityReference = 6,
    CapabilityProductionAdapter = 7,
    CareDisclosureReference = 8,
    EntryStorageMigration = 9,
    LinkTopologyClassification = 10,
    LinkConstructionClassification = 11,
    LocatorReference = 12,
    LocatorIntegration = 13,
    IdentitySummaryIntegration = 14,
    AuditReference = 15,
    AuditIntegration = 16,
    MonitoringReference = 17,
    MultiAgentConfidentiality = 18,
    P0Closure = 19,
    LegacyWriteFreezeIntegration = 20,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementScope {
    Reference(ContractRef),
    Deployment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRequirement {
    pub line: ProofLine,
    pub scope: RequirementScope,
}

pub const REQUIRED_PROOFS: [ProofRequirement; 20] = [
    ProofRequirement { line: ProofLine::PatientRoutingReference, scope: RequirementScope::Reference(PATIENT_PROFILE_MIGRATION_V2) },
    ProofRequirement { line: ProofLine::EnvelopeReference, scope: RequirementScope::Reference(PROTECTED_ENVELOPE_V2) },
    ProofRequirement { line: ProofLine::EnvelopeProductionAdapter, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::VaultSessionReference, scope: RequirementScope::Reference(VAULT_SESSION_V1) },
    ProofRequirement { line: ProofLine::VaultSessionIntegration, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::CareCapabilityReference, scope: RequirementScope::Reference(CARE_CAPABILITY_V1) },
    ProofRequirement { line: ProofLine::CapabilityProductionAdapter, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::CareDisclosureReference, scope: RequirementScope::Reference(CARE_DISCLOSURE_V1) },
    ProofRequirement { line: ProofLine::EntryStorageMigration, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::LinkTopologyClassification, scope: RequirementScope::Reference(LINK_TOPOLOGY_INVENTORY_V1) },
    ProofRequirement { line: ProofLine::LinkConstructionClassification, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::LocatorReference, scope: RequirementScope::Reference(OPAQUE_LOCATOR_V1) },
    ProofRequirement { line: ProofLine::LocatorIntegration, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::IdentitySummaryIntegration, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::AuditReference, scope: RequirementScope::Reference(AUDIT_CHAIN_V1) },
    ProofRequirement { line: ProofLine::AuditIntegration, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::MonitoringReference, scope: RequirementScope::Reference(SECURITY_MONITORING_V1) },
    ProofRequirement { line: ProofLine::MultiAgentConfidentiality, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::P0Closure, scope: RequirementScope::Deployment },
    ProofRequirement { line: ProofLine::LegacyWriteFreezeIntegration, scope: RequirementScope::Deployment },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceState {
    Qualified,
    SourceStaged,
    QueuedInfrastructure,
    Failed,
    Stale,
    Missing,
    Superseded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceScope {
    Reference(ContractRef),
    Deployment(DeploymentIdentity),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceReceipt {
    line: ProofLine,
    evidence_id: EvidenceId,
    state: EvidenceState,
    scope: EvidenceScope,
}

impl EvidenceReceipt {
    /// Structural adapter boundary only. This core does not verify CI/signatures itself.
    pub fn from_adapter(
        line: ProofLine,
        evidence_id: EvidenceId,
        state: EvidenceState,
        scope: EvidenceScope,
    ) -> Self {
        Self { line, evidence_id, state, scope }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationCandidate {
    deployment: DeploymentIdentity,
    legacy_public_exposure_may_persist: bool,
    contract_version: u16,
}

impl ActivationCandidate {
    pub fn new(deployment: DeploymentIdentity) -> Self {
        Self {
            deployment,
            legacy_public_exposure_may_persist: true,
            contract_version: ACTIVATION_CONTRACT_V2,
        }
    }

    pub fn deployment(&self) -> DeploymentIdentity {
        self.deployment
    }

    pub fn legacy_public_exposure_may_persist(&self) -> bool {
        self.legacy_public_exposure_may_persist
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceBinding {
    line: ProofLine,
    evidence_id: EvidenceId,
    scope: EvidenceScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceSetReceipt {
    deployment: DeploymentIdentity,
    bindings: Vec<EvidenceBinding>,
    legacy_public_exposure_may_persist: bool,
    contract_version: u16,
}

impl EvidenceSetReceipt {
    pub fn deployment(&self) -> DeploymentIdentity {
        self.deployment
    }

    pub fn bindings(&self) -> &[EvidenceBinding] {
        &self.bindings
    }

    pub fn legacy_public_exposure_may_persist(&self) -> bool {
        self.legacy_public_exposure_may_persist
    }

    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(EVIDENCE_TRANSCRIPT_DOMAIN);
        out.extend_from_slice(&self.contract_version.to_be_bytes());
        self.deployment.append_transcript(&mut out);
        out.extend_from_slice(&(self.bindings.len() as u64).to_be_bytes());
        for binding in &self.bindings {
            out.push(binding.line as u8);
            out.extend_from_slice(binding.evidence_id.bytes());
            match binding.scope {
                EvidenceScope::Reference(contract) => {
                    out.push(1);
                    out.push(contract.id as u8);
                    out.extend_from_slice(&contract.version.to_be_bytes());
                }
                EvidenceScope::Deployment(deployment) => {
                    out.push(2);
                    deployment.append_transcript(&mut out);
                }
            }
        }
        out.push(u8::from(self.legacy_public_exposure_may_persist));
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceGateError {
    MissingProof(ProofLine),
    DuplicateProof(ProofLine),
    DuplicateEvidenceId,
    NotQualified(ProofLine, EvidenceState),
    ScopeKindMismatch(ProofLine),
    ReferenceContractMismatch(ProofLine),
    DeploymentMismatch(ProofLine),
}

pub fn evaluate_evidence(
    candidate: ActivationCandidate,
    receipts: &[EvidenceReceipt],
) -> Result<EvidenceSetReceipt, EvidenceGateError> {
    for (index, receipt) in receipts.iter().enumerate() {
        if receipts[index + 1..]
            .iter()
            .any(|other| other.evidence_id == receipt.evidence_id)
        {
            return Err(EvidenceGateError::DuplicateEvidenceId);
        }
    }

    let mut admitted = Vec::with_capacity(REQUIRED_PROOFS.len());
    for requirement in REQUIRED_PROOFS {
        let mut matching = receipts.iter().filter(|receipt| receipt.line == requirement.line);
        let Some(receipt) = matching.next() else {
            return Err(EvidenceGateError::MissingProof(requirement.line));
        };
        if matching.next().is_some() {
            return Err(EvidenceGateError::DuplicateProof(requirement.line));
        }
        if receipt.state != EvidenceState::Qualified {
            return Err(EvidenceGateError::NotQualified(requirement.line, receipt.state));
        }
        match (requirement.scope, receipt.scope) {
            (RequirementScope::Reference(expected), EvidenceScope::Reference(actual)) => {
                if expected != actual {
                    return Err(EvidenceGateError::ReferenceContractMismatch(requirement.line));
                }
            }
            (RequirementScope::Deployment, EvidenceScope::Deployment(actual)) => {
                if actual != candidate.deployment {
                    return Err(EvidenceGateError::DeploymentMismatch(requirement.line));
                }
            }
            _ => return Err(EvidenceGateError::ScopeKindMismatch(requirement.line)),
        }
        admitted.push(EvidenceBinding {
            line: requirement.line,
            evidence_id: receipt.evidence_id,
            scope: receipt.scope,
        });
    }
    admitted.sort_by_key(|binding| binding.line);

    Ok(EvidenceSetReceipt {
        deployment: candidate.deployment,
        bindings: admitted,
        legacy_public_exposure_may_persist: true,
        contract_version: ACTIVATION_CONTRACT_V2,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutoverFreezeReceipt {
    evidence_id: EvidenceId,
    state: EvidenceState,
    deployment: DeploymentIdentity,
    legacy_state_digest: LegacyStateDigest,
    freeze_epoch: u64,
    legacy_writes_blocked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoverFreezeError {
    NotQualified,
    DeploymentMismatch,
    ZeroFreezeEpoch,
    LegacyWritesNotBlocked,
    DuplicateEvidenceId,
}

impl CutoverFreezeReceipt {
    pub fn from_verified_adapter(
        evidence_id: EvidenceId,
        state: EvidenceState,
        deployment: DeploymentIdentity,
        legacy_state_digest: LegacyStateDigest,
        freeze_epoch: u64,
        legacy_writes_blocked: bool,
    ) -> Self {
        Self {
            evidence_id,
            state,
            deployment,
            legacy_state_digest,
            freeze_epoch,
            legacy_writes_blocked,
        }
    }

    pub fn legacy_state_digest(&self) -> LegacyStateDigest {
        self.legacy_state_digest
    }
}

fn validate_freeze(
    evidence: &EvidenceSetReceipt,
    freeze: &CutoverFreezeReceipt,
) -> Result<(), CutoverFreezeError> {
    if freeze.state != EvidenceState::Qualified {
        return Err(CutoverFreezeError::NotQualified);
    }
    if freeze.deployment != evidence.deployment {
        return Err(CutoverFreezeError::DeploymentMismatch);
    }
    if freeze.freeze_epoch == 0 {
        return Err(CutoverFreezeError::ZeroFreezeEpoch);
    }
    if !freeze.legacy_writes_blocked {
        return Err(CutoverFreezeError::LegacyWritesNotBlocked);
    }
    if evidence.bindings.iter().any(|binding| binding.evidence_id == freeze.evidence_id) {
        return Err(CutoverFreezeError::DuplicateEvidenceId);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrationRehearsalReceipt {
    evidence_id: EvidenceId,
    state: EvidenceState,
    deployment: DeploymentIdentity,
    legacy_state_digest: LegacyStateDigest,
    freeze_epoch: u64,
    idempotency_verified: bool,
    conflict_detection_verified: bool,
    no_plaintext_fallback_verified: bool,
    unresolved_routes_accounted_for: bool,
    legacy_public_exposure_may_persist: bool,
}

impl MigrationRehearsalReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_adapter(
        evidence_id: EvidenceId,
        state: EvidenceState,
        deployment: DeploymentIdentity,
        legacy_state_digest: LegacyStateDigest,
        freeze_epoch: u64,
        idempotency_verified: bool,
        conflict_detection_verified: bool,
        no_plaintext_fallback_verified: bool,
        unresolved_routes_accounted_for: bool,
    ) -> Self {
        Self {
            evidence_id,
            state,
            deployment,
            legacy_state_digest,
            freeze_epoch,
            idempotency_verified,
            conflict_detection_verified,
            no_plaintext_fallback_verified,
            unresolved_routes_accounted_for,
            legacy_public_exposure_may_persist: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RehearsalError {
    NotQualified,
    DeploymentMismatch,
    FrozenStateMismatch,
    FreezeEpochMismatch,
    IncompleteRehearsal,
    DuplicateEvidenceId,
}

fn validate_rehearsal(
    evidence: &EvidenceSetReceipt,
    freeze: &CutoverFreezeReceipt,
    rehearsal: &MigrationRehearsalReceipt,
) -> Result<(), RehearsalError> {
    if rehearsal.state != EvidenceState::Qualified {
        return Err(RehearsalError::NotQualified);
    }
    if rehearsal.deployment != evidence.deployment {
        return Err(RehearsalError::DeploymentMismatch);
    }
    if rehearsal.legacy_state_digest != freeze.legacy_state_digest {
        return Err(RehearsalError::FrozenStateMismatch);
    }
    if rehearsal.freeze_epoch != freeze.freeze_epoch {
        return Err(RehearsalError::FreezeEpochMismatch);
    }
    if evidence.bindings.iter().any(|binding| binding.evidence_id == rehearsal.evidence_id)
        || freeze.evidence_id == rehearsal.evidence_id
    {
        return Err(RehearsalError::DuplicateEvidenceId);
    }
    if !(rehearsal.idempotency_verified
        && rehearsal.conflict_detection_verified
        && rehearsal.no_plaintext_fallback_verified
        && rehearsal.unresolved_routes_accounted_for
        && rehearsal.legacy_public_exposure_may_persist)
    {
        return Err(RehearsalError::IncompleteRehearsal);
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub struct ActivationAuthorityReceipt {
    authority_ref: AuthorityRef,
    deployment: DeploymentIdentity,
    evidence_set_transcript: Vec<u8>,
    freeze_evidence_id: EvidenceId,
    freeze_state_digest: LegacyStateDigest,
    freeze_epoch: u64,
    rehearsal_evidence_id: EvidenceId,
    issued_at_micros: i64,
    expires_at_micros: i64,
    legacy_public_exposure_acknowledged: bool,
}

impl fmt::Debug for ActivationAuthorityReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivationAuthorityReceipt")
            .field("authority_ref", &self.authority_ref)
            .field("deployment", &self.deployment)
            .field("evidence_set_transcript", &"[redacted]")
            .field("freeze_evidence_id", &self.freeze_evidence_id)
            .field("freeze_state_digest", &self.freeze_state_digest)
            .field("freeze_epoch", &self.freeze_epoch)
            .field("rehearsal_evidence_id", &self.rehearsal_evidence_id)
            .field("issued_at_micros", &self.issued_at_micros)
            .field("expires_at_micros", &self.expires_at_micros)
            .field("legacy_public_exposure_acknowledged", &self.legacy_public_exposure_acknowledged)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityReceiptError {
    InvalidTimeWindow,
    EmptyEvidenceTranscript,
}

impl ActivationAuthorityReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_adapter(
        authority_ref: AuthorityRef,
        deployment: DeploymentIdentity,
        evidence_set_transcript: Vec<u8>,
        freeze_evidence_id: EvidenceId,
        freeze_state_digest: LegacyStateDigest,
        freeze_epoch: u64,
        rehearsal_evidence_id: EvidenceId,
        issued_at_micros: i64,
        expires_at_micros: i64,
        legacy_public_exposure_acknowledged: bool,
    ) -> Result<Self, AuthorityReceiptError> {
        if expires_at_micros <= issued_at_micros {
            return Err(AuthorityReceiptError::InvalidTimeWindow);
        }
        if evidence_set_transcript.is_empty() {
            return Err(AuthorityReceiptError::EmptyEvidenceTranscript);
        }
        Ok(Self {
            authority_ref,
            deployment,
            evidence_set_transcript,
            freeze_evidence_id,
            freeze_state_digest,
            freeze_epoch,
            rehearsal_evidence_id,
            issued_at_micros,
            expires_at_micros,
            legacy_public_exposure_acknowledged,
        })
    }

    fn valid_at(&self, now_micros: i64) -> Result<(), TransitionError> {
        if now_micros < self.issued_at_micros {
            return Err(TransitionError::AuthorityNotYetValid);
        }
        if now_micros >= self.expires_at_micros {
            return Err(TransitionError::AuthorityExpired);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationPhase {
    LegacyV1Writable,
    CandidatePrepared,
    EvidenceComplete,
    LegacyWritesFrozen,
    MigrationRehearsed,
    ActivationAuthorityAdmitted,
    V2ActivationAuthorized,
    V2Active,
    LegacyV1ReadOnly,
}

#[derive(Clone)]
pub struct ActivationState {
    phase: ActivationPhase,
    candidate: Option<ActivationCandidate>,
    evidence: Option<EvidenceSetReceipt>,
    freeze: Option<CutoverFreezeReceipt>,
    rehearsal: Option<MigrationRehearsalReceipt>,
    authority: Option<ActivationAuthorityReceipt>,
}

impl fmt::Debug for ActivationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivationState")
            .field("phase", &self.phase)
            .field("candidate", &self.candidate)
            .field("evidence", &self.evidence.as_ref().map(|_| "[bound]"))
            .field("freeze", &self.freeze)
            .field("rehearsal", &self.rehearsal)
            .field("authority", &self.authority.as_ref().map(|_| "[bound]"))
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionError {
    InvalidState,
    CandidateMismatch,
    FreezeNotQualified,
    FreezeDeploymentMismatch,
    FreezeEpochInvalid,
    LegacyWritesNotBlocked,
    DuplicateFreezeEvidenceId,
    RehearsalNotQualified,
    RehearsalDeploymentMismatch,
    RehearsalFrozenStateMismatch,
    RehearsalFreezeEpochMismatch,
    RehearsalIncomplete,
    DuplicateRehearsalEvidenceId,
    AuthorityDeploymentMismatch,
    AuthorityEvidenceMismatch,
    AuthorityFreezeMismatch,
    AuthorityRehearsalMismatch,
    AuthorityNotYetValid,
    AuthorityExpired,
    AuthorityMissingHistoricalExposureAcknowledgement,
}

impl Default for ActivationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivationState {
    pub fn new() -> Self {
        Self {
            phase: ActivationPhase::LegacyV1Writable,
            candidate: None,
            evidence: None,
            freeze: None,
            rehearsal: None,
            authority: None,
        }
    }

    pub fn phase(&self) -> ActivationPhase {
        self.phase
    }

    pub fn prepare_candidate(&mut self, candidate: ActivationCandidate) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::LegacyV1Writable {
            return Err(TransitionError::InvalidState);
        }
        self.candidate = Some(candidate);
        self.phase = ActivationPhase::CandidatePrepared;
        Ok(())
    }

    pub fn admit_evidence(&mut self, evidence: EvidenceSetReceipt) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::CandidatePrepared {
            return Err(TransitionError::InvalidState);
        }
        let candidate = self.candidate.ok_or(TransitionError::InvalidState)?;
        if candidate.deployment != evidence.deployment {
            return Err(TransitionError::CandidateMismatch);
        }
        self.evidence = Some(evidence);
        self.phase = ActivationPhase::EvidenceComplete;
        Ok(())
    }

    pub fn admit_cutover_freeze(&mut self, freeze: CutoverFreezeReceipt) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::EvidenceComplete {
            return Err(TransitionError::InvalidState);
        }
        let evidence = self.evidence.as_ref().ok_or(TransitionError::InvalidState)?;
        match validate_freeze(evidence, &freeze) {
            Ok(()) => {}
            Err(CutoverFreezeError::NotQualified) => return Err(TransitionError::FreezeNotQualified),
            Err(CutoverFreezeError::DeploymentMismatch) => return Err(TransitionError::FreezeDeploymentMismatch),
            Err(CutoverFreezeError::ZeroFreezeEpoch) => return Err(TransitionError::FreezeEpochInvalid),
            Err(CutoverFreezeError::LegacyWritesNotBlocked) => return Err(TransitionError::LegacyWritesNotBlocked),
            Err(CutoverFreezeError::DuplicateEvidenceId) => return Err(TransitionError::DuplicateFreezeEvidenceId),
        }
        self.freeze = Some(freeze);
        self.phase = ActivationPhase::LegacyWritesFrozen;
        Ok(())
    }

    pub fn admit_rehearsal(&mut self, rehearsal: MigrationRehearsalReceipt) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::LegacyWritesFrozen {
            return Err(TransitionError::InvalidState);
        }
        let evidence = self.evidence.as_ref().ok_or(TransitionError::InvalidState)?;
        let freeze = self.freeze.as_ref().ok_or(TransitionError::InvalidState)?;
        match validate_rehearsal(evidence, freeze, &rehearsal) {
            Ok(()) => {}
            Err(RehearsalError::NotQualified) => return Err(TransitionError::RehearsalNotQualified),
            Err(RehearsalError::DeploymentMismatch) => return Err(TransitionError::RehearsalDeploymentMismatch),
            Err(RehearsalError::FrozenStateMismatch) => return Err(TransitionError::RehearsalFrozenStateMismatch),
            Err(RehearsalError::FreezeEpochMismatch) => return Err(TransitionError::RehearsalFreezeEpochMismatch),
            Err(RehearsalError::IncompleteRehearsal) => return Err(TransitionError::RehearsalIncomplete),
            Err(RehearsalError::DuplicateEvidenceId) => return Err(TransitionError::DuplicateRehearsalEvidenceId),
        }
        self.rehearsal = Some(rehearsal);
        self.phase = ActivationPhase::MigrationRehearsed;
        Ok(())
    }

    pub fn admit_activation_authority(
        &mut self,
        authority: ActivationAuthorityReceipt,
        now_micros: i64,
    ) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::MigrationRehearsed {
            return Err(TransitionError::InvalidState);
        }
        let candidate = self.candidate.ok_or(TransitionError::InvalidState)?;
        let evidence = self.evidence.as_ref().ok_or(TransitionError::InvalidState)?;
        let freeze = self.freeze.ok_or(TransitionError::InvalidState)?;
        let rehearsal = self.rehearsal.ok_or(TransitionError::InvalidState)?;

        if authority.deployment != candidate.deployment {
            return Err(TransitionError::AuthorityDeploymentMismatch);
        }
        if authority.evidence_set_transcript != evidence.transcript_bytes() {
            return Err(TransitionError::AuthorityEvidenceMismatch);
        }
        if authority.freeze_evidence_id != freeze.evidence_id
            || authority.freeze_state_digest != freeze.legacy_state_digest
            || authority.freeze_epoch != freeze.freeze_epoch
        {
            return Err(TransitionError::AuthorityFreezeMismatch);
        }
        if authority.rehearsal_evidence_id != rehearsal.evidence_id {
            return Err(TransitionError::AuthorityRehearsalMismatch);
        }
        authority.valid_at(now_micros)?;
        if !authority.legacy_public_exposure_acknowledged {
            return Err(TransitionError::AuthorityMissingHistoricalExposureAcknowledgement);
        }
        self.authority = Some(authority);
        self.phase = ActivationPhase::ActivationAuthorityAdmitted;
        Ok(())
    }

    pub fn authorize_activation(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::ActivationAuthorityAdmitted {
            return Err(TransitionError::InvalidState);
        }
        self.authority
            .as_ref()
            .ok_or(TransitionError::InvalidState)?
            .valid_at(now_micros)?;
        self.phase = ActivationPhase::V2ActivationAuthorized;
        Ok(())
    }

    pub fn activate_v2(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::V2ActivationAuthorized {
            return Err(TransitionError::InvalidState);
        }
        self.authority
            .as_ref()
            .ok_or(TransitionError::InvalidState)?
            .valid_at(now_micros)?;
        self.phase = ActivationPhase::V2Active;
        Ok(())
    }

    pub fn retire_legacy_v1(&mut self) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::V2Active {
            return Err(TransitionError::InvalidState);
        }
        self.phase = ActivationPhase::LegacyV1ReadOnly;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatientWriteTarget {
    LegacyV1,
    DenyDuringCutover,
    ProtectedV2,
}

pub fn write_target(state: &ActivationState) -> PatientWriteTarget {
    match state.phase {
        ActivationPhase::LegacyV1Writable
        | ActivationPhase::CandidatePrepared
        | ActivationPhase::EvidenceComplete => PatientWriteTarget::LegacyV1,
        ActivationPhase::LegacyWritesFrozen
        | ActivationPhase::MigrationRehearsed
        | ActivationPhase::ActivationAuthorityAdmitted
        | ActivationPhase::V2ActivationAuthorized => PatientWriteTarget::DenyDuringCutover,
        ActivationPhase::V2Active | ActivationPhase::LegacyV1ReadOnly => PatientWriteTarget::ProtectedV2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatientReadTarget {
    LegacyV1,
    ProtectedV2Required,
}

pub fn read_target(state: &ActivationState) -> PatientReadTarget {
    match state.phase {
        ActivationPhase::V2Active | ActivationPhase::LegacyV1ReadOnly => PatientReadTarget::ProtectedV2Required,
        _ => PatientReadTarget::LegacyV1,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectedReadFailure {
    NotFound,
    AuthorizationDenied,
    AuthenticationFailed,
    DecryptionFailed,
    InvalidEnvelope,
    LocatorUnavailable,
    StaleCapability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadResolution {
    LegacyAllowed,
    ProtectedRequired,
    DenyNoLegacyFallback(ProtectedReadFailure),
}

pub fn resolve_read(
    state: &ActivationState,
    protected_result: Result<(), ProtectedReadFailure>,
) -> ReadResolution {
    match read_target(state) {
        PatientReadTarget::LegacyV1 => ReadResolution::LegacyAllowed,
        PatientReadTarget::ProtectedV2Required => match protected_result {
            Ok(()) => ReadResolution::ProtectedRequired,
            Err(error) => ReadResolution::DenyNoLegacyFallback(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(value: u8) -> [u8; 32] { [value; 32] }
    fn evidence_id(value: u8) -> EvidenceId { EvidenceId::new(bytes(value)).unwrap() }
    fn digest(value: u8) -> LegacyStateDigest { LegacyStateDigest::new(bytes(value)).unwrap() }

    fn deployment(seed: u8) -> DeploymentIdentity {
        DeploymentIdentity::new(
            SourceCommit::new(bytes(seed)).unwrap(),
            DnaManifestDigest::new(bytes(seed + 1)).unwrap(),
            IntegrityWasmSetDigest::new(bytes(seed + 2)).unwrap(),
            CoordinatorWasmSetDigest::new(bytes(seed + 3)).unwrap(),
            ToolchainDigest::new(bytes(seed + 4)).unwrap(),
            1,
        ).unwrap()
    }

    fn qualified_receipts(candidate: ActivationCandidate) -> Vec<EvidenceReceipt> {
        REQUIRED_PROOFS.iter().enumerate().map(|(index, requirement)| {
            EvidenceReceipt::from_adapter(
                requirement.line,
                evidence_id((index + 1) as u8),
                EvidenceState::Qualified,
                match requirement.scope {
                    RequirementScope::Reference(contract) => EvidenceScope::Reference(contract),
                    RequirementScope::Deployment => EvidenceScope::Deployment(candidate.deployment()),
                },
            )
        }).collect()
    }

    fn complete_evidence() -> (ActivationCandidate, EvidenceSetReceipt) {
        let candidate = ActivationCandidate::new(deployment(20));
        let evidence = evaluate_evidence(candidate, &qualified_receipts(candidate)).unwrap();
        (candidate, evidence)
    }

    fn freeze(candidate: ActivationCandidate) -> CutoverFreezeReceipt {
        CutoverFreezeReceipt::from_verified_adapter(
            evidence_id(101),
            EvidenceState::Qualified,
            candidate.deployment(),
            digest(150),
            7,
            true,
        )
    }

    fn rehearsal(candidate: ActivationCandidate) -> MigrationRehearsalReceipt {
        MigrationRehearsalReceipt::from_verified_adapter(
            evidence_id(102),
            EvidenceState::Qualified,
            candidate.deployment(),
            digest(150),
            7,
            true,
            true,
            true,
            true,
        )
    }

    fn authority(
        candidate: ActivationCandidate,
        evidence: &EvidenceSetReceipt,
        freeze: CutoverFreezeReceipt,
        rehearsal: MigrationRehearsalReceipt,
        issued_at: i64,
        expires_at: i64,
    ) -> ActivationAuthorityReceipt {
        ActivationAuthorityReceipt::from_verified_adapter(
            AuthorityRef::new(bytes(200)).unwrap(),
            candidate.deployment(),
            evidence.transcript_bytes(),
            freeze.evidence_id,
            freeze.legacy_state_digest,
            freeze.freeze_epoch,
            rehearsal.evidence_id,
            issued_at,
            expires_at,
            true,
        ).unwrap()
    }

    #[test]
    fn missing_or_nonqualified_evidence_denies() {
        let candidate = ActivationCandidate::new(deployment(20));
        let mut receipts = qualified_receipts(candidate);
        receipts.pop();
        assert!(matches!(evaluate_evidence(candidate, &receipts), Err(EvidenceGateError::MissingProof(_))));

        for state in [EvidenceState::SourceStaged, EvidenceState::QueuedInfrastructure, EvidenceState::Failed, EvidenceState::Stale, EvidenceState::Missing, EvidenceState::Superseded] {
            let mut receipts = qualified_receipts(candidate);
            receipts[0].state = state;
            assert!(matches!(evaluate_evidence(candidate, &receipts), Err(EvidenceGateError::NotQualified(_, observed)) if observed == state));
        }
    }

    #[test]
    fn evidence_scope_and_ids_are_exact() {
        let candidate = ActivationCandidate::new(deployment(20));
        let mut receipts = qualified_receipts(candidate);
        let prod = REQUIRED_PROOFS.iter().position(|r| r.line == ProofLine::EnvelopeProductionAdapter).unwrap();
        receipts[prod].scope = EvidenceScope::Deployment(deployment(40));
        assert_eq!(evaluate_evidence(candidate, &receipts), Err(EvidenceGateError::DeploymentMismatch(ProofLine::EnvelopeProductionAdapter)));

        let mut receipts = qualified_receipts(candidate);
        receipts[1].evidence_id = receipts[0].evidence_id;
        assert_eq!(evaluate_evidence(candidate, &receipts), Err(EvidenceGateError::DuplicateEvidenceId));
    }

    #[test]
    fn evidence_transcript_binds_full_deployment_and_scope() {
        let (candidate, evidence) = complete_evidence();
        assert_eq!(evidence.bindings().len(), REQUIRED_PROOFS.len());
        assert!(evidence.legacy_public_exposure_may_persist());

        let other = ActivationCandidate::new(deployment(40));
        let other_evidence = evaluate_evidence(other, &qualified_receipts(other)).unwrap();
        assert_ne!(evidence.transcript_bytes(), other_evidence.transcript_bytes());
        assert_eq!(candidate.deployment(), evidence.deployment());
    }

    #[test]
    fn freeze_must_be_qualified_bound_and_actually_block_writes() {
        let (candidate, evidence) = complete_evidence();
        let mut good = freeze(candidate);
        assert_eq!(validate_freeze(&evidence, &good), Ok(()));
        good.state = EvidenceState::QueuedInfrastructure;
        assert_eq!(validate_freeze(&evidence, &good), Err(CutoverFreezeError::NotQualified));

        let mut bad = freeze(candidate);
        bad.legacy_writes_blocked = false;
        assert_eq!(validate_freeze(&evidence, &bad), Err(CutoverFreezeError::LegacyWritesNotBlocked));
    }

    #[test]
    fn rehearsal_must_bind_exact_frozen_state_and_epoch() {
        let (candidate, evidence) = complete_evidence();
        let freeze = freeze(candidate);
        let mut rehearsal = rehearsal(candidate);
        assert_eq!(validate_rehearsal(&evidence, &freeze, &rehearsal), Ok(()));

        rehearsal.legacy_state_digest = digest(151);
        assert_eq!(validate_rehearsal(&evidence, &freeze, &rehearsal), Err(RehearsalError::FrozenStateMismatch));
        rehearsal = self::rehearsal(candidate);
        rehearsal.freeze_epoch = 8;
        assert_eq!(validate_rehearsal(&evidence, &freeze, &rehearsal), Err(RehearsalError::FreezeEpochMismatch));
    }

    #[test]
    fn state_is_not_externally_constructed_from_phase() {
        let state = ActivationState::new();
        assert_eq!(state.phase(), ActivationPhase::LegacyV1Writable);
        assert_eq!(write_target(&state), PatientWriteTarget::LegacyV1);
    }

    #[test]
    fn cutover_freeze_switches_write_target_to_deny() {
        let (candidate, evidence) = complete_evidence();
        let freeze = freeze(candidate);
        let mut state = ActivationState::new();
        state.prepare_candidate(candidate).unwrap();
        state.admit_evidence(evidence).unwrap();
        assert_eq!(write_target(&state), PatientWriteTarget::LegacyV1);
        state.admit_cutover_freeze(freeze).unwrap();
        assert_eq!(write_target(&state), PatientWriteTarget::DenyDuringCutover);
    }

    #[test]
    fn rehearsal_cannot_be_admitted_before_freeze() {
        let (candidate, evidence) = complete_evidence();
        let rehearsal = rehearsal(candidate);
        let mut state = ActivationState::new();
        state.prepare_candidate(candidate).unwrap();
        state.admit_evidence(evidence).unwrap();
        assert_eq!(state.admit_rehearsal(rehearsal), Err(TransitionError::InvalidState));
    }

    #[test]
    fn authority_binds_evidence_freeze_and_rehearsal() {
        let (candidate, evidence) = complete_evidence();
        let freeze = freeze(candidate);
        let rehearsal = rehearsal(candidate);
        let good = authority(candidate, &evidence, freeze, rehearsal, 10, 100);

        let mut state = ActivationState::new();
        state.prepare_candidate(candidate).unwrap();
        state.admit_evidence(evidence.clone()).unwrap();
        state.admit_cutover_freeze(freeze).unwrap();
        state.admit_rehearsal(rehearsal).unwrap();
        state.admit_activation_authority(good, 50).unwrap();
        assert_eq!(state.phase(), ActivationPhase::ActivationAuthorityAdmitted);

        let wrong_freeze = CutoverFreezeReceipt::from_verified_adapter(
            evidence_id(103), EvidenceState::Qualified, candidate.deployment(), digest(150), 7, true,
        );
        let wrong = authority(candidate, &evidence, wrong_freeze, rehearsal, 10, 100);
        let mut wrong_state = ActivationState::new();
        wrong_state.prepare_candidate(candidate).unwrap();
        wrong_state.admit_evidence(evidence).unwrap();
        wrong_state.admit_cutover_freeze(freeze).unwrap();
        wrong_state.admit_rehearsal(rehearsal).unwrap();
        assert_eq!(wrong_state.admit_activation_authority(wrong, 50), Err(TransitionError::AuthorityFreezeMismatch));
    }

    #[test]
    fn authority_expiry_is_rechecked_at_authorize_and_activation() {
        let (candidate, evidence) = complete_evidence();
        let freeze = freeze(candidate);
        let rehearsal = rehearsal(candidate);
        let authority = authority(candidate, &evidence, freeze, rehearsal, 10, 100);

        let mut state = ActivationState::new();
        state.prepare_candidate(candidate).unwrap();
        state.admit_evidence(evidence).unwrap();
        state.admit_cutover_freeze(freeze).unwrap();
        state.admit_rehearsal(rehearsal).unwrap();
        state.admit_activation_authority(authority, 50).unwrap();
        assert_eq!(state.authorize_activation(100), Err(TransitionError::AuthorityExpired));

        state.authorize_activation(99).unwrap();
        assert_eq!(state.activate_v2(100), Err(TransitionError::AuthorityExpired));
        state.activate_v2(99).unwrap();
        assert_eq!(state.phase(), ActivationPhase::V2Active);
    }

    #[test]
    fn authority_cannot_skip_freeze_rehearsal_or_evidence() {
        let mut state = ActivationState::new();
        assert_eq!(state.activate_v2(50), Err(TransitionError::InvalidState));
        let candidate = ActivationCandidate::new(deployment(20));
        state.prepare_candidate(candidate).unwrap();
        assert_eq!(state.authorize_activation(50), Err(TransitionError::InvalidState));
    }

    #[test]
    fn after_activation_writes_are_v2_and_reads_never_fallback() {
        let (candidate, evidence) = complete_evidence();
        let freeze = freeze(candidate);
        let rehearsal = rehearsal(candidate);
        let authority = authority(candidate, &evidence, freeze, rehearsal, 10, 100);

        let mut state = ActivationState::new();
        state.prepare_candidate(candidate).unwrap();
        state.admit_evidence(evidence).unwrap();
        state.admit_cutover_freeze(freeze).unwrap();
        state.admit_rehearsal(rehearsal).unwrap();
        state.admit_activation_authority(authority, 50).unwrap();
        state.authorize_activation(60).unwrap();
        state.activate_v2(70).unwrap();

        assert_eq!(write_target(&state), PatientWriteTarget::ProtectedV2);
        for failure in [
            ProtectedReadFailure::NotFound,
            ProtectedReadFailure::AuthorizationDenied,
            ProtectedReadFailure::AuthenticationFailed,
            ProtectedReadFailure::DecryptionFailed,
            ProtectedReadFailure::InvalidEnvelope,
            ProtectedReadFailure::LocatorUnavailable,
            ProtectedReadFailure::StaleCapability,
        ] {
            assert_eq!(resolve_read(&state, Err(failure)), ReadResolution::DenyNoLegacyFallback(failure));
        }
    }

    #[test]
    fn retirement_is_one_way_from_active() {
        let mut state = ActivationState::new();
        assert_eq!(state.retire_legacy_v1(), Err(TransitionError::InvalidState));
    }

    #[test]
    fn authority_requires_historical_exposure_acknowledgement() {
        let (candidate, evidence) = complete_evidence();
        let freeze = freeze(candidate);
        let rehearsal = rehearsal(candidate);
        let mut authority = authority(candidate, &evidence, freeze, rehearsal, 10, 100);
        authority.legacy_public_exposure_acknowledged = false;

        let mut state = ActivationState::new();
        state.prepare_candidate(candidate).unwrap();
        state.admit_evidence(evidence).unwrap();
        state.admit_cutover_freeze(freeze).unwrap();
        state.admit_rehearsal(rehearsal).unwrap();
        assert_eq!(state.admit_activation_authority(authority, 50), Err(TransitionError::AuthorityMissingHistoricalExposureAcknowledgement));
    }

    #[test]
    fn sensitive_debug_is_redacted() {
        let (candidate, evidence) = complete_evidence();
        let freeze = freeze(candidate);
        let rehearsal = rehearsal(candidate);
        let authority = authority(candidate, &evidence, freeze, rehearsal, 10, 100);
        let rendered = format!("{authority:?}");
        assert!(rendered.contains("[redacted]"));
        assert!(!rendered.contains("patient-v2-evidence-set"));
    }

    #[test]
    fn opaque_identifiers_reject_zero() {
        assert_eq!(EvidenceId::new([0; 32]), Err(OpaqueIdError::AllZero));
        assert_eq!(LegacyStateDigest::new([0; 32]), Err(OpaqueIdError::AllZero));
    }
}
