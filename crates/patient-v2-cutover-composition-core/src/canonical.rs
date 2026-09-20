use core::fmt;

pub const COMPOSITION_CONTRACT_V1: u16 = 1;
pub const REQUIRED_ACTIVATION_CONTRACT_V2: u16 = 2;
pub const REQUIRED_MIGRATION_CONTRACT_V1: u16 = 1;
const DOMAIN: &[u8] = b"mycelix-health/patient-v2-final-composition/v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueIdError { AllZero }

macro_rules! opaque32 {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);
        impl $name {
            pub fn new(bytes: [u8; 32]) -> Result<Self, OpaqueIdError> {
                if bytes == [0; 32] { return Err(OpaqueIdError::AllZero); }
                Ok(Self(bytes))
            }
            pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
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
    pub fn new(source_commit: SourceCommit, dna_manifest: DnaManifestDigest, integrity_wasm_set: IntegrityWasmSetDigest, coordinator_wasm_set: CoordinatorWasmSetDigest, toolchain: ToolchainDigest, privacy_contract_epoch: u64) -> Result<Self, DeploymentIdentityError> {
        if privacy_contract_epoch == 0 { return Err(DeploymentIdentityError::ZeroPrivacyContractEpoch); }
        Ok(Self { source_commit, dna_manifest, integrity_wasm_set, coordinator_wasm_set, toolchain, privacy_contract_epoch })
    }
    fn append(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.source_commit.as_bytes()); out.extend_from_slice(self.dna_manifest.as_bytes());
        out.extend_from_slice(self.integrity_wasm_set.as_bytes()); out.extend_from_slice(self.coordinator_wasm_set.as_bytes());
        out.extend_from_slice(self.toolchain.as_bytes()); out.extend_from_slice(&self.privacy_contract_epoch.to_be_bytes());
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
    pub fn new(deployment: DeploymentIdentity, freeze_evidence_id: EvidenceId, frozen_legacy_state: LegacyStateDigest, freeze_epoch: u64, rehearsal_evidence_id: EvidenceId) -> Result<Self, CutoverIdentityError> {
        if freeze_epoch == 0 { return Err(CutoverIdentityError::ZeroFreezeEpoch); }
        if freeze_evidence_id == rehearsal_evidence_id { return Err(CutoverIdentityError::DuplicateEvidenceId); }
        Ok(Self { deployment, freeze_evidence_id, frozen_legacy_state, freeze_epoch, rehearsal_evidence_id })
    }
    pub fn deployment(&self) -> DeploymentIdentity { self.deployment }
    fn append(&self, out: &mut Vec<u8>) {
        self.deployment.append(out); out.extend_from_slice(self.freeze_evidence_id.as_bytes());
        out.extend_from_slice(self.frozen_legacy_state.as_bytes()); out.extend_from_slice(&self.freeze_epoch.to_be_bytes());
        out.extend_from_slice(self.rehearsal_evidence_id.as_bytes());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationSideReceipt {
    cutover: CutoverIdentity,
    evidence_set: EvidenceSetDigest,
    qualified: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationSideError { UnsupportedContractVersion }
impl ActivationSideReceipt {
    pub fn from_verified_adapter(cutover: CutoverIdentity, evidence_set: EvidenceSetDigest, contract_version: u16, qualified: bool) -> Result<Self, ActivationSideError> {
        if contract_version != REQUIRED_ACTIVATION_CONTRACT_V2 { return Err(ActivationSideError::UnsupportedContractVersion); }
        Ok(Self { cutover, evidence_set, qualified })
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
    fn append(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.destination_root.as_bytes()); out.extend_from_slice(self.protected_manifest.as_bytes());
        out.extend_from_slice(&self.key_epoch.to_be_bytes()); out.extend_from_slice(&self.policy_epoch.to_be_bytes()); out.extend_from_slice(&self.generation.to_be_bytes());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrationSideReceipt {
    cutover: CutoverIdentity,
    migration_evidence_id: EvidenceId,
    migration_plan: MigrationPlanDigest,
    route_accounting: RouteAccountingDigest,
    destination: DestinationIdentity,
    migration_binding: MigrationBindingDigest,
    qualified: bool,
    latest_active: bool,
    abandoned: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationSideError { UnsupportedContractVersion }
impl MigrationSideReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_adapter(cutover: CutoverIdentity, migration_evidence_id: EvidenceId, migration_plan: MigrationPlanDigest, route_accounting: RouteAccountingDigest, destination: DestinationIdentity, migration_binding: MigrationBindingDigest, contract_version: u16, qualified: bool, latest_active: bool, abandoned: bool) -> Result<Self, MigrationSideError> {
        if contract_version != REQUIRED_MIGRATION_CONTRACT_V1 { return Err(MigrationSideError::UnsupportedContractVersion); }
        Ok(Self { cutover, migration_evidence_id, migration_plan, route_accounting, destination, migration_binding, qualified, latest_active, abandoned })
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
        f.debug_struct("FinalActivationAuthority").field("authority_ref", &self.authority_ref).field("authority_binding", &self.authority_binding)
            .field("cutover", &self.cutover).field("destination", &self.destination).field("issued_at_micros", &self.issued_at_micros)
            .field("expires_at_micros", &self.expires_at_micros).field("historical_exposure_acknowledged", &self.historical_exposure_acknowledged).finish()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalAuthorityError { InvalidTimeWindow }
impl FinalActivationAuthority {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_adapter(authority_ref: AuthorityRef, authority_binding: AuthorityBindingDigest, cutover: CutoverIdentity, evidence_set: EvidenceSetDigest, migration_binding: MigrationBindingDigest, destination: DestinationIdentity, issued_at_micros: i64, expires_at_micros: i64, historical_exposure_acknowledged: bool) -> Result<Self, FinalAuthorityError> {
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
}
impl CompositionReceipt {
    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out=Vec::new(); out.extend_from_slice(DOMAIN); out.extend_from_slice(&COMPOSITION_CONTRACT_V1.to_be_bytes()); self.cutover.append(&mut out);
        out.extend_from_slice(self.evidence_set.as_bytes()); out.extend_from_slice(self.migration_evidence_id.as_bytes()); out.extend_from_slice(self.migration_plan.as_bytes());
        out.extend_from_slice(self.route_accounting.as_bytes()); out.extend_from_slice(self.migration_binding.as_bytes()); self.destination.append(&mut out);
        out.extend_from_slice(self.authority_ref.as_bytes()); out.extend_from_slice(self.authority_binding.as_bytes()); out.push(1); out
    }
    pub fn destination(&self) -> DestinationIdentity { self.destination }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionError {
    ActivationSourceNotQualified, MigrationSourceNotQualified, CutoverMismatch, MigrationNotLatest, MigrationAbandoned,
    AuthorityCutoverMismatch, AuthorityEvidenceMismatch, AuthorityMigrationMismatch, AuthorityDestinationMismatch,
    HistoricalExposureNotAcknowledged, AuthorityNotYetValid, AuthorityExpired, ClockRollback, InvalidState,
    CommitmentNotQualified, PrepareNotDurable, PrepareMismatch, ActivationNotDurable, ActivationMismatch,
    AuthoritySubstitution, ConflictingReplay,
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
    Ok(CompositionReceipt { cutover: activation.cutover, evidence_set: activation.evidence_set, migration_evidence_id: migration.migration_evidence_id, migration_plan: migration.migration_plan, route_accounting: migration.route_accounting, migration_binding: migration.migration_binding, destination: migration.destination, authority_ref: authority.authority_ref, authority_binding: authority.authority_binding })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositionCommitmentReceipt { digest: CompositionDigest, qualified: bool }
impl CompositionCommitmentReceipt { pub fn from_verified_adapter(digest: CompositionDigest, qualified: bool) -> Self { Self { digest, qualified } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurablePrepareReceipt {
    receipt_id: PrepareReceiptId,
    composition_digest: CompositionDigest,
    activation_epoch: u64,
    prepared_at_micros: i64,
    durable: bool,
}
impl DurablePrepareReceipt {
    pub fn from_verified_adapter(receipt_id: PrepareReceiptId, composition_digest: CompositionDigest, activation_epoch: u64, prepared_at_micros: i64, durable: bool) -> Self {
        Self { receipt_id, composition_digest, activation_epoch, prepared_at_micros, durable }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatientV2ActivationReceipt {
    receipt_id: ActivationReceiptId,
    prepare_receipt_id: PrepareReceiptId,
    composition_digest: CompositionDigest,
    activation_epoch: u64,
    activated_at_micros: i64,
    cutover: CutoverIdentity,
    destination: DestinationIdentity,
    authority_ref: AuthorityRef,
    authority_binding: AuthorityBindingDigest,
    v2_discoverable: bool,
    legacy_v1_read_only: bool,
    durable: bool,
}
impl PatientV2ActivationReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_adapter(receipt_id: ActivationReceiptId, prepare_receipt_id: PrepareReceiptId, composition_digest: CompositionDigest, activation_epoch: u64, activated_at_micros: i64, cutover: CutoverIdentity, destination: DestinationIdentity, authority_ref: AuthorityRef, authority_binding: AuthorityBindingDigest, v2_discoverable: bool, legacy_v1_read_only: bool, durable: bool) -> Self {
        Self { receipt_id, prepare_receipt_id, composition_digest, activation_epoch, activated_at_micros, cutover, destination, authority_ref, authority_binding, v2_discoverable, legacy_v1_read_only, durable }
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
        self.latest_time_micros=Some(now); Ok(())
    }
    pub fn admit_composition(&mut self, activation: ActivationSideReceipt, migration: MigrationSideReceipt, authority: &FinalActivationAuthority, commitment: CompositionCommitmentReceipt, now: i64) -> Result<(), CompositionError> {
        if self.phase != FinalPhase::AwaitingComposition { return Err(CompositionError::InvalidState); }
        self.observe_time(now)?; if !commitment.qualified { return Err(CompositionError::CommitmentNotQualified); }
        self.composition=Some(compose(activation,migration,authority,now)?); self.commitment=Some(commitment); self.phase=FinalPhase::CompositionAdmitted; Ok(())
    }
    pub fn admit_prepare(&mut self, prepare: DurablePrepareReceipt) -> Result<(), CompositionError> {
        if self.phase != FinalPhase::CompositionAdmitted { return Err(CompositionError::InvalidState); }
        self.observe_time(prepare.prepared_at_micros)?;
        let expected=self.commitment.ok_or(CompositionError::InvalidState)?;
        if !prepare.durable { return Err(CompositionError::PrepareNotDurable); }
        if prepare.composition_digest != expected.digest || prepare.activation_epoch == 0 { return Err(CompositionError::PrepareMismatch); }
        self.prepare=Some(prepare); self.phase=FinalPhase::ActivationCommitPrepared; Ok(())
    }
    pub fn admit_activation(&mut self, receipt: PatientV2ActivationReceipt, authority: &FinalActivationAuthority) -> Result<CommitOutcome, CompositionError> {
        if self.phase == FinalPhase::V2Active { return if self.activation == Some(receipt) { Ok(CommitOutcome::IdempotentReplay) } else { Err(CompositionError::ConflictingReplay) }; }
        if self.phase != FinalPhase::ActivationCommitPrepared { return Err(CompositionError::InvalidState); }
        self.observe_time(receipt.activated_at_micros)?; authority.valid_at(receipt.activated_at_micros)?;
        let composition=self.composition.as_ref().ok_or(CompositionError::InvalidState)?;
        if authority.authority_ref != composition.authority_ref || authority.authority_binding != composition.authority_binding || authority.cutover != composition.cutover || authority.destination != composition.destination { return Err(CompositionError::AuthoritySubstitution); }
        let prepare=self.prepare.ok_or(CompositionError::InvalidState)?; let digest=self.commitment.ok_or(CompositionError::InvalidState)?.digest;
        if !receipt.durable { return Err(CompositionError::ActivationNotDurable); }
        if receipt.prepare_receipt_id != prepare.receipt_id || receipt.composition_digest != digest || receipt.activation_epoch != prepare.activation_epoch
            || receipt.activated_at_micros < prepare.prepared_at_micros || receipt.cutover != composition.cutover || receipt.destination != composition.destination
            || receipt.authority_ref != composition.authority_ref || receipt.authority_binding != composition.authority_binding || !receipt.v2_discoverable || !receipt.legacy_v1_read_only { return Err(CompositionError::ActivationMismatch); }
        self.activation=Some(receipt); self.phase=FinalPhase::V2Active; Ok(CommitOutcome::Committed)
    }
    pub fn recover(composition: CompositionReceipt, commitment: CompositionCommitmentReceipt, prepare: Option<DurablePrepareReceipt>, activation: Option<PatientV2ActivationReceipt>) -> Result<Self, CompositionError> {
        if !commitment.qualified { return Err(CompositionError::CommitmentNotQualified); }
        match (prepare,activation) {
            (None,None) => Ok(Self { phase:FinalPhase::CompositionAdmitted, composition:Some(composition), commitment:Some(commitment), prepare:None, activation:None, latest_time_micros:None }),
            (Some(p),None) => {
                if !p.durable || p.composition_digest != commitment.digest || p.activation_epoch == 0 { return Err(CompositionError::PrepareMismatch); }
                Ok(Self { phase:FinalPhase::ActivationCommitPrepared, composition:Some(composition), commitment:Some(commitment), prepare:Some(p), activation:None, latest_time_micros:Some(p.prepared_at_micros) })
            }
            (Some(p),Some(a)) => {
                if !p.durable || !a.durable || p.composition_digest != commitment.digest || a.composition_digest != commitment.digest || a.prepare_receipt_id != p.receipt_id
                    || a.activation_epoch != p.activation_epoch || a.activated_at_micros < p.prepared_at_micros || a.cutover != composition.cutover || a.destination != composition.destination
                    || a.authority_ref != composition.authority_ref || a.authority_binding != composition.authority_binding || !a.v2_discoverable || !a.legacy_v1_read_only { return Err(CompositionError::ActivationMismatch); }
                Ok(Self { phase:FinalPhase::V2Active, composition:Some(composition), commitment:Some(commitment), prepare:Some(p), activation:Some(a), latest_time_micros:Some(a.activated_at_micros) })
            }
            (None,Some(_)) => Err(CompositionError::ActivationMismatch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn b(v:u8)->[u8;32]{[v;32]} macro_rules! o{($t:ident,$v:expr)=>{$t::new(b($v)).unwrap()};}
    fn dep(s:u8)->DeploymentIdentity{DeploymentIdentity::new(o!(SourceCommit,s),o!(DnaManifestDigest,s+1),o!(IntegrityWasmSetDigest,s+2),o!(CoordinatorWasmSetDigest,s+3),o!(ToolchainDigest,s+4),1).unwrap()}
    fn cut(s:u8)->CutoverIdentity{CutoverIdentity::new(dep(s),o!(EvidenceId,s+5),o!(LegacyStateDigest,s+6),7,o!(EvidenceId,s+7)).unwrap()}
    fn dst(s:u8)->DestinationIdentity{DestinationIdentity::new(o!(DestinationRootDigest,s),o!(ProtectedManifestDigest,s+1),2,3,1).unwrap()}
    fn fixture(s:u8)->(ActivationSideReceipt,MigrationSideReceipt,FinalActivationAuthority){let c=cut(s);let e=o!(EvidenceSetDigest,s+8);let d=dst(s+20);let m=o!(MigrationBindingDigest,s+30);(ActivationSideReceipt::from_verified_adapter(c,e,2,true).unwrap(),MigrationSideReceipt::from_verified_adapter(c,o!(EvidenceId,s+9),o!(MigrationPlanDigest,s+10),o!(RouteAccountingDigest,s+11),d,m,1,true,true,false).unwrap(),FinalActivationAuthority::from_verified_adapter(o!(AuthorityRef,s+40),o!(AuthorityBindingDigest,s+41),c,e,m,d,10,100,true).unwrap())}
    fn prepared()->(FinalCutoverCoordinator,FinalActivationAuthority,CompositionDigest,DurablePrepareReceipt){let(a,m,auth)=fixture(10);let d=o!(CompositionDigest,200);let mut c=FinalCutoverCoordinator::new();c.admit_composition(a,m,&auth,CompositionCommitmentReceipt::from_verified_adapter(d,true),50).unwrap();let p=DurablePrepareReceipt::from_verified_adapter(o!(PrepareReceiptId,201),d,1,55,true);c.admit_prepare(p).unwrap();(c,auth,d,p)}
    fn active_receipt(d:CompositionDigest,p:DurablePrepareReceipt,t:i64,id:u8)->PatientV2ActivationReceipt{PatientV2ActivationReceipt::from_verified_adapter(o!(ActivationReceiptId,id),p.receipt_id,d,1,t,cut(10),dst(30),o!(AuthorityRef,50),o!(AuthorityBindingDigest,51),true,true,true)}
    #[test] fn wrong_contract_versions_refused(){assert_eq!(ActivationSideReceipt::from_verified_adapter(cut(10),o!(EvidenceSetDigest,18),1,true),Err(ActivationSideError::UnsupportedContractVersion));assert_eq!(MigrationSideReceipt::from_verified_adapter(cut(10),o!(EvidenceId,19),o!(MigrationPlanDigest,20),o!(RouteAccountingDigest,21),dst(30),o!(MigrationBindingDigest,40),2,true,true,false),Err(MigrationSideError::UnsupportedContractVersion));}
    #[test] fn cross_cutover_mismatch_denies(){let(a,mut m,auth)=fixture(10);m.cutover=cut(60);assert_eq!(compose(a,m,&auth,50),Err(CompositionError::CutoverMismatch));}
    #[test] fn stale_and_abandoned_migrations_deny(){let(a,mut m,auth)=fixture(10);m.latest_active=false;assert_eq!(compose(a,m,&auth,50),Err(CompositionError::MigrationNotLatest));let(a,mut m,auth)=fixture(10);m.abandoned=true;assert_eq!(compose(a,m,&auth,50),Err(CompositionError::MigrationAbandoned));}
    #[test] fn final_authority_binds_destination(){let(a,m,mut auth)=fixture(10);auth.destination=dst(90);assert_eq!(compose(a,m,&auth,50),Err(CompositionError::AuthorityDestinationMismatch));}
    #[test] fn unqualified_commitment_denies(){let(a,m,auth)=fixture(10);let mut c=FinalCutoverCoordinator::new();assert_eq!(c.admit_composition(a,m,&auth,CompositionCommitmentReceipt::from_verified_adapter(o!(CompositionDigest,200),false),50),Err(CompositionError::CommitmentNotQualified));}
    #[test] fn prepare_time_is_durable_and_monotonic(){let(a,m,auth)=fixture(10);let d=o!(CompositionDigest,200);let mut c=FinalCutoverCoordinator::new();c.admit_composition(a,m,&auth,CompositionCommitmentReceipt::from_verified_adapter(d,true),50).unwrap();let p=DurablePrepareReceipt::from_verified_adapter(o!(PrepareReceiptId,201),d,1,49,true);assert_eq!(c.admit_prepare(p),Err(CompositionError::ClockRollback));}
    #[test] fn substituted_authority_denies_at_final_commit(){let(mut c,_auth,d,p)=prepared();let(_,_,mut other)=fixture(10);other.authority_ref=o!(AuthorityRef,99);let r=active_receipt(d,p,60,202);assert_eq!(c.admit_activation(r,&other),Err(CompositionError::AuthoritySubstitution));}
    #[test] fn exact_replay_is_idempotent_conflict_is_not(){let(mut c,auth,d,p)=prepared();let r=active_receipt(d,p,60,202);assert_eq!(c.admit_activation(r,&auth),Ok(CommitOutcome::Committed));assert_eq!(c.admit_activation(r,&auth),Ok(CommitOutcome::IdempotentReplay));let conflict=active_receipt(d,p,60,203);assert_eq!(c.admit_activation(conflict,&auth),Err(CompositionError::ConflictingReplay));}
    #[test] fn activation_time_must_not_precede_prepare(){let(mut c,auth,d,p)=prepared();let r=active_receipt(d,p,54,202);assert_eq!(c.admit_activation(r,&auth),Err(CompositionError::ClockRollback));}
    #[test] fn expiry_observation_cannot_be_rolled_back(){let(mut c,auth,d,p)=prepared();let r=active_receipt(d,p,100,202);assert_eq!(c.admit_activation(r,&auth),Err(CompositionError::AuthorityExpired));let earlier=active_receipt(d,p,99,202);assert_eq!(c.admit_activation(earlier,&auth),Err(CompositionError::ClockRollback));}
    #[test] fn recovery_preserves_prepare_time_fence(){let(a,m,auth)=fixture(10);let comp=compose(a,m,&auth,50).unwrap();let d=o!(CompositionDigest,200);let cm=CompositionCommitmentReceipt::from_verified_adapter(d,true);let p=DurablePrepareReceipt::from_verified_adapter(o!(PrepareReceiptId,201),d,1,70,true);let mut recovered=FinalCutoverCoordinator::recover(comp,cm,Some(p),None).unwrap();let r=active_receipt(d,p,69,202);assert_eq!(recovered.admit_activation(r,&auth),Err(CompositionError::ClockRollback));}
    #[test] fn durable_active_recovery_never_reopens_v1(){let(a,m,auth)=fixture(10);let comp=compose(a,m,&auth,50).unwrap();let d=o!(CompositionDigest,200);let cm=CompositionCommitmentReceipt::from_verified_adapter(d,true);let p=DurablePrepareReceipt::from_verified_adapter(o!(PrepareReceiptId,201),d,1,55,true);let r=active_receipt(d,p,60,202);let recovered=FinalCutoverCoordinator::recover(comp,cm,Some(p),Some(r)).unwrap();assert_eq!(recovered.phase(),FinalPhase::V2Active);assert_eq!(recovered.activation_receipt(),Some(r));}
    #[test] fn orphan_activation_receipt_is_rejected(){let(a,m,auth)=fixture(10);let comp=compose(a,m,&auth,50).unwrap();let d=o!(CompositionDigest,200);let cm=CompositionCommitmentReceipt::from_verified_adapter(d,true);let p=DurablePrepareReceipt::from_verified_adapter(o!(PrepareReceiptId,201),d,1,55,true);let r=active_receipt(d,p,60,202);assert!(matches!(FinalCutoverCoordinator::recover(comp,cm,None,Some(r)),Err(CompositionError::ActivationMismatch)));}
    #[test] fn transcript_changes_with_destination(){let(a,m,auth)=fixture(10);let x=compose(a,m,&auth,50).unwrap();let(a2,mut m2,mut auth2)=fixture(10);let d=dst(90);m2.destination=d;auth2.destination=d;let y=compose(a2,m2,&auth2,50).unwrap();assert_ne!(x.transcript_bytes(),y.transcript_bytes());}
    #[test] fn zero_ids_refused(){assert_eq!(EvidenceId::new([0;32]),Err(OpaqueIdError::AllZero));}
}
