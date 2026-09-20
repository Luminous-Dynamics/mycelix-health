use core::fmt;

pub const ACTIVATION_CONTRACT_V2: u16 = 2;
const TRANSCRIPT_DOMAIN: &[u8] = b"mycelix-health/patient-v2-evidence-set/v2\0";

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
            #[allow(dead_code)]
            fn bytes(&self) -> &[u8; 32] {
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

const PATIENT_PROFILE_MIGRATION_V2: ContractRef = ContractRef { id: ContractId::PatientProfileMigration, version: 2 };
const PROTECTED_ENVELOPE_V2: ContractRef = ContractRef { id: ContractId::ProtectedEnvelope, version: 2 };
const VAULT_SESSION_V1: ContractRef = ContractRef { id: ContractId::VaultSession, version: 1 };
const CARE_CAPABILITY_V1: ContractRef = ContractRef { id: ContractId::CareCapability, version: 1 };
const CARE_DISCLOSURE_V1: ContractRef = ContractRef { id: ContractId::CareDisclosure, version: 1 };
const LINK_TOPOLOGY_INVENTORY_V1: ContractRef = ContractRef { id: ContractId::LinkTopologyInventory, version: 1 };
const OPAQUE_LOCATOR_V1: ContractRef = ContractRef { id: ContractId::OpaqueLocator, version: 1 };
const AUDIT_CHAIN_V1: ContractRef = ContractRef { id: ContractId::AuditChain, version: 1 };
const SECURITY_MONITORING_V1: ContractRef = ContractRef { id: ContractId::SecurityMonitoring, version: 1 };

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
}

impl ActivationCandidate {
    pub fn new(deployment: DeploymentIdentity) -> Self {
        Self { deployment, legacy_public_exposure_may_persist: true }
    }
    pub fn deployment(&self) -> DeploymentIdentity { self.deployment }
    pub fn legacy_public_exposure_may_persist(&self) -> bool { self.legacy_public_exposure_may_persist }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceSetReceipt {
    deployment: DeploymentIdentity,
    bindings: Vec<(ProofLine, EvidenceId, EvidenceScope)>,
    legacy_public_exposure_may_persist: bool,
}

impl EvidenceSetReceipt {
    pub fn deployment(&self) -> DeploymentIdentity { self.deployment }
    pub fn binding_count(&self) -> usize { self.bindings.len() }
    pub fn legacy_public_exposure_may_persist(&self) -> bool { self.legacy_public_exposure_may_persist }

    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(TRANSCRIPT_DOMAIN);
        out.extend_from_slice(&ACTIVATION_CONTRACT_V2.to_be_bytes());
        self.deployment.append_transcript(&mut out);
        out.extend_from_slice(&(self.bindings.len() as u64).to_be_bytes());
        for (line, id, scope) in &self.bindings {
            out.push(*line as u8);
            out.extend_from_slice(id.bytes());
            match scope {
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

pub fn evaluate_evidence(candidate: ActivationCandidate, receipts: &[EvidenceReceipt]) -> Result<EvidenceSetReceipt, EvidenceGateError> {
    for (index, receipt) in receipts.iter().enumerate() {
        if receipts[index + 1..].iter().any(|other| other.evidence_id == receipt.evidence_id) {
            return Err(EvidenceGateError::DuplicateEvidenceId);
        }
    }

    let mut bindings = Vec::with_capacity(REQUIRED_PROOFS.len());
    for requirement in REQUIRED_PROOFS {
        let mut matches = receipts.iter().filter(|receipt| receipt.line == requirement.line);
        let Some(receipt) = matches.next() else { return Err(EvidenceGateError::MissingProof(requirement.line)); };
        if matches.next().is_some() { return Err(EvidenceGateError::DuplicateProof(requirement.line)); }
        if receipt.state != EvidenceState::Qualified { return Err(EvidenceGateError::NotQualified(requirement.line, receipt.state)); }
        match (requirement.scope, receipt.scope) {
            (RequirementScope::Reference(expected), EvidenceScope::Reference(actual)) if expected == actual => {}
            (RequirementScope::Reference(_), EvidenceScope::Reference(_)) => return Err(EvidenceGateError::ReferenceContractMismatch(requirement.line)),
            (RequirementScope::Deployment, EvidenceScope::Deployment(actual)) if actual == candidate.deployment => {}
            (RequirementScope::Deployment, EvidenceScope::Deployment(_)) => return Err(EvidenceGateError::DeploymentMismatch(requirement.line)),
            _ => return Err(EvidenceGateError::ScopeKindMismatch(requirement.line)),
        }
        bindings.push((requirement.line, receipt.evidence_id, receipt.scope));
    }
    bindings.sort_by_key(|(line, _, _)| *line);
    Ok(EvidenceSetReceipt { deployment: candidate.deployment, bindings, legacy_public_exposure_may_persist: true })
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

impl CutoverFreezeReceipt {
    pub fn from_verified_adapter(
        evidence_id: EvidenceId,
        state: EvidenceState,
        deployment: DeploymentIdentity,
        legacy_state_digest: LegacyStateDigest,
        freeze_epoch: u64,
        legacy_writes_blocked: bool,
    ) -> Self {
        Self { evidence_id, state, deployment, legacy_state_digest, freeze_epoch, legacy_writes_blocked }
    }
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

#[derive(Clone, PartialEq, Eq)]
pub struct ActivationAuthorityReceipt {
    authority_ref: AuthorityRef,
    deployment: DeploymentIdentity,
    evidence_transcript: Vec<u8>,
    freeze_evidence_id: EvidenceId,
    freeze_state_digest: LegacyStateDigest,
    freeze_epoch: u64,
    rehearsal_evidence_id: EvidenceId,
    issued_at_micros: i64,
    expires_at_micros: i64,
    historical_exposure_acknowledged: bool,
}

impl fmt::Debug for ActivationAuthorityReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivationAuthorityReceipt")
            .field("authority_ref", &self.authority_ref)
            .field("deployment", &self.deployment)
            .field("evidence_transcript", &"[redacted]")
            .field("freeze_evidence_id", &self.freeze_evidence_id)
            .field("freeze_state_digest", &self.freeze_state_digest)
            .field("freeze_epoch", &self.freeze_epoch)
            .field("rehearsal_evidence_id", &self.rehearsal_evidence_id)
            .field("issued_at_micros", &self.issued_at_micros)
            .field("expires_at_micros", &self.expires_at_micros)
            .field("historical_exposure_acknowledged", &self.historical_exposure_acknowledged)
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
        evidence_transcript: Vec<u8>,
        freeze_evidence_id: EvidenceId,
        freeze_state_digest: LegacyStateDigest,
        freeze_epoch: u64,
        rehearsal_evidence_id: EvidenceId,
        issued_at_micros: i64,
        expires_at_micros: i64,
        historical_exposure_acknowledged: bool,
    ) -> Result<Self, AuthorityReceiptError> {
        if expires_at_micros <= issued_at_micros { return Err(AuthorityReceiptError::InvalidTimeWindow); }
        if evidence_transcript.is_empty() { return Err(AuthorityReceiptError::EmptyEvidenceTranscript); }
        Ok(Self {
            authority_ref,
            deployment,
            evidence_transcript,
            freeze_evidence_id,
            freeze_state_digest,
            freeze_epoch,
            rehearsal_evidence_id,
            issued_at_micros,
            expires_at_micros,
            historical_exposure_acknowledged,
        })
    }

    fn valid_at(&self, now_micros: i64) -> Result<(), TransitionError> {
        if now_micros < self.issued_at_micros { return Err(TransitionError::AuthorityNotYetValid); }
        if now_micros >= self.expires_at_micros { return Err(TransitionError::AuthorityExpired); }
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
    last_authority_time_micros: Option<i64>,
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
            .field("last_authority_time_micros", &self.last_authority_time_micros)
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
    ClockRollback,
}

impl Default for ActivationState {
    fn default() -> Self { Self::new() }
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
            last_authority_time_micros: None,
        }
    }

    pub fn phase(&self) -> ActivationPhase { self.phase }

    fn observe_authority_time(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        if self.last_authority_time_micros.is_some_and(|last| now_micros < last) {
            return Err(TransitionError::ClockRollback);
        }
        self.last_authority_time_micros = Some(now_micros);
        Ok(())
    }

    pub fn prepare_candidate(&mut self, candidate: ActivationCandidate) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::LegacyV1Writable { return Err(TransitionError::InvalidState); }
        self.candidate = Some(candidate);
        self.phase = ActivationPhase::CandidatePrepared;
        Ok(())
    }

    pub fn admit_evidence(&mut self, evidence: EvidenceSetReceipt) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::CandidatePrepared { return Err(TransitionError::InvalidState); }
        if self.candidate.ok_or(TransitionError::InvalidState)?.deployment != evidence.deployment {
            return Err(TransitionError::CandidateMismatch);
        }
        self.evidence = Some(evidence);
        self.phase = ActivationPhase::EvidenceComplete;
        Ok(())
    }

    pub fn admit_cutover_freeze(&mut self, freeze: CutoverFreezeReceipt) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::EvidenceComplete { return Err(TransitionError::InvalidState); }
        let evidence = self.evidence.as_ref().ok_or(TransitionError::InvalidState)?;
        if freeze.state != EvidenceState::Qualified { return Err(TransitionError::FreezeNotQualified); }
        if freeze.deployment != evidence.deployment { return Err(TransitionError::FreezeDeploymentMismatch); }
        if freeze.freeze_epoch == 0 { return Err(TransitionError::FreezeEpochInvalid); }
        if !freeze.legacy_writes_blocked { return Err(TransitionError::LegacyWritesNotBlocked); }
        if evidence.bindings.iter().any(|(_, id, _)| *id == freeze.evidence_id) { return Err(TransitionError::DuplicateFreezeEvidenceId); }
        self.freeze = Some(freeze);
        self.phase = ActivationPhase::LegacyWritesFrozen;
        Ok(())
    }

    pub fn admit_rehearsal(&mut self, rehearsal: MigrationRehearsalReceipt) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::LegacyWritesFrozen { return Err(TransitionError::InvalidState); }
        let evidence = self.evidence.as_ref().ok_or(TransitionError::InvalidState)?;
        let freeze = self.freeze.ok_or(TransitionError::InvalidState)?;
        if rehearsal.state != EvidenceState::Qualified { return Err(TransitionError::RehearsalNotQualified); }
        if rehearsal.deployment != evidence.deployment { return Err(TransitionError::RehearsalDeploymentMismatch); }
        if rehearsal.legacy_state_digest != freeze.legacy_state_digest { return Err(TransitionError::RehearsalFrozenStateMismatch); }
        if rehearsal.freeze_epoch != freeze.freeze_epoch { return Err(TransitionError::RehearsalFreezeEpochMismatch); }
        if evidence.bindings.iter().any(|(_, id, _)| *id == rehearsal.evidence_id) || rehearsal.evidence_id == freeze.evidence_id {
            return Err(TransitionError::DuplicateRehearsalEvidenceId);
        }
        if !(rehearsal.idempotency_verified
            && rehearsal.conflict_detection_verified
            && rehearsal.no_plaintext_fallback_verified
            && rehearsal.unresolved_routes_accounted_for
            && rehearsal.legacy_public_exposure_may_persist)
        {
            return Err(TransitionError::RehearsalIncomplete);
        }
        self.rehearsal = Some(rehearsal);
        self.phase = ActivationPhase::MigrationRehearsed;
        Ok(())
    }

    pub fn admit_activation_authority(&mut self, authority: ActivationAuthorityReceipt, now_micros: i64) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::MigrationRehearsed { return Err(TransitionError::InvalidState); }
        self.observe_authority_time(now_micros)?;
        let candidate = self.candidate.ok_or(TransitionError::InvalidState)?;
        let evidence = self.evidence.as_ref().ok_or(TransitionError::InvalidState)?;
        let freeze = self.freeze.ok_or(TransitionError::InvalidState)?;
        let rehearsal = self.rehearsal.ok_or(TransitionError::InvalidState)?;
        if authority.deployment != candidate.deployment { return Err(TransitionError::AuthorityDeploymentMismatch); }
        if authority.evidence_transcript != evidence.transcript_bytes() { return Err(TransitionError::AuthorityEvidenceMismatch); }
        if authority.freeze_evidence_id != freeze.evidence_id
            || authority.freeze_state_digest != freeze.legacy_state_digest
            || authority.freeze_epoch != freeze.freeze_epoch
        {
            return Err(TransitionError::AuthorityFreezeMismatch);
        }
        if authority.rehearsal_evidence_id != rehearsal.evidence_id { return Err(TransitionError::AuthorityRehearsalMismatch); }
        authority.valid_at(now_micros)?;
        if !authority.historical_exposure_acknowledged { return Err(TransitionError::AuthorityMissingHistoricalExposureAcknowledgement); }
        self.authority = Some(authority);
        self.phase = ActivationPhase::ActivationAuthorityAdmitted;
        Ok(())
    }

    pub fn authorize_activation(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::ActivationAuthorityAdmitted { return Err(TransitionError::InvalidState); }
        self.observe_authority_time(now_micros)?;
        self.authority.as_ref().ok_or(TransitionError::InvalidState)?.valid_at(now_micros)?;
        self.phase = ActivationPhase::V2ActivationAuthorized;
        Ok(())
    }

    pub fn activate_v2(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::V2ActivationAuthorized { return Err(TransitionError::InvalidState); }
        self.observe_authority_time(now_micros)?;
        self.authority.as_ref().ok_or(TransitionError::InvalidState)?.valid_at(now_micros)?;
        self.phase = ActivationPhase::V2Active;
        Ok(())
    }

    pub fn retire_legacy_v1(&mut self) -> Result<(), TransitionError> {
        if self.phase != ActivationPhase::V2Active { return Err(TransitionError::InvalidState); }
        self.phase = ActivationPhase::LegacyV1ReadOnly;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatientWriteTarget { LegacyV1, DenyDuringCutover, ProtectedV2 }

pub fn write_target(state: &ActivationState) -> PatientWriteTarget {
    match state.phase {
        ActivationPhase::LegacyV1Writable | ActivationPhase::CandidatePrepared | ActivationPhase::EvidenceComplete => PatientWriteTarget::LegacyV1,
        ActivationPhase::LegacyWritesFrozen | ActivationPhase::MigrationRehearsed | ActivationPhase::ActivationAuthorityAdmitted | ActivationPhase::V2ActivationAuthorized => PatientWriteTarget::DenyDuringCutover,
        ActivationPhase::V2Active | ActivationPhase::LegacyV1ReadOnly => PatientWriteTarget::ProtectedV2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectedReadFailure { NotFound, AuthorizationDenied, AuthenticationFailed, DecryptionFailed, InvalidEnvelope, LocatorUnavailable, StaleCapability }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadResolution { LegacyAllowed, ProtectedRequired, DenyNoLegacyFallback(ProtectedReadFailure) }

pub fn resolve_read(state: &ActivationState, protected_result: Result<(), ProtectedReadFailure>) -> ReadResolution {
    if matches!(state.phase, ActivationPhase::V2Active | ActivationPhase::LegacyV1ReadOnly) {
        return match protected_result {
            Ok(()) => ReadResolution::ProtectedRequired,
            Err(error) => ReadResolution::DenyNoLegacyFallback(error),
        };
    }
    ReadResolution::LegacyAllowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(v: u8) -> [u8; 32] { [v; 32] }
    fn eid(v: u8) -> EvidenceId { EvidenceId::new(b(v)).unwrap() }
    fn legacy(v: u8) -> LegacyStateDigest { LegacyStateDigest::new(b(v)).unwrap() }
    fn deployment(seed: u8) -> DeploymentIdentity {
        DeploymentIdentity::new(
            SourceCommit::new(b(seed)).unwrap(),
            DnaManifestDigest::new(b(seed + 1)).unwrap(),
            IntegrityWasmSetDigest::new(b(seed + 2)).unwrap(),
            CoordinatorWasmSetDigest::new(b(seed + 3)).unwrap(),
            ToolchainDigest::new(b(seed + 4)).unwrap(),
            1,
        ).unwrap()
    }
    fn candidate() -> ActivationCandidate { ActivationCandidate::new(deployment(20)) }
    fn receipts(c: ActivationCandidate) -> Vec<EvidenceReceipt> {
        REQUIRED_PROOFS.iter().enumerate().map(|(i, req)| EvidenceReceipt::from_adapter(
            req.line,
            eid((i + 1) as u8),
            EvidenceState::Qualified,
            match req.scope {
                RequirementScope::Reference(contract) => EvidenceScope::Reference(contract),
                RequirementScope::Deployment => EvidenceScope::Deployment(c.deployment()),
            },
        )).collect()
    }
    fn evidence(c: ActivationCandidate) -> EvidenceSetReceipt { evaluate_evidence(c, &receipts(c)).unwrap() }
    fn freeze(c: ActivationCandidate) -> CutoverFreezeReceipt {
        CutoverFreezeReceipt::from_verified_adapter(eid(101), EvidenceState::Qualified, c.deployment(), legacy(150), 7, true)
    }
    fn rehearsal(c: ActivationCandidate) -> MigrationRehearsalReceipt {
        MigrationRehearsalReceipt::from_verified_adapter(eid(102), EvidenceState::Qualified, c.deployment(), legacy(150), 7, true, true, true, true)
    }
    fn authority(c: ActivationCandidate, e: &EvidenceSetReceipt, f: CutoverFreezeReceipt, r: MigrationRehearsalReceipt, start: i64, end: i64) -> ActivationAuthorityReceipt {
        ActivationAuthorityReceipt::from_verified_adapter(
            AuthorityRef::new(b(200)).unwrap(), c.deployment(), e.transcript_bytes(), f.evidence_id,
            f.legacy_state_digest, f.freeze_epoch, r.evidence_id, start, end, true,
        ).unwrap()
    }
    fn frozen_rehearsed() -> (ActivationState, ActivationCandidate, EvidenceSetReceipt, CutoverFreezeReceipt, MigrationRehearsalReceipt) {
        let c = candidate();
        let e = evidence(c);
        let f = freeze(c);
        let r = rehearsal(c);
        let mut s = ActivationState::new();
        s.prepare_candidate(c).unwrap();
        s.admit_evidence(e.clone()).unwrap();
        s.admit_cutover_freeze(f).unwrap();
        s.admit_rehearsal(r).unwrap();
        (s, c, e, f, r)
    }

    #[test]
    fn every_nonqualified_evidence_state_denies() {
        let c = candidate();
        for state in [EvidenceState::SourceStaged, EvidenceState::QueuedInfrastructure, EvidenceState::Failed, EvidenceState::Stale, EvidenceState::Missing, EvidenceState::Superseded] {
            let mut rs = receipts(c);
            rs[0].state = state;
            assert!(matches!(evaluate_evidence(c, &rs), Err(EvidenceGateError::NotQualified(_, observed)) if observed == state));
        }
    }

    #[test]
    fn evidence_is_exactly_deployment_and_contract_bound() {
        let c = candidate();
        let mut rs = receipts(c);
        let prod = REQUIRED_PROOFS.iter().position(|r| r.line == ProofLine::EnvelopeProductionAdapter).unwrap();
        rs[prod].scope = EvidenceScope::Deployment(deployment(40));
        assert_eq!(evaluate_evidence(c, &rs), Err(EvidenceGateError::DeploymentMismatch(ProofLine::EnvelopeProductionAdapter)));
        let mut rs = receipts(c);
        rs[1].evidence_id = rs[0].evidence_id;
        assert_eq!(evaluate_evidence(c, &rs), Err(EvidenceGateError::DuplicateEvidenceId));
    }

    #[test]
    fn transcript_binds_full_deployment_identity() {
        let c = candidate();
        let e = evidence(c);
        let other = ActivationCandidate::new(deployment(40));
        let other_e = evidence(other);
        assert_eq!(e.binding_count(), 20);
        assert_ne!(e.transcript_bytes(), other_e.transcript_bytes());
        assert!(e.legacy_public_exposure_may_persist());
    }

    #[test]
    fn cutover_freeze_blocks_legacy_writes_before_rehearsal() {
        let c = candidate();
        let e = evidence(c);
        let f = freeze(c);
        let mut s = ActivationState::new();
        s.prepare_candidate(c).unwrap();
        s.admit_evidence(e).unwrap();
        assert_eq!(write_target(&s), PatientWriteTarget::LegacyV1);
        s.admit_cutover_freeze(f).unwrap();
        assert_eq!(write_target(&s), PatientWriteTarget::DenyDuringCutover);
    }

    #[test]
    fn freeze_and_rehearsal_must_match_exact_legacy_state() {
        let c = candidate();
        let e = evidence(c);
        let f = freeze(c);
        let mut r = rehearsal(c);
        r.legacy_state_digest = legacy(151);
        let mut s = ActivationState::new();
        s.prepare_candidate(c).unwrap();
        s.admit_evidence(e).unwrap();
        s.admit_cutover_freeze(f).unwrap();
        assert_eq!(s.admit_rehearsal(r), Err(TransitionError::RehearsalFrozenStateMismatch));
    }

    #[test]
    fn authority_binds_evidence_freeze_and_rehearsal() {
        let (mut s, c, e, f, r) = frozen_rehearsed();
        let a = authority(c, &e, f, r, 10, 100);
        s.admit_activation_authority(a, 50).unwrap();
        assert_eq!(s.phase(), ActivationPhase::ActivationAuthorityAdmitted);

        let wrong_freeze = CutoverFreezeReceipt::from_verified_adapter(eid(103), EvidenceState::Qualified, c.deployment(), legacy(150), 7, true);
        let wrong = authority(c, &e, wrong_freeze, r, 10, 100);
        let (mut s2, _, _, _, _) = frozen_rehearsed();
        assert_eq!(s2.admit_activation_authority(wrong, 50), Err(TransitionError::AuthorityFreezeMismatch));
    }

    #[test]
    fn authority_expiry_cannot_be_bypassed_by_clock_rollback() {
        let (mut s, c, e, f, r) = frozen_rehearsed();
        s.admit_activation_authority(authority(c, &e, f, r, 10, 100), 50).unwrap();
        assert_eq!(s.authorize_activation(100), Err(TransitionError::AuthorityExpired));
        assert_eq!(s.authorize_activation(99), Err(TransitionError::ClockRollback));
    }

    #[test]
    fn authority_is_rechecked_at_final_activation() {
        let (mut s, c, e, f, r) = frozen_rehearsed();
        s.admit_activation_authority(authority(c, &e, f, r, 10, 100), 50).unwrap();
        s.authorize_activation(60).unwrap();
        assert_eq!(s.activate_v2(100), Err(TransitionError::AuthorityExpired));
        assert_eq!(s.activate_v2(99), Err(TransitionError::ClockRollback));
    }

    #[test]
    fn successful_activation_is_monotonic_and_no_plaintext_fallback_exists() {
        let (mut s, c, e, f, r) = frozen_rehearsed();
        s.admit_activation_authority(authority(c, &e, f, r, 10, 100), 50).unwrap();
        s.authorize_activation(60).unwrap();
        s.activate_v2(70).unwrap();
        assert_eq!(write_target(&s), PatientWriteTarget::ProtectedV2);
        for failure in [ProtectedReadFailure::NotFound, ProtectedReadFailure::AuthorizationDenied, ProtectedReadFailure::AuthenticationFailed, ProtectedReadFailure::DecryptionFailed, ProtectedReadFailure::InvalidEnvelope, ProtectedReadFailure::LocatorUnavailable, ProtectedReadFailure::StaleCapability] {
            assert_eq!(resolve_read(&s, Err(failure)), ReadResolution::DenyNoLegacyFallback(failure));
        }
        s.retire_legacy_v1().unwrap();
        assert_eq!(s.phase(), ActivationPhase::LegacyV1ReadOnly);
    }

    #[test]
    fn state_machine_cannot_skip_evidence_freeze_rehearsal_or_authority() {
        let mut s = ActivationState::new();
        assert_eq!(s.activate_v2(1), Err(TransitionError::InvalidState));
        let c = candidate();
        s.prepare_candidate(c).unwrap();
        assert_eq!(s.admit_cutover_freeze(freeze(c)), Err(TransitionError::InvalidState));
        assert_eq!(s.authorize_activation(1), Err(TransitionError::InvalidState));
    }

    #[test]
    fn historical_exposure_ack_is_required_and_ids_redact() {
        let (mut s, c, e, f, r) = frozen_rehearsed();
        let mut a = authority(c, &e, f, r, 10, 100);
        a.historical_exposure_acknowledged = false;
        assert_eq!(s.admit_activation_authority(a, 50), Err(TransitionError::AuthorityMissingHistoricalExposureAcknowledgement));
        assert_eq!(EvidenceId::new([0; 32]), Err(OpaqueIdError::AllZero));
        assert_eq!(format!("{:?}", eid(1)), "EvidenceId([redacted])");
    }
}
