use core::fmt;

pub const COMPOSITION_CONTRACT_V1: u16 = 1;
const COMPOSITION_DOMAIN: &[u8] = b"mycelix-health/patient-v2-final-composition/v1\0";

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
            fn bytes(&self) -> &[u8; 32] { &self.0 }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    };
}

opaque32!(SourceCommit);
opaque32!(DnaManifestDigest);
opaque32!(IntegrityWasmSetDigest);
opaque32!(CoordinatorWasmSetDigest);
opaque32!(ToolchainDigest);
opaque32!(EvidenceSetDigest);
opaque32!(EvidenceId);
opaque32!(LegacyStateDigest);
opaque32!(MigrationPlanDigest);
opaque32!(RouteAccountingDigest);
opaque32!(DestinationRootDigest);
opaque32!(ProtectedManifestDigest);
opaque32!(MigrationBindingDigest);
opaque32!(AuthorityRef);
opaque32!(AuthorityBindingDigest);
opaque32!(CompositionDigest);
opaque32!(PrepareReceiptId);
opaque32!(ActivationReceiptId);

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
pub enum DeploymentIdentityError { ZeroPrivacyContractEpoch }

impl DeploymentIdentity {
    pub fn new(
        source_commit: SourceCommit,
        dna_manifest: DnaManifestDigest,
        integrity_wasm_set: IntegrityWasmSetDigest,
        coordinator_wasm_set: CoordinatorWasmSetDigest,
        toolchain: ToolchainDigest,
        privacy_contract_epoch: u64,
    ) -> Result<Self, DeploymentIdentityError> {
        if privacy_contract_epoch == 0 { return Err(DeploymentIdentityError::ZeroPrivacyContractEpoch); }
        Ok(Self { source_commit, dna_manifest, integrity_wasm_set, coordinator_wasm_set, toolchain, privacy_contract_epoch })
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutoverIdentity {
    deployment: DeploymentIdentity,
    freeze_evidence_id: EvidenceId,
    frozen_legacy_state: LegacyStateDigest,
    freeze_epoch: u64,
    rehearsal_evidence_id: EvidenceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoverIdentityError { ZeroFreezeEpoch, DuplicateEvidenceId }

impl CutoverIdentity {
    pub fn new(
        deployment: DeploymentIdentity,
        freeze_evidence_id: EvidenceId,
        frozen_legacy_state: LegacyStateDigest,
        freeze_epoch: u64,
        rehearsal_evidence_id: EvidenceId,
    ) -> Result<Self, CutoverIdentityError> {
        if freeze_epoch == 0 { return Err(CutoverIdentityError::ZeroFreezeEpoch); }
        if freeze_evidence_id == rehearsal_evidence_id { return Err(CutoverIdentityError::DuplicateEvidenceId); }
        Ok(Self { deployment, freeze_evidence_id, frozen_legacy_state, freeze_epoch, rehearsal_evidence_id })
    }
    pub fn deployment(&self) -> DeploymentIdentity { self.deployment }
    fn append_transcript(&self, out: &mut Vec<u8>) {
        self.deployment.append_transcript(out);
        out.extend_from_slice(self.freeze_evidence_id.bytes());
        out.extend_from_slice(self.frozen_legacy_state.bytes());
        out.extend_from_slice(&self.freeze_epoch.to_be_bytes());
        out.extend_from_slice(self.rehearsal_evidence_id.bytes());
    }
}

/// Adapter receipt for the exact qualified #204 side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationSideReceipt {
    cutover: CutoverIdentity,
    evidence_set: EvidenceSetDigest,
    contract_version: u16,
    qualified: bool,
}
impl ActivationSideReceipt {
    pub fn from_verified_adapter(cutover: CutoverIdentity, evidence_set: EvidenceSetDigest, contract_version: u16, qualified: bool) -> Self {
        Self { cutover, evidence_set, contract_version, qualified }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DestinationIdentity {
    destination_root: DestinationRootDigest,
    protected_manifest: ProtectedManifestDigest,
    key_epoch: u64,
    policy_epoch: u64,
    generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationIdentityError { ZeroKeyEpoch, ZeroPolicyEpoch, ZeroGeneration }
impl DestinationIdentity {
    pub fn new(destination_root: DestinationRootDigest, protected_manifest: ProtectedManifestDigest, key_epoch: u64, policy_epoch: u64, generation: u64) -> Result<Self, DestinationIdentityError> {
        if key_epoch == 0 { return Err(DestinationIdentityError::ZeroKeyEpoch); }
        if policy_epoch == 0 { return Err(DestinationIdentityError::ZeroPolicyEpoch); }
        if generation == 0 { return Err(DestinationIdentityError::ZeroGeneration); }
        Ok(Self { destination_root, protected_manifest, key_epoch, policy_epoch, generation })
    }
    fn append_transcript(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.destination_root.bytes());
        out.extend_from_slice(self.protected_manifest.bytes());
        out.extend_from_slice(&self.key_epoch.to_be_bytes());
        out.extend_from_slice(&self.policy_epoch.to_be_bytes());
        out.extend_from_slice(&self.generation.to_be_bytes());
    }
}

/// Adapter receipt for the strict latest-active #206 result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrationSideReceipt {
    cutover: CutoverIdentity,
    migration_evidence_id: EvidenceId,
    migration_plan: MigrationPlanDigest,
    route_accounting: RouteAccountingDigest,
    destination: DestinationIdentity,
    migration_binding: MigrationBindingDigest,
    contract_version: u16,
    qualified: bool,
    latest_active: bool,
    abandoned: bool,
}
#[allow(clippy::too_many_arguments)]
impl MigrationSideReceipt {
    pub fn from_verified_adapter(
        cutover: CutoverIdentity,
        migration_evidence_id: EvidenceId,
        migration_plan: MigrationPlanDigest,
        route_accounting: RouteAccountingDigest,
        destination: DestinationIdentity,
        migration_binding: MigrationBindingDigest,
        contract_version: u16,
        qualified: bool,
        latest_active: bool,
        abandoned: bool,
    ) -> Self {
        Self { cutover, migration_evidence_id, migration_plan, route_accounting, destination, migration_binding, contract_version, qualified, latest_active, abandoned }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FinalActivationAuthority {
    authority_ref: AuthorityRef,
    authority_binding: AuthorityBindingDigest,
    cutover: CutoverIdentity,
    evidence_set: EvidenceSetDigest,
    migration_binding: MigrationBindingDigest,
    destination: DestinationIdentity,
    issued_at_micros: i64,
    expires_at_micros: i64,
    historical_exposure_acknowledged: bool,
}
impl fmt::Debug for FinalActivationAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FinalActivationAuthority")
            .field("authority_ref", &self.authority_ref)
            .field("authority_binding", &self.authority_binding)
            .field("cutover", &self.cutover)
            .field("destination", &self.destination)
            .field("issued_at_micros", &self.issued_at_micros)
            .field("expires_at_micros", &self.expires_at_micros)
            .field("historical_exposure_acknowledged", &self.historical_exposure_acknowledged)
            .finish()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalAuthorityError { InvalidTimeWindow }
#[allow(clippy::too_many_arguments)]
impl FinalActivationAuthority {
    pub fn from_verified_adapter(
        authority_ref: AuthorityRef,
        authority_binding: AuthorityBindingDigest,
        cutover: CutoverIdentity,
        evidence_set: EvidenceSetDigest,
        migration_binding: MigrationBindingDigest,
        destination: DestinationIdentity,
        issued_at_micros: i64,
        expires_at_micros: i64,
        historical_exposure_acknowledged: bool,
    ) -> Result<Self, FinalAuthorityError> {
        if expires_at_micros <= issued_at_micros { return Err(FinalAuthorityError::InvalidTimeWindow); }
        Ok(Self { authority_ref, authority_binding, cutover, evidence_set, migration_binding, destination, issued_at_micros, expires_at_micros, historical_exposure_acknowledged })
    }
    fn valid_at(&self, now: i64) -> Result<(), CompositionError> {
        if now < self.issued_at_micros { return Err(CompositionError::AuthorityNotYetValid); }
        if now >= self.expires_at_micros { return Err(CompositionError::AuthorityExpired); }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionReceipt {
    cutover: CutoverIdentity,
    evidence_set: EvidenceSetDigest,
    migration_evidence_id: EvidenceId,
    migration_plan: MigrationPlanDigest,
    route_accounting: RouteAccountingDigest,
    migration_binding: MigrationBindingDigest,
    destination: DestinationIdentity,
    authority_ref: AuthorityRef,
    authority_binding: AuthorityBindingDigest,
    activation_contract_version: u16,
    migration_contract_version: u16,
}
impl CompositionReceipt {
    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(COMPOSITION_DOMAIN);
        out.extend_from_slice(&COMPOSITION_CONTRACT_V1.to_be_bytes());
        self.cutover.append_transcript(&mut out);
        out.extend_from_slice(self.evidence_set.bytes());
        out.extend_from_slice(self.migration_evidence_id.bytes());
        out.extend_from_slice(self.migration_plan.bytes());
        out.extend_from_slice(self.route_accounting.bytes());
        out.extend_from_slice(self.migration_binding.bytes());
        self.destination.append_transcript(&mut out);
        out.extend_from_slice(self.authority_ref.bytes());
        out.extend_from_slice(self.authority_binding.bytes());
        out.extend_from_slice(&self.activation_contract_version.to_be_bytes());
        out.extend_from_slice(&self.migration_contract_version.to_be_bytes());
        out.push(1); // historical-public-exposure acknowledgement is mandatory
        out
    }
    pub fn destination(&self) -> DestinationIdentity { self.destination }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionError {
    ActivationSourceNotQualified,
    MigrationSourceNotQualified,
    CutoverMismatch,
    MigrationNotLatest,
    MigrationAbandoned,
    AuthorityCutoverMismatch,
    AuthorityEvidenceMismatch,
    AuthorityMigrationMismatch,
    AuthorityDestinationMismatch,
    HistoricalExposureNotAcknowledged,
    AuthorityNotYetValid,
    AuthorityExpired,
    ClockRollback,
    InvalidState,
    CommitmentNotQualified,
    PrepareNotDurable,
    PrepareMismatch,
    CommitNotDurable,
    CommitMismatch,
    AuthoritySubstitution,
    ConflictingReplay,
}

pub fn compose(activation: ActivationSideReceipt, migration: MigrationSideReceipt, authority: &FinalActivationAuthority, now: i64) -> Result<CompositionReceipt, CompositionError> {
    if !activation.qualified { return Err(CompositionError::ActivationSourceNotQualified); }
    if !migration.qualified { return Err(CompositionError::MigrationSourceNotQualified); }
    if activation.cutover != migration.cutover { return Err(CompositionError::CutoverMismatch); }
    if !migration.latest_active { return Err(CompositionError::MigrationNotLatest); }
    if migration.abandoned { return Err(CompositionError::MigrationAbandoned); }
    if authority.cutover != activation.cutover { return Err(CompositionError::AuthorityCutoverMismatch); }
    if authority.evidence_set != activation.evidence_set { return Err(CompositionError::AuthorityEvidenceMismatch); }
    if authority.migration_binding != migration.migration_binding { return Err(CompositionError::AuthorityMigrationMismatch); }
    if authority.destination != migration.destination { return Err(CompositionError::AuthorityDestinationMismatch); }
    if !authority.historical_exposure_acknowledged { return Err(CompositionError::HistoricalExposureNotAcknowledged); }
    authority.valid_at(now)?;
    Ok(CompositionReceipt {
        cutover: activation.cutover,
        evidence_set: activation.evidence_set,
        migration_evidence_id: migration.migration_evidence_id,
        migration_plan: migration.migration_plan,
        route_accounting: migration.route_accounting,
        migration_binding: migration.migration_binding,
        destination: migration.destination,
        authority_ref: authority.authority_ref,
        authority_binding: authority.authority_binding,
        activation_contract_version: activation.contract_version,
        migration_contract_version: migration.contract_version,
    })
}

/// External hash/commitment adapter assertion over the canonical composition transcript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositionCommitmentReceipt {
    digest: CompositionDigest,
    qualified: bool,
}
impl CompositionCommitmentReceipt {
    pub fn from_verified_adapter(digest: CompositionDigest, qualified: bool) -> Self { Self { digest, qualified } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurablePrepareReceipt {
    receipt_id: PrepareReceiptId,
    composition_digest: CompositionDigest,
    activation_epoch: u64,
    durable: bool,
}
impl DurablePrepareReceipt {
    pub fn from_verified_adapter(receipt_id: PrepareReceiptId, composition_digest: CompositionDigest, activation_epoch: u64, durable: bool) -> Self {
        Self { receipt_id, composition_digest, activation_epoch, durable }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatientV2ActivationReceipt {
    receipt_id: ActivationReceiptId,
    prepare_receipt_id: PrepareReceiptId,
    composition_digest: CompositionDigest,
    activation_epoch: u64,
    cutover: CutoverIdentity,
    destination: DestinationIdentity,
    authority_ref: AuthorityRef,
    authority_binding: AuthorityBindingDigest,
    v2_discoverable: bool,
    legacy_v1_read_only: bool,
    durable: bool,
}
#[allow(clippy::too_many_arguments)]
impl PatientV2ActivationReceipt {
    pub fn from_verified_adapter(
        receipt_id: ActivationReceiptId,
        prepare_receipt_id: PrepareReceiptId,
        composition_digest: CompositionDigest,
        activation_epoch: u64,
        cutover: CutoverIdentity,
        destination: DestinationIdentity,
        authority_ref: AuthorityRef,
        authority_binding: AuthorityBindingDigest,
        v2_discoverable: bool,
        legacy_v1_read_only: bool,
        durable: bool,
    ) -> Self {
        Self { receipt_id, prepare_receipt_id, composition_digest, activation_epoch, cutover, destination, authority_ref, authority_binding, v2_discoverable, legacy_v1_read_only, durable }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitOutcome { Committed, IdempotentReplay }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalPhase { AwaitingComposition, CompositionAdmitted, ActivationCommitPrepared, V2Active }

#[derive(Clone, Debug)]
pub struct FinalCutoverCoordinator {
    phase: FinalPhase,
    composition: Option<CompositionReceipt>,
    commitment: Option<CompositionCommitmentReceipt>,
    prepare: Option<DurablePrepareReceipt>,
    activation: Option<PatientV2ActivationReceipt>,
    latest_time_micros: Option<i64>,
}
impl Default for FinalCutoverCoordinator { fn default() -> Self { Self::new() } }
impl FinalCutoverCoordinator {
    pub fn new() -> Self { Self { phase: FinalPhase::AwaitingComposition, composition: None, commitment: None, prepare: None, activation: None, latest_time_micros: None } }
    pub fn phase(&self) -> FinalPhase { self.phase }
    pub fn composition(&self) -> Option<&CompositionReceipt> { self.composition.as_ref() }
    pub fn activation_receipt(&self) -> Option<PatientV2ActivationReceipt> { self.activation }
    fn observe_time(&mut self, now: i64) -> Result<(), CompositionError> {
        if self.latest_time_micros.is_some_and(|previous| now < previous) { return Err(CompositionError::ClockRollback); }
        self.latest_time_micros = Some(now);
        Ok(())
    }
    pub fn admit_composition(
        &mut self,
        activation: ActivationSideReceipt,
        migration: MigrationSideReceipt,
        authority: &FinalActivationAuthority,
        commitment: CompositionCommitmentReceipt,
        now: i64,
    ) -> Result<(), CompositionError> {
        if self.phase != FinalPhase::AwaitingComposition { return Err(CompositionError::InvalidState); }
        self.observe_time(now)?;
        if !commitment.qualified { return Err(CompositionError::CommitmentNotQualified); }
        let receipt = compose(activation, migration, authority, now)?;
        self.composition = Some(receipt);
        self.commitment = Some(commitment);
        self.phase = FinalPhase::CompositionAdmitted;
        Ok(())
    }
    pub fn admit_prepare(&mut self, prepare: DurablePrepareReceipt) -> Result<(), CompositionError> {
        if self.phase != FinalPhase::CompositionAdmitted { return Err(CompositionError::InvalidState); }
        let expected = self.commitment.ok_or(CompositionError::InvalidState)?;
        if !prepare.durable { return Err(CompositionError::PrepareNotDurable); }
        if prepare.composition_digest != expected.digest || prepare.activation_epoch == 0 { return Err(CompositionError::PrepareMismatch); }
        self.prepare = Some(prepare);
        self.phase = FinalPhase::ActivationCommitPrepared;
        Ok(())
    }
    pub fn admit_activation(
        &mut self,
        receipt: PatientV2ActivationReceipt,
        authority: &FinalActivationAuthority,
        now: i64,
    ) -> Result<CommitOutcome, CompositionError> {
        if self.phase == FinalPhase::V2Active {
            return if self.activation == Some(receipt) { Ok(CommitOutcome::IdempotentReplay) } else { Err(CompositionError::ConflictingReplay) };
        }
        if self.phase != FinalPhase::ActivationCommitPrepared { return Err(CompositionError::InvalidState); }
        self.observe_time(now)?;
        authority.valid_at(now)?;
        let composition = self.composition.as_ref().ok_or(CompositionError::InvalidState)?;
        if authority.authority_ref != composition.authority_ref || authority.authority_binding != composition.authority_binding {
            return Err(CompositionError::AuthoritySubstitution);
        }
        let prepare = self.prepare.ok_or(CompositionError::InvalidState)?;
        let expected_digest = self.commitment.ok_or(CompositionError::InvalidState)?.digest;
        if !receipt.durable { return Err(CompositionError::CommitNotDurable); }
        if receipt.prepare_receipt_id != prepare.receipt_id
            || receipt.composition_digest != expected_digest
            || receipt.activation_epoch != prepare.activation_epoch
            || receipt.cutover != composition.cutover
            || receipt.destination != composition.destination
            || receipt.authority_ref != composition.authority_ref
            || receipt.authority_binding != composition.authority_binding
            || !receipt.v2_discoverable
            || !receipt.legacy_v1_read_only
        { return Err(CompositionError::CommitMismatch); }
        self.activation = Some(receipt);
        self.phase = FinalPhase::V2Active;
        Ok(CommitOutcome::Committed)
    }
    pub fn recover(
        composition: CompositionReceipt,
        commitment: CompositionCommitmentReceipt,
        prepare: Option<DurablePrepareReceipt>,
        activation: Option<PatientV2ActivationReceipt>,
    ) -> Result<Self, CompositionError> {
        if !commitment.qualified { return Err(CompositionError::CommitmentNotQualified); }
        match (prepare, activation) {
            (None, None) => Ok(Self { phase: FinalPhase::CompositionAdmitted, composition: Some(composition), commitment: Some(commitment), prepare: None, activation: None, latest_time_micros: None }),
            (Some(prepare), None) => {
                if !prepare.durable || prepare.composition_digest != commitment.digest || prepare.activation_epoch == 0 { return Err(CompositionError::PrepareMismatch); }
                Ok(Self { phase: FinalPhase::ActivationCommitPrepared, composition: Some(composition), commitment: Some(commitment), prepare: Some(prepare), activation: None, latest_time_micros: None })
            }
            (Some(prepare), Some(active)) => {
                if !prepare.durable
                    || !active.durable
                    || prepare.composition_digest != commitment.digest
                    || active.composition_digest != commitment.digest
                    || active.prepare_receipt_id != prepare.receipt_id
                    || active.activation_epoch != prepare.activation_epoch
                    || active.cutover != composition.cutover
                    || active.destination != composition.destination
                    || active.authority_ref != composition.authority_ref
                    || active.authority_binding != composition.authority_binding
                    || !active.v2_discoverable
                    || !active.legacy_v1_read_only
                { return Err(CompositionError::CommitMismatch); }
                Ok(Self { phase: FinalPhase::V2Active, composition: Some(composition), commitment: Some(commitment), prepare: Some(prepare), activation: Some(active), latest_time_micros: None })
            }
            (None, Some(_)) => Err(CompositionError::CommitMismatch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bytes(v: u8) -> [u8; 32] { [v; 32] }
    macro_rules! oid { ($t:ident, $v:expr) => { $t::new(bytes($v)).unwrap() }; }
    fn deployment(s: u8) -> DeploymentIdentity { DeploymentIdentity::new(oid!(SourceCommit,s),oid!(DnaManifestDigest,s+1),oid!(IntegrityWasmSetDigest,s+2),oid!(CoordinatorWasmSetDigest,s+3),oid!(ToolchainDigest,s+4),1).unwrap() }
    fn cutover(s: u8) -> CutoverIdentity { CutoverIdentity::new(deployment(s),oid!(EvidenceId,s+5),oid!(LegacyStateDigest,s+6),7,oid!(EvidenceId,s+7)).unwrap() }
    fn destination(s: u8) -> DestinationIdentity { DestinationIdentity::new(oid!(DestinationRootDigest,s),oid!(ProtectedManifestDigest,s+1),2,3,1).unwrap() }
    fn fixture(s: u8) -> (ActivationSideReceipt,MigrationSideReceipt,FinalActivationAuthority) {
        let c=cutover(s); let e=oid!(EvidenceSetDigest,s+8); let d=destination(s+20); let m=oid!(MigrationBindingDigest,s+30);
        let a=ActivationSideReceipt::from_verified_adapter(c,e,2,true);
        let g=MigrationSideReceipt::from_verified_adapter(c,oid!(EvidenceId,s+9),oid!(MigrationPlanDigest,s+10),oid!(RouteAccountingDigest,s+11),d,m,1,true,true,false);
        let auth=FinalActivationAuthority::from_verified_adapter(oid!(AuthorityRef,s+40),oid!(AuthorityBindingDigest,s+41),c,e,m,d,10,100,true).unwrap();
        (a,g,auth)
    }
    fn admitted() -> (FinalCutoverCoordinator, FinalActivationAuthority, CompositionDigest, DurablePrepareReceipt) {
        let (a,m,auth)=fixture(10); let digest=oid!(CompositionDigest,200); let commitment=CompositionCommitmentReceipt::from_verified_adapter(digest,true);
        let mut c=FinalCutoverCoordinator::new(); c.admit_composition(a,m,&auth,commitment,50).unwrap();
        let p=DurablePrepareReceipt::from_verified_adapter(oid!(PrepareReceiptId,201),digest,1,true); c.admit_prepare(p).unwrap(); (c,auth,digest,p)
    }
    #[test] fn mismatch_cutover_denies(){ let (a,mut m,auth)=fixture(10); m.cutover=cutover(50); assert_eq!(compose(a,m,&auth,50),Err(CompositionError::CutoverMismatch)); }
    #[test] fn stale_or_abandoned_denies(){ let (a,mut m,auth)=fixture(10); m.latest_active=false; assert_eq!(compose(a,m,&auth,50),Err(CompositionError::MigrationNotLatest)); let (a,mut m,auth)=fixture(10);m.abandoned=true;assert_eq!(compose(a,m,&auth,50),Err(CompositionError::MigrationAbandoned)); }
    #[test] fn final_authority_must_bind_exact_destination(){ let (a,m,mut auth)=fixture(10);auth.destination=destination(90);assert_eq!(compose(a,m,&auth,50),Err(CompositionError::AuthorityDestinationMismatch)); }
    #[test] fn commitment_must_be_qualified(){ let (a,m,auth)=fixture(10);let mut c=FinalCutoverCoordinator::new();let r=c.admit_composition(a,m,&auth,CompositionCommitmentReceipt::from_verified_adapter(oid!(CompositionDigest,200),false),50);assert_eq!(r,Err(CompositionError::CommitmentNotQualified)); }
    #[test] fn substituted_authority_denies_at_commit(){ let (mut c,_auth,d,p)=admitted(); let (a,m,mut other)=fixture(10); other.authority_ref=oid!(AuthorityRef,99); let _=(a,m); let receipt=PatientV2ActivationReceipt::from_verified_adapter(oid!(ActivationReceiptId,202),p.receipt_id,d,1,cutover(10),destination(30),oid!(AuthorityRef,50),oid!(AuthorityBindingDigest,51),true,true,true); assert_eq!(c.admit_activation(receipt,&other,60),Err(CompositionError::AuthoritySubstitution)); }
    #[test] fn commit_requires_durable_discoverable_read_only_exact_receipt(){ let (mut c,auth,d,p)=admitted(); let bad=PatientV2ActivationReceipt::from_verified_adapter(oid!(ActivationReceiptId,202),p.receipt_id,d,1,cutover(10),destination(30),oid!(AuthorityRef,50),oid!(AuthorityBindingDigest,51),false,true,true); assert_eq!(c.admit_activation(bad,&auth,60),Err(CompositionError::CommitMismatch)); }
    #[test] fn exact_commit_replay_is_idempotent_but_conflict_denies(){ let (mut c,auth,d,p)=admitted(); let r=PatientV2ActivationReceipt::from_verified_adapter(oid!(ActivationReceiptId,202),p.receipt_id,d,1,cutover(10),destination(30),oid!(AuthorityRef,50),oid!(AuthorityBindingDigest,51),true,true,true); assert_eq!(c.admit_activation(r,&auth,60),Ok(CommitOutcome::Committed)); assert_eq!(c.admit_activation(r,&auth,61),Ok(CommitOutcome::IdempotentReplay)); let conflict=PatientV2ActivationReceipt::from_verified_adapter(oid!(ActivationReceiptId,203),p.receipt_id,d,1,cutover(10),destination(30),oid!(AuthorityRef,50),oid!(AuthorityBindingDigest,51),true,true,true); assert_eq!(c.admit_activation(conflict,&auth,62),Err(CompositionError::ConflictingReplay)); }
    #[test] fn expired_then_earlier_retry_is_clock_rollback(){ let (mut c,auth,d,p)=admitted();let r=PatientV2ActivationReceipt::from_verified_adapter(oid!(ActivationReceiptId,202),p.receipt_id,d,1,cutover(10),destination(30),oid!(AuthorityRef,50),oid!(AuthorityBindingDigest,51),true,true,true); assert_eq!(c.admit_activation(r,&auth,100),Err(CompositionError::AuthorityExpired)); assert_eq!(c.admit_activation(r,&auth,99),Err(CompositionError::ClockRollback)); }
    #[test] fn crash_recovery_never_has_legacy_writable_phase(){ let (a,m,auth)=fixture(10);let comp=compose(a,m,&auth,50).unwrap();let d=oid!(CompositionDigest,200);let cm=CompositionCommitmentReceipt::from_verified_adapter(d,true);let p=DurablePrepareReceipt::from_verified_adapter(oid!(PrepareReceiptId,201),d,1,true);let prepared=FinalCutoverCoordinator::recover(comp.clone(),cm,Some(p),None).unwrap();assert_eq!(prepared.phase(),FinalPhase::ActivationCommitPrepared);let active=PatientV2ActivationReceipt::from_verified_adapter(oid!(ActivationReceiptId,202),p.receipt_id,d,1,cutover(10),destination(30),oid!(AuthorityRef,50),oid!(AuthorityBindingDigest,51),true,true,true);let recovered=FinalCutoverCoordinator::recover(comp,cm,Some(p),Some(active)).unwrap();assert_eq!(recovered.phase(),FinalPhase::V2Active); }
    #[test] fn orphan_or_mismatched_commit_recovery_denies(){ let (a,m,auth)=fixture(10);let comp=compose(a,m,&auth,50).unwrap();let d=oid!(CompositionDigest,200);let cm=CompositionCommitmentReceipt::from_verified_adapter(d,true);let active=PatientV2ActivationReceipt::from_verified_adapter(oid!(ActivationReceiptId,202),oid!(PrepareReceiptId,201),d,1,cutover(10),destination(30),oid!(AuthorityRef,50),oid!(AuthorityBindingDigest,51),true,true,true);assert!(matches!(FinalCutoverCoordinator::recover(comp,cm,None,Some(active)),Err(CompositionError::CommitMismatch))); }
    #[test] fn composition_transcript_changes_with_destination(){ let (a,m,auth)=fixture(10);let x=compose(a,m,&auth,50).unwrap();let (a2,mut m2,mut auth2)=fixture(10);let nd=destination(90);m2.destination=nd;auth2.destination=nd;let y=compose(a2,m2,&auth2,50).unwrap();assert_ne!(x.transcript_bytes(),y.transcript_bytes()); }
    #[test] fn zero_ids_refused(){ assert_eq!(EvidenceId::new([0;32]),Err(OpaqueIdError::AllZero)); }
}
