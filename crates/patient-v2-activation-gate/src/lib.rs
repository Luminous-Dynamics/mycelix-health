#![forbid(unsafe_code)]
//! Fail-closed reference semantics for Patient v2 activation qualification.
//!
//! This crate does not inspect GitHub, run CI, verify signatures, hash WASM, or
//! activate a Holochain DNA. It freezes the evidence-aggregation and activation
//! state machine so individually valid reference results cannot silently become
//! deployment authority.

use core::fmt;

pub const ACTIVATION_CONTRACT_V1: u16 = 1;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeploymentIdentity {
    pub source_commit: SourceCommit,
    pub dna_manifest: DnaManifestDigest,
    pub integrity_wasm_set: IntegrityWasmSetDigest,
    pub coordinator_wasm_set: CoordinatorWasmSetDigest,
    pub toolchain: ToolchainDigest,
    pub privacy_contract_epoch: u64,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractId {
    PatientProfileMigration,
    ProtectedEnvelope,
    VaultSession,
    CareCapability,
    CareDisclosure,
    LinkTopologyInventory,
    OpaqueLocator,
    AuditChain,
    SecurityMonitoring,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractRef {
    pub id: ContractId,
    pub version: u16,
}

pub const PATIENT_PROFILE_MIGRATION_V2: ContractRef = ContractRef {
    id: ContractId::PatientProfileMigration,
    version: 2,
};
pub const PROTECTED_ENVELOPE_V2: ContractRef = ContractRef {
    id: ContractId::ProtectedEnvelope,
    version: 2,
};
pub const VAULT_SESSION_V1: ContractRef = ContractRef {
    id: ContractId::VaultSession,
    version: 1,
};
pub const CARE_CAPABILITY_V1: ContractRef = ContractRef {
    id: ContractId::CareCapability,
    version: 1,
};
pub const CARE_DISCLOSURE_V1: ContractRef = ContractRef {
    id: ContractId::CareDisclosure,
    version: 1,
};
pub const LINK_TOPOLOGY_INVENTORY_V1: ContractRef = ContractRef {
    id: ContractId::LinkTopologyInventory,
    version: 1,
};
pub const OPAQUE_LOCATOR_V1: ContractRef = ContractRef {
    id: ContractId::OpaqueLocator,
    version: 1,
};
pub const AUDIT_CHAIN_V1: ContractRef = ContractRef {
    id: ContractId::AuditChain,
    version: 1,
};
pub const SECURITY_MONITORING_V1: ContractRef = ContractRef {
    id: ContractId::SecurityMonitoring,
    version: 1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProofLine {
    PatientRoutingReference,
    EnvelopeReference,
    EnvelopeProductionAdapter,
    VaultSessionReference,
    VaultSessionIntegration,
    CareCapabilityReference,
    CapabilityProductionAdapter,
    CareDisclosureReference,
    EntryStorageMigration,
    LinkTopologyClassification,
    LinkConstructionClassification,
    LocatorReference,
    LocatorIntegration,
    IdentitySummaryIntegration,
    AuditReference,
    AuditIntegration,
    MonitoringReference,
    MultiAgentConfidentiality,
    P0Closure,
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

pub const REQUIRED_PROOFS: [ProofRequirement; 19] = [
    ProofRequirement {
        line: ProofLine::PatientRoutingReference,
        scope: RequirementScope::Reference(PATIENT_PROFILE_MIGRATION_V2),
    },
    ProofRequirement {
        line: ProofLine::EnvelopeReference,
        scope: RequirementScope::Reference(PROTECTED_ENVELOPE_V2),
    },
    ProofRequirement {
        line: ProofLine::EnvelopeProductionAdapter,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::VaultSessionReference,
        scope: RequirementScope::Reference(VAULT_SESSION_V1),
    },
    ProofRequirement {
        line: ProofLine::VaultSessionIntegration,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::CareCapabilityReference,
        scope: RequirementScope::Reference(CARE_CAPABILITY_V1),
    },
    ProofRequirement {
        line: ProofLine::CapabilityProductionAdapter,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::CareDisclosureReference,
        scope: RequirementScope::Reference(CARE_DISCLOSURE_V1),
    },
    ProofRequirement {
        line: ProofLine::EntryStorageMigration,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::LinkTopologyClassification,
        scope: RequirementScope::Reference(LINK_TOPOLOGY_INVENTORY_V1),
    },
    ProofRequirement {
        line: ProofLine::LinkConstructionClassification,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::LocatorReference,
        scope: RequirementScope::Reference(OPAQUE_LOCATOR_V1),
    },
    ProofRequirement {
        line: ProofLine::LocatorIntegration,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::IdentitySummaryIntegration,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::AuditReference,
        scope: RequirementScope::Reference(AUDIT_CHAIN_V1),
    },
    ProofRequirement {
        line: ProofLine::AuditIntegration,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::MonitoringReference,
        scope: RequirementScope::Reference(SECURITY_MONITORING_V1),
    },
    ProofRequirement {
        line: ProofLine::MultiAgentConfidentiality,
        scope: RequirementScope::Deployment,
    },
    ProofRequirement {
        line: ProofLine::P0Closure,
        scope: RequirementScope::Deployment,
    },
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
    pub line: ProofLine,
    pub evidence_id: EvidenceId,
    pub state: EvidenceState,
    pub scope: EvidenceScope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationCandidate {
    pub deployment: DeploymentIdentity,
    /// Fixed true for migration from the known public v1 lineage. The API has no
    /// constructor parameter that can claim prior public observations were recalled.
    pub legacy_public_exposure_may_persist: bool,
    pub contract_version: u16,
}

impl ActivationCandidate {
    pub fn new(deployment: DeploymentIdentity) -> Self {
        Self {
            deployment,
            legacy_public_exposure_may_persist: true,
            contract_version: ACTIVATION_CONTRACT_V1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceSetReceipt {
    pub deployment: DeploymentIdentity,
    evidence_ids: Vec<(ProofLine, EvidenceId)>,
    pub legacy_public_exposure_may_persist: bool,
    pub contract_version: u16,
}

impl EvidenceSetReceipt {
    pub fn evidence_ids(&self) -> &[(ProofLine, EvidenceId)] {
        &self.evidence_ids
    }

    /// Canonical structural transcript for a separately qualified hash/signature
    /// adapter. No cryptographic claim is made by this dependency-free core.
    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"mycelix-health/patient-v2-evidence-set/v1\0");
        out.extend_from_slice(&self.contract_version.to_be_bytes());
        out.extend_from_slice(&self.deployment.privacy_contract_epoch.to_be_bytes());
        out.extend_from_slice(&(self.evidence_ids.len() as u64).to_be_bytes());
        for (line, id) in &self.evidence_ids {
            out.push(*line as u8);
            out.extend_from_slice(&id.0);
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
    // Reject duplicate evidence IDs even across different proof lines. A single
    // artifact may be cited by higher-level documentation, but this gate requires
    // each mandatory theorem to have an independently named evidence receipt.
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
        let matching: Vec<&EvidenceReceipt> = receipts
            .iter()
            .filter(|receipt| receipt.line == requirement.line)
            .collect();
        match matching.len() {
            0 => return Err(EvidenceGateError::MissingProof(requirement.line)),
            1 => {}
            _ => return Err(EvidenceGateError::DuplicateProof(requirement.line)),
        }
        let receipt = matching[0];
        if receipt.state != EvidenceState::Qualified {
            return Err(EvidenceGateError::NotQualified(
                requirement.line,
                receipt.state,
            ));
        }
        match (requirement.scope, receipt.scope) {
            (RequirementScope::Reference(expected), EvidenceScope::Reference(actual)) => {
                if expected != actual {
                    return Err(EvidenceGateError::ReferenceContractMismatch(
                        requirement.line,
                    ));
                }
            }
            (RequirementScope::Deployment, EvidenceScope::Deployment(actual)) => {
                if actual != candidate.deployment {
                    return Err(EvidenceGateError::DeploymentMismatch(requirement.line));
                }
            }
            _ => return Err(EvidenceGateError::ScopeKindMismatch(requirement.line)),
        }
        admitted.push((requirement.line, receipt.evidence_id));
    }

    admitted.sort_by_key(|(line, _)| *line);
    Ok(EvidenceSetReceipt {
        deployment: candidate.deployment,
        evidence_ids: admitted,
        legacy_public_exposure_may_persist: true,
        contract_version: ACTIVATION_CONTRACT_V1,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrationRehearsalReceipt {
    pub evidence_id: EvidenceId,
    pub state: EvidenceState,
    pub deployment: DeploymentIdentity,
    pub idempotency_verified: bool,
    pub conflict_detection_verified: bool,
    pub no_plaintext_fallback_verified: bool,
    pub unresolved_routes_accounted_for: bool,
    pub legacy_public_exposure_may_persist: bool,
}

impl MigrationRehearsalReceipt {
    pub fn qualified(evidence_id: EvidenceId, deployment: DeploymentIdentity) -> Self {
        Self {
            evidence_id,
            state: EvidenceState::Qualified,
            deployment,
            idempotency_verified: true,
            conflict_detection_verified: true,
            no_plaintext_fallback_verified: true,
            unresolved_routes_accounted_for: true,
            legacy_public_exposure_may_persist: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RehearsalError {
    NotQualified,
    DeploymentMismatch,
    IncompleteRehearsal,
    DuplicateEvidenceId,
}

fn validate_rehearsal(
    evidence: &EvidenceSetReceipt,
    rehearsal: &MigrationRehearsalReceipt,
) -> Result<(), RehearsalError> {
    if rehearsal.state != EvidenceState::Qualified {
        return Err(RehearsalError::NotQualified);
    }
    if rehearsal.deployment != evidence.deployment {
        return Err(RehearsalError::DeploymentMismatch);
    }
    if evidence
        .evidence_ids
        .iter()
        .any(|(_, existing)| *existing == rehearsal.evidence_id)
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

/// Opaque already-verified activation-authority artifact. Signature/governance
/// verification happens in a separate adapter. This type binds exact technical
/// evidence rather than granting authority from CI state itself.
#[derive(Clone, PartialEq, Eq)]
pub struct ActivationAuthorityReceipt {
    pub authority_ref: AuthorityRef,
    pub deployment: DeploymentIdentity,
    pub evidence_ids: Vec<EvidenceId>,
    pub issued_at_micros: i64,
    pub expires_at_micros: i64,
    pub legacy_public_exposure_acknowledged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityReceiptError {
    InvalidTimeWindow,
    EmptyEvidenceSet,
    DuplicateEvidenceId,
}

impl ActivationAuthorityReceipt {
    pub fn new(
        authority_ref: AuthorityRef,
        deployment: DeploymentIdentity,
        mut evidence_ids: Vec<EvidenceId>,
        issued_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, AuthorityReceiptError> {
        if expires_at_micros <= issued_at_micros {
            return Err(AuthorityReceiptError::InvalidTimeWindow);
        }
        if evidence_ids.is_empty() {
            return Err(AuthorityReceiptError::EmptyEvidenceSet);
        }
        evidence_ids.sort();
        if evidence_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AuthorityReceiptError::DuplicateEvidenceId);
        }
        Ok(Self {
            authority_ref,
            deployment,
            evidence_ids,
            issued_at_micros,
            expires_at_micros,
            legacy_public_exposure_acknowledged: true,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivationState {
    LegacyV1Writable,
    CandidatePrepared {
        candidate: ActivationCandidate,
    },
    EvidenceComplete {
        candidate: ActivationCandidate,
        evidence: EvidenceSetReceipt,
    },
    MigrationRehearsed {
        candidate: ActivationCandidate,
        evidence: EvidenceSetReceipt,
        rehearsal: MigrationRehearsalReceipt,
    },
    ActivationAuthorityAdmitted {
        candidate: ActivationCandidate,
        evidence: EvidenceSetReceipt,
        rehearsal: MigrationRehearsalReceipt,
        authority: ActivationAuthorityReceipt,
    },
    V2ActivationAuthorized {
        candidate: ActivationCandidate,
        evidence: EvidenceSetReceipt,
        rehearsal: MigrationRehearsalReceipt,
        authority: ActivationAuthorityReceipt,
    },
    V2Active {
        deployment: DeploymentIdentity,
    },
    LegacyV1ReadOnly {
        deployment: DeploymentIdentity,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionError {
    InvalidState,
    CandidateMismatch,
    RehearsalNotQualified,
    RehearsalDeploymentMismatch,
    RehearsalIncomplete,
    DuplicateRehearsalEvidenceId,
    AuthorityDeploymentMismatch,
    AuthorityEvidenceMismatch,
    AuthorityNotYetValid,
    AuthorityExpired,
    AuthorityMissingHistoricalExposureAcknowledgement,
}

pub fn prepare_candidate(
    state: &mut ActivationState,
    candidate: ActivationCandidate,
) -> Result<(), TransitionError> {
    match state {
        ActivationState::LegacyV1Writable => {
            *state = ActivationState::CandidatePrepared { candidate };
            Ok(())
        }
        _ => Err(TransitionError::InvalidState),
    }
}

pub fn admit_evidence(
    state: &mut ActivationState,
    evidence: EvidenceSetReceipt,
) -> Result<(), TransitionError> {
    let candidate = match state {
        ActivationState::CandidatePrepared { candidate } => *candidate,
        _ => return Err(TransitionError::InvalidState),
    };
    if candidate.deployment != evidence.deployment {
        return Err(TransitionError::CandidateMismatch);
    }
    *state = ActivationState::EvidenceComplete {
        candidate,
        evidence,
    };
    Ok(())
}

pub fn admit_rehearsal(
    state: &mut ActivationState,
    rehearsal: MigrationRehearsalReceipt,
) -> Result<(), TransitionError> {
    let (candidate, evidence) = match state {
        ActivationState::EvidenceComplete {
            candidate,
            evidence,
        } => (*candidate, evidence.clone()),
        _ => return Err(TransitionError::InvalidState),
    };
    match validate_rehearsal(&evidence, &rehearsal) {
        Ok(()) => {}
        Err(RehearsalError::NotQualified) => return Err(TransitionError::RehearsalNotQualified),
        Err(RehearsalError::DeploymentMismatch) => {
            return Err(TransitionError::RehearsalDeploymentMismatch)
        }
        Err(RehearsalError::IncompleteRehearsal) => {
            return Err(TransitionError::RehearsalIncomplete)
        }
        Err(RehearsalError::DuplicateEvidenceId) => {
            return Err(TransitionError::DuplicateRehearsalEvidenceId)
        }
    }
    *state = ActivationState::MigrationRehearsed {
        candidate,
        evidence,
        rehearsal,
    };
    Ok(())
}

fn exact_authority_evidence_ids(
    evidence: &EvidenceSetReceipt,
    rehearsal: &MigrationRehearsalReceipt,
) -> Vec<EvidenceId> {
    let mut ids: Vec<EvidenceId> = evidence.evidence_ids.iter().map(|(_, id)| *id).collect();
    ids.push(rehearsal.evidence_id);
    ids.sort();
    ids
}

pub fn admit_activation_authority(
    state: &mut ActivationState,
    authority: ActivationAuthorityReceipt,
    now_micros: i64,
) -> Result<(), TransitionError> {
    let (candidate, evidence, rehearsal) = match state {
        ActivationState::MigrationRehearsed {
            candidate,
            evidence,
            rehearsal,
        } => (*candidate, evidence.clone(), *rehearsal),
        _ => return Err(TransitionError::InvalidState),
    };
    if authority.deployment != candidate.deployment {
        return Err(TransitionError::AuthorityDeploymentMismatch);
    }
    if authority.evidence_ids != exact_authority_evidence_ids(&evidence, &rehearsal) {
        return Err(TransitionError::AuthorityEvidenceMismatch);
    }
    if now_micros < authority.issued_at_micros {
        return Err(TransitionError::AuthorityNotYetValid);
    }
    if now_micros >= authority.expires_at_micros {
        return Err(TransitionError::AuthorityExpired);
    }
    if !authority.legacy_public_exposure_acknowledged {
        return Err(TransitionError::AuthorityMissingHistoricalExposureAcknowledgement);
    }
    *state = ActivationState::ActivationAuthorityAdmitted {
        candidate,
        evidence,
        rehearsal,
        authority,
    };
    Ok(())
}

pub fn authorize_activation(state: &mut ActivationState) -> Result<(), TransitionError> {
    let (candidate, evidence, rehearsal, authority) = match state {
        ActivationState::ActivationAuthorityAdmitted {
            candidate,
            evidence,
            rehearsal,
            authority,
        } => (*candidate, evidence.clone(), *rehearsal, authority.clone()),
        _ => return Err(TransitionError::InvalidState),
    };
    *state = ActivationState::V2ActivationAuthorized {
        candidate,
        evidence,
        rehearsal,
        authority,
    };
    Ok(())
}

pub fn activate_v2(state: &mut ActivationState) -> Result<(), TransitionError> {
    let deployment = match state {
        ActivationState::V2ActivationAuthorized { candidate, .. } => candidate.deployment,
        _ => return Err(TransitionError::InvalidState),
    };
    *state = ActivationState::V2Active { deployment };
    Ok(())
}

pub fn retire_legacy_v1(state: &mut ActivationState) -> Result<(), TransitionError> {
    let deployment = match state {
        ActivationState::V2Active { deployment } => *deployment,
        _ => return Err(TransitionError::InvalidState),
    };
    *state = ActivationState::LegacyV1ReadOnly { deployment };
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatientWriteTarget {
    LegacyV1,
    ProtectedV2,
}

pub fn write_target(state: &ActivationState) -> PatientWriteTarget {
    match state {
        ActivationState::V2Active { .. } | ActivationState::LegacyV1ReadOnly { .. } => {
            PatientWriteTarget::ProtectedV2
        }
        _ => PatientWriteTarget::LegacyV1,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatientReadTarget {
    LegacyV1,
    ProtectedV2Required,
}

pub fn read_target(state: &ActivationState) -> PatientReadTarget {
    match state {
        ActivationState::V2Active { .. } | ActivationState::LegacyV1ReadOnly { .. } => {
            PatientReadTarget::ProtectedV2Required
        }
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

    fn bytes(value: u8) -> [u8; 32] {
        [value; 32]
    }
    fn evidence_id(value: u8) -> EvidenceId {
        EvidenceId::new(bytes(value)).unwrap()
    }
    fn deployment(seed: u8) -> DeploymentIdentity {
        DeploymentIdentity::new(
            SourceCommit::new(bytes(seed)).unwrap(),
            DnaManifestDigest::new(bytes(seed + 1)).unwrap(),
            IntegrityWasmSetDigest::new(bytes(seed + 2)).unwrap(),
            CoordinatorWasmSetDigest::new(bytes(seed + 3)).unwrap(),
            ToolchainDigest::new(bytes(seed + 4)).unwrap(),
            1,
        )
        .unwrap()
    }

    fn qualified_receipts(candidate: ActivationCandidate) -> Vec<EvidenceReceipt> {
        REQUIRED_PROOFS
            .iter()
            .enumerate()
            .map(|(index, requirement)| EvidenceReceipt {
                line: requirement.line,
                evidence_id: evidence_id((index + 1) as u8),
                state: EvidenceState::Qualified,
                scope: match requirement.scope {
                    RequirementScope::Reference(contract) => EvidenceScope::Reference(contract),
                    RequirementScope::Deployment => EvidenceScope::Deployment(candidate.deployment),
                },
            })
            .collect()
    }

    fn complete_state() -> (ActivationCandidate, EvidenceSetReceipt) {
        let candidate = ActivationCandidate::new(deployment(20));
        let receipts = qualified_receipts(candidate);
        let evidence = evaluate_evidence(candidate, &receipts).unwrap();
        (candidate, evidence)
    }

    #[test]
    fn missing_required_proof_denies() {
        let candidate = ActivationCandidate::new(deployment(20));
        let mut receipts = qualified_receipts(candidate);
        receipts.pop();
        assert!(matches!(
            evaluate_evidence(candidate, &receipts),
            Err(EvidenceGateError::MissingProof(_))
        ));
    }

    #[test]
    fn source_staged_and_queued_are_not_qualified() {
        let candidate = ActivationCandidate::new(deployment(20));
        for state in [
            EvidenceState::SourceStaged,
            EvidenceState::QueuedInfrastructure,
            EvidenceState::Failed,
            EvidenceState::Stale,
            EvidenceState::Missing,
            EvidenceState::Superseded,
        ] {
            let mut receipts = qualified_receipts(candidate);
            receipts[0].state = state;
            assert!(matches!(
                evaluate_evidence(candidate, &receipts),
                Err(EvidenceGateError::NotQualified(_, observed)) if observed == state
            ));
        }
    }

    #[test]
    fn deployment_bound_proof_must_match_exact_candidate() {
        let candidate = ActivationCandidate::new(deployment(20));
        let mut receipts = qualified_receipts(candidate);
        let index = REQUIRED_PROOFS
            .iter()
            .position(|r| r.line == ProofLine::EnvelopeProductionAdapter)
            .unwrap();
        receipts[index].scope = EvidenceScope::Deployment(deployment(40));
        assert_eq!(
            evaluate_evidence(candidate, &receipts),
            Err(EvidenceGateError::DeploymentMismatch(
                ProofLine::EnvelopeProductionAdapter
            ))
        );
    }

    #[test]
    fn reference_contract_version_must_match() {
        let candidate = ActivationCandidate::new(deployment(20));
        let mut receipts = qualified_receipts(candidate);
        let index = REQUIRED_PROOFS
            .iter()
            .position(|r| r.line == ProofLine::LocatorReference)
            .unwrap();
        receipts[index].scope = EvidenceScope::Reference(ContractRef {
            id: ContractId::OpaqueLocator,
            version: 2,
        });
        assert_eq!(
            evaluate_evidence(candidate, &receipts),
            Err(EvidenceGateError::ReferenceContractMismatch(
                ProofLine::LocatorReference
            ))
        );
    }

    #[test]
    fn duplicate_proof_and_reused_evidence_id_deny() {
        let candidate = ActivationCandidate::new(deployment(20));
        let mut duplicate_line = qualified_receipts(candidate);
        duplicate_line.push(duplicate_line[0]);
        // Duplicate evidence ID is even stronger and is checked first.
        assert_eq!(
            evaluate_evidence(candidate, &duplicate_line),
            Err(EvidenceGateError::DuplicateEvidenceId)
        );

        let mut duplicate_proof = qualified_receipts(candidate);
        let mut extra = duplicate_proof[0];
        extra.evidence_id = evidence_id(99);
        duplicate_proof.push(extra);
        assert_eq!(
            evaluate_evidence(candidate, &duplicate_proof),
            Err(EvidenceGateError::DuplicateProof(extra.line))
        );
    }

    #[test]
    fn coherent_all_qualified_evidence_is_admitted() {
        let candidate = ActivationCandidate::new(deployment(20));
        let receipts = qualified_receipts(candidate);
        let evidence = evaluate_evidence(candidate, &receipts).unwrap();
        assert_eq!(evidence.evidence_ids().len(), REQUIRED_PROOFS.len());
        assert!(evidence.legacy_public_exposure_may_persist);
        assert!(!evidence.transcript_bytes().is_empty());
    }

    #[test]
    fn candidate_always_acknowledges_historical_exposure() {
        let candidate = ActivationCandidate::new(deployment(20));
        assert!(candidate.legacy_public_exposure_may_persist);
    }

    #[test]
    fn rehearsal_requires_qualified_exact_deployment_and_complete_flags() {
        let (_, evidence) = complete_state();
        let mut rehearsal = MigrationRehearsalReceipt::qualified(evidence_id(100), evidence.deployment);
        assert_eq!(validate_rehearsal(&evidence, &rehearsal), Ok(()));
        rehearsal.state = EvidenceState::QueuedInfrastructure;
        assert_eq!(
            validate_rehearsal(&evidence, &rehearsal),
            Err(RehearsalError::NotQualified)
        );
        rehearsal = MigrationRehearsalReceipt::qualified(evidence_id(100), deployment(40));
        assert_eq!(
            validate_rehearsal(&evidence, &rehearsal),
            Err(RehearsalError::DeploymentMismatch)
        );
        rehearsal = MigrationRehearsalReceipt::qualified(evidence_id(100), evidence.deployment);
        rehearsal.no_plaintext_fallback_verified = false;
        assert_eq!(
            validate_rehearsal(&evidence, &rehearsal),
            Err(RehearsalError::IncompleteRehearsal)
        );
    }

    #[test]
    fn activation_authority_must_bind_exact_evidence_set() {
        let (candidate, evidence) = complete_state();
        let rehearsal = MigrationRehearsalReceipt::qualified(evidence_id(100), candidate.deployment);
        let mut expected = exact_authority_evidence_ids(&evidence, &rehearsal);
        let authority = ActivationAuthorityReceipt::new(
            AuthorityRef::new(bytes(200)).unwrap(),
            candidate.deployment,
            expected.clone(),
            10,
            100,
        )
        .unwrap();
        assert_eq!(authority.evidence_ids, expected);
        expected.pop();
        let wrong = ActivationAuthorityReceipt::new(
            AuthorityRef::new(bytes(201)).unwrap(),
            candidate.deployment,
            expected,
            10,
            100,
        )
        .unwrap();
        let mut state = ActivationState::MigrationRehearsed {
            candidate,
            evidence,
            rehearsal,
        };
        assert_eq!(
            admit_activation_authority(&mut state, wrong, 50),
            Err(TransitionError::AuthorityEvidenceMismatch)
        );
    }

    #[test]
    fn activation_authority_is_time_and_deployment_bound() {
        let (candidate, evidence) = complete_state();
        let rehearsal = MigrationRehearsalReceipt::qualified(evidence_id(100), candidate.deployment);
        let ids = exact_authority_evidence_ids(&evidence, &rehearsal);
        let authority = ActivationAuthorityReceipt::new(
            AuthorityRef::new(bytes(200)).unwrap(),
            candidate.deployment,
            ids.clone(),
            10,
            100,
        )
        .unwrap();
        let mut early = ActivationState::MigrationRehearsed {
            candidate,
            evidence: evidence.clone(),
            rehearsal,
        };
        assert_eq!(
            admit_activation_authority(&mut early, authority.clone(), 9),
            Err(TransitionError::AuthorityNotYetValid)
        );
        let mut expired = ActivationState::MigrationRehearsed {
            candidate,
            evidence: evidence.clone(),
            rehearsal,
        };
        assert_eq!(
            admit_activation_authority(&mut expired, authority, 100),
            Err(TransitionError::AuthorityExpired)
        );
        let wrong_deployment = ActivationAuthorityReceipt::new(
            AuthorityRef::new(bytes(202)).unwrap(),
            deployment(40),
            ids,
            10,
            100,
        )
        .unwrap();
        let mut wrong = ActivationState::MigrationRehearsed {
            candidate,
            evidence,
            rehearsal,
        };
        assert_eq!(
            admit_activation_authority(&mut wrong, wrong_deployment, 50),
            Err(TransitionError::AuthorityDeploymentMismatch)
        );
    }

    #[test]
    fn activation_state_machine_is_ordered_and_fail_closed() {
        let candidate = ActivationCandidate::new(deployment(20));
        let evidence = evaluate_evidence(candidate, &qualified_receipts(candidate)).unwrap();
        let rehearsal = MigrationRehearsalReceipt::qualified(evidence_id(100), candidate.deployment);
        let authority = ActivationAuthorityReceipt::new(
            AuthorityRef::new(bytes(200)).unwrap(),
            candidate.deployment,
            exact_authority_evidence_ids(&evidence, &rehearsal),
            10,
            100,
        )
        .unwrap();

        let mut state = ActivationState::LegacyV1Writable;
        assert_eq!(activate_v2(&mut state), Err(TransitionError::InvalidState));
        prepare_candidate(&mut state, candidate).unwrap();
        assert_eq!(activate_v2(&mut state), Err(TransitionError::InvalidState));
        admit_evidence(&mut state, evidence).unwrap();
        assert_eq!(authorize_activation(&mut state), Err(TransitionError::InvalidState));
        admit_rehearsal(&mut state, rehearsal).unwrap();
        admit_activation_authority(&mut state, authority, 50).unwrap();
        authorize_activation(&mut state).unwrap();
        activate_v2(&mut state).unwrap();
        assert!(matches!(state, ActivationState::V2Active { .. }));
        retire_legacy_v1(&mut state).unwrap();
        assert!(matches!(state, ActivationState::LegacyV1ReadOnly { .. }));
    }

    #[test]
    fn writes_switch_only_after_actual_v2_activation() {
        let candidate = ActivationCandidate::new(deployment(20));
        let states = [
            ActivationState::LegacyV1Writable,
            ActivationState::CandidatePrepared { candidate },
        ];
        for state in states {
            assert_eq!(write_target(&state), PatientWriteTarget::LegacyV1);
        }
        assert_eq!(
            write_target(&ActivationState::V2Active {
                deployment: candidate.deployment,
            }),
            PatientWriteTarget::ProtectedV2
        );
        assert_eq!(
            write_target(&ActivationState::LegacyV1ReadOnly {
                deployment: candidate.deployment,
            }),
            PatientWriteTarget::ProtectedV2
        );
    }

    #[test]
    fn protected_read_failure_never_falls_back_after_activation() {
        let deployment = deployment(20);
        let state = ActivationState::V2Active { deployment };
        for failure in [
            ProtectedReadFailure::NotFound,
            ProtectedReadFailure::AuthorizationDenied,
            ProtectedReadFailure::AuthenticationFailed,
            ProtectedReadFailure::DecryptionFailed,
            ProtectedReadFailure::InvalidEnvelope,
            ProtectedReadFailure::LocatorUnavailable,
            ProtectedReadFailure::StaleCapability,
        ] {
            assert_eq!(
                resolve_read(&state, Err(failure)),
                ReadResolution::DenyNoLegacyFallback(failure)
            );
        }
    }

    #[test]
    fn opaque_identifiers_reject_zero_and_redact_debug() {
        assert_eq!(EvidenceId::new([0; 32]), Err(OpaqueIdError::AllZero));
        assert_eq!(format!("{:?}", evidence_id(1)), "EvidenceId([redacted])");
        assert_eq!(
            format!("{:?}", AuthorityRef::new(bytes(2)).unwrap()),
            "AuthorityRef([redacted])"
        );
    }
}
