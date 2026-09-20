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
opaque32!(CommitReceiptId);

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
pub enum CutoverIdentityError {
    ZeroFreezeEpoch,
    DuplicateFreezeAndRehearsalEvidenceId,
}

impl CutoverIdentity {
    pub fn new(
        deployment: DeploymentIdentity,
        freeze_evidence_id: EvidenceId,
        frozen_legacy_state: LegacyStateDigest,
        freeze_epoch: u64,
        rehearsal_evidence_id: EvidenceId,
    ) -> Result<Self, CutoverIdentityError> {
        if freeze_epoch == 0 {
            return Err(CutoverIdentityError::ZeroFreezeEpoch);
        }
        if freeze_evidence_id == rehearsal_evidence_id {
            return Err(CutoverIdentityError::DuplicateFreezeAndRehearsalEvidenceId);
        }
        Ok(Self {
            deployment,
            freeze_evidence_id,
            frozen_legacy_state,
            freeze_epoch,
            rehearsal_evidence_id,
        })
    }

    pub fn deployment(&self) -> DeploymentIdentity {
        self.deployment
    }

    fn append_transcript(&self, out: &mut Vec<u8>) {
        self.deployment.append_transcript(out);
        out.extend_from_slice(self.freeze_evidence_id.bytes());
        out.extend_from_slice(self.frozen_legacy_state.bytes());
        out.extend_from_slice(&self.freeze_epoch.to_be_bytes());
        out.extend_from_slice(self.rehearsal_evidence_id.bytes());
    }
}

/// Already-verified adapter receipt representing the #204 activation/freeze/rehearsal side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationSideReceipt {
    cutover: CutoverIdentity,
    evidence_set: EvidenceSetDigest,
    activation_contract_version: u16,
    source_qualified: bool,
}

impl ActivationSideReceipt {
    pub fn from_verified_adapter(
        cutover: CutoverIdentity,
        evidence_set: EvidenceSetDigest,
        activation_contract_version: u16,
        source_qualified: bool,
    ) -> Self {
        Self {
            cutover,
            evidence_set,
            activation_contract_version,
            source_qualified,
        }
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
pub enum DestinationIdentityError {
    ZeroKeyEpoch,
    ZeroPolicyEpoch,
    ZeroGeneration,
}

impl DestinationIdentity {
    pub fn new(
        destination_root: DestinationRootDigest,
        protected_manifest: ProtectedManifestDigest,
        key_epoch: u64,
        policy_epoch: u64,
        generation: u64,
    ) -> Result<Self, DestinationIdentityError> {
        if key_epoch == 0 {
            return Err(DestinationIdentityError::ZeroKeyEpoch);
        }
        if policy_epoch == 0 {
            return Err(DestinationIdentityError::ZeroPolicyEpoch);
        }
        if generation == 0 {
            return Err(DestinationIdentityError::ZeroGeneration);
        }
        Ok(Self {
            destination_root,
            protected_manifest,
            key_epoch,
            policy_epoch,
            generation,
        })
    }

    fn append_transcript(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.destination_root.bytes());
        out.extend_from_slice(self.protected_manifest.bytes());
        out.extend_from_slice(&self.key_epoch.to_be_bytes());
        out.extend_from_slice(&self.policy_epoch.to_be_bytes());
        out.extend_from_slice(&self.generation.to_be_bytes());
    }
}

/// Already-verified adapter receipt representing the strict/latest #206 migration binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrationSideReceipt {
    cutover: CutoverIdentity,
    migration_evidence_id: EvidenceId,
    migration_plan: MigrationPlanDigest,
    route_accounting: RouteAccountingDigest,
    destination: DestinationIdentity,
    migration_binding: MigrationBindingDigest,
    migration_contract_version: u16,
    source_qualified: bool,
    latest_active_generation: bool,
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
        migration_contract_version: u16,
        source_qualified: bool,
        latest_active_generation: bool,
        abandoned: bool,
    ) -> Self {
        Self {
            cutover,
            migration_evidence_id,
            migration_plan,
            route_accounting,
            destination,
            migration_binding,
            migration_contract_version,
            source_qualified,
            latest_active_generation,
            abandoned,
        }
    }
}

/// Final authority over the exact post-migration destination, verified by an external adapter.
#[derive(Clone, PartialEq, Eq)]
pub struct FinalActivationAuthority {
    authority_ref: AuthorityRef,
    cutover: CutoverIdentity,
    evidence_set: EvidenceSetDigest,
    migration_binding: MigrationBindingDigest,
    destination: DestinationIdentity,
    authority_binding: AuthorityBindingDigest,
    issued_at_micros: i64,
    expires_at_micros: i64,
    historical_exposure_acknowledged: bool,
}

impl fmt::Debug for FinalActivationAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FinalActivationAuthority")
            .field("authority_ref", &self.authority_ref)
            .field("cutover", &self.cutover)
            .field("evidence_set", &self.evidence_set)
            .field("migration_binding", &self.migration_binding)
            .field("destination", &self.destination)
            .field("authority_binding", &self.authority_binding)
            .field("issued_at_micros", &self.issued_at_micros)
            .field("expires_at_micros", &self.expires_at_micros)
            .field("historical_exposure_acknowledged", &self.historical_exposure_acknowledged)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalAuthorityError {
    InvalidTimeWindow,
}

#[allow(clippy::too_many_arguments)]
impl FinalActivationAuthority {
    pub fn from_verified_adapter(
        authority_ref: AuthorityRef,
        cutover: CutoverIdentity,
        evidence_set: EvidenceSetDigest,
        migration_binding: MigrationBindingDigest,
        destination: DestinationIdentity,
        authority_binding: AuthorityBindingDigest,
        issued_at_micros: i64,
        expires_at_micros: i64,
        historical_exposure_acknowledged: bool,
    ) -> Result<Self, FinalAuthorityError> {
        if expires_at_micros <= issued_at_micros {
            return Err(FinalAuthorityError::InvalidTimeWindow);
        }
        Ok(Self {
            authority_ref,
            cutover,
            evidence_set,
            migration_binding,
            destination,
            authority_binding,
            issued_at_micros,
            expires_at_micros,
            historical_exposure_acknowledged,
        })
    }

    fn valid_at(&self, now_micros: i64) -> Result<(), CompositionError> {
        if now_micros < self.issued_at_micros {
            return Err(CompositionError::AuthorityNotYetValid);
        }
        if now_micros >= self.expires_at_micros {
            return Err(CompositionError::AuthorityExpired);
        }
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
    final_authority: AuthorityRef,
    authority_binding: AuthorityBindingDigest,
    activation_contract_version: u16,
    migration_contract_version: u16,
    composition_contract_version: u16,
    historical_exposure_acknowledged: bool,
}

impl CompositionReceipt {
    pub fn transcript_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(COMPOSITION_DOMAIN);
        out.extend_from_slice(&self.composition_contract_version.to_be_bytes());
        self.cutover.append_transcript(&mut out);
        out.extend_from_slice(self.evidence_set.bytes());
        out.extend_from_slice(self.migration_evidence_id.bytes());
        out.extend_from_slice(self.migration_plan.bytes());
        out.extend_from_slice(self.route_accounting.bytes());
        out.extend_from_slice(self.migration_binding.bytes());
        self.destination.append_transcript(&mut out);
        out.extend_from_slice(self.final_authority.bytes());
        out.extend_from_slice(self.authority_binding.bytes());
        out.extend_from_slice(&self.activation_contract_version.to_be_bytes());
        out.extend_from_slice(&self.migration_contract_version.to_be_bytes());
        out.push(u8::from(self.historical_exposure_acknowledged));
        out
    }
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
    PrepareNotDurable,
    PrepareMismatch,
    CommitNotDurable,
    CommitMismatch,
    ConflictingReplay,
}

pub fn compose(
    activation: ActivationSideReceipt,
    migration: MigrationSideReceipt,
    authority: &FinalActivationAuthority,
    now_micros: i64,
) -> Result<CompositionReceipt, CompositionError> {
    if !activation.source_qualified {
        return Err(CompositionError::ActivationSourceNotQualified);
    }
    if !migration.source_qualified {
        return Err(CompositionError::MigrationSourceNotQualified);
    }
    if activation.cutover != migration.cutover {
        return Err(CompositionError::CutoverMismatch);
    }
    if !migration.latest_active_generation {
        return Err(CompositionError::MigrationNotLatest);
    }
    if migration.abandoned {
        return Err(CompositionError::MigrationAbandoned);
    }
    if authority.cutover != activation.cutover {
        return Err(CompositionError::AuthorityCutoverMismatch);
    }
    if authority.evidence_set != activation.evidence_set {
        return Err(CompositionError::AuthorityEvidenceMismatch);
    }
    if authority.migration_binding != migration.migration_binding {
        return Err(CompositionError::AuthorityMigrationMismatch);
    }
    if authority.destination != migration.destination {
        return Err(CompositionError::AuthorityDestinationMismatch);
    }
    if !authority.historical_exposure_acknowledged {
        return Err(CompositionError::HistoricalExposureNotAcknowledged);
    }
    authority.valid_at(now_micros)?;

    Ok(CompositionReceipt {
        cutover: activation.cutover,
        evidence_set: activation.evidence_set,
        migration_evidence_id: migration.migration_evidence_id,
        migration_plan: migration.migration_plan,
        route_accounting: migration.route_accounting,
        migration_binding: migration.migration_binding,
        destination: migration.destination,
        final_authority: authority.authority_ref,
        authority_binding: authority.authority_binding,
        activation_contract_version: activation.activation_contract_version,
        migration_contract_version: migration.migration_contract_version,
        composition_contract_version: COMPOSITION_CONTRACT_V1,
        historical_exposure_acknowledged: true,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurablePrepareReceipt {
    receipt_id: PrepareReceiptId,
    composition_digest: CompositionDigest,
    activation_epoch: u64,
    durable: bool,
}

impl DurablePrepareReceipt {
    pub fn from_verified_adapter(
        receipt_id: PrepareReceiptId,
        composition_digest: CompositionDigest,
        activation_epoch: u64,
        durable: bool,
    ) -> Self {
        Self {
            receipt_id,
            composition_digest,
            activation_epoch,
            durable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurableCommitReceipt {
    receipt_id: CommitReceiptId,
    prepare_receipt_id: PrepareReceiptId,
    composition_digest: CompositionDigest,
    activation_epoch: u64,
    v2_discoverable: bool,
    legacy_v1_read_only: bool,
    durable: bool,
}

#[allow(clippy::too_many_arguments)]
impl DurableCommitReceipt {
    pub fn from_verified_adapter(
        receipt_id: CommitReceiptId,
        prepare_receipt_id: PrepareReceiptId,
        composition_digest: CompositionDigest,
        activation_epoch: u64,
        v2_discoverable: bool,
        legacy_v1_read_only: bool,
        durable: bool,
    ) -> Self {
        Self {
            receipt_id,
            prepare_receipt_id,
            composition_digest,
            activation_epoch,
            v2_discoverable,
            legacy_v1_read_only,
            durable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalPhase {
    AwaitingComposition,
    CompositionAdmitted,
    ActivationCommitPrepared,
    V2Active,
}

#[derive(Clone, Debug)]
pub struct FinalCutoverCoordinator {
    phase: FinalPhase,
    composition: Option<CompositionReceipt>,
    composition_digest: Option<CompositionDigest>,
    prepare: Option<DurablePrepareReceipt>,
    commit: Option<DurableCommitReceipt>,
    latest_time_micros: Option<i64>,
}

impl Default for FinalCutoverCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl FinalCutoverCoordinator {
    pub fn new() -> Self {
        Self {
            phase: FinalPhase::AwaitingComposition,
            composition: None,
            composition_digest: None,
            prepare: None,
            commit: None,
            latest_time_micros: None,
        }
    }

    pub fn phase(&self) -> FinalPhase {
        self.phase
    }

    fn observe_time(&mut self, now_micros: i64) -> Result<(), CompositionError> {
        if let Some(previous) = self.latest_time_micros {
            if now_micros < previous {
                return Err(CompositionError::ClockRollback);
            }
        }
        self.latest_time_micros = Some(now_micros);
        Ok(())
    }

    pub fn admit_composition(
        &mut self,
        activation: ActivationSideReceipt,
        migration: MigrationSideReceipt,
        authority: &FinalActivationAuthority,
        composition_digest: CompositionDigest,
        now_micros: i64,
    ) -> Result<(), CompositionError> {
        if self.phase != FinalPhase::AwaitingComposition {
            return Err(CompositionError::InvalidState);
        }
        self.observe_time(now_micros)?;
        let receipt = compose(activation, migration, authority, now_micros)?;
        self.composition = Some(receipt);
        self.composition_digest = Some(composition_digest);
        self.phase = FinalPhase::CompositionAdmitted;
        Ok(())
    }

    pub fn admit_prepare(
        &mut self,
        prepare: DurablePrepareReceipt,
    ) -> Result<(), CompositionError> {
        if self.phase != FinalPhase::CompositionAdmitted {
            return Err(CompositionError::InvalidState);
        }
        let expected = self.composition_digest.ok_or(CompositionError::InvalidState)?;
        if !prepare.durable {
            return Err(CompositionError::PrepareNotDurable);
        }
        if prepare.composition_digest != expected || prepare.activation_epoch == 0 {
            return Err(CompositionError::PrepareMismatch);
        }
        self.prepare = Some(prepare);
        self.phase = FinalPhase::ActivationCommitPrepared;
        Ok(())
    }

    pub fn admit_commit(
        &mut self,
        commit: DurableCommitReceipt,
        authority: &FinalActivationAuthority,
        now_micros: i64,
    ) -> Result<(), CompositionError> {
        if self.phase != FinalPhase::ActivationCommitPrepared {
            return Err(CompositionError::InvalidState);
        }
        self.observe_time(now_micros)?;
        authority.valid_at(now_micros)?;
        let prepare = self.prepare.ok_or(CompositionError::InvalidState)?;
        if !commit.durable {
            return Err(CompositionError::CommitNotDurable);
        }
        if commit.prepare_receipt_id != prepare.receipt_id
            || commit.composition_digest != prepare.composition_digest
            || commit.activation_epoch != prepare.activation_epoch
            || !commit.v2_discoverable
            || !commit.legacy_v1_read_only
        {
            return Err(CompositionError::CommitMismatch);
        }
        self.commit = Some(commit);
        self.phase = FinalPhase::V2Active;
        Ok(())
    }

    pub fn recover(
        composition: CompositionReceipt,
        composition_digest: CompositionDigest,
        prepare: Option<DurablePrepareReceipt>,
        commit: Option<DurableCommitReceipt>,
    ) -> Result<Self, CompositionError> {
        match (prepare, commit) {
            (None, None) => Ok(Self {
                phase: FinalPhase::CompositionAdmitted,
                composition: Some(composition),
                composition_digest: Some(composition_digest),
                prepare: None,
                commit: None,
                latest_time_micros: None,
            }),
            (Some(prepare), None) => {
                if !prepare.durable
                    || prepare.composition_digest != composition_digest
                    || prepare.activation_epoch == 0
                {
                    return Err(CompositionError::PrepareMismatch);
                }
                Ok(Self {
                    phase: FinalPhase::ActivationCommitPrepared,
                    composition: Some(composition),
                    composition_digest: Some(composition_digest),
                    prepare: Some(prepare),
                    commit: None,
                    latest_time_micros: None,
                })
            }
            (Some(prepare), Some(commit)) => {
                if !prepare.durable
                    || !commit.durable
                    || prepare.composition_digest != composition_digest
                    || commit.composition_digest != composition_digest
                    || commit.prepare_receipt_id != prepare.receipt_id
                    || commit.activation_epoch != prepare.activation_epoch
                    || !commit.v2_discoverable
                    || !commit.legacy_v1_read_only
                {
                    return Err(CompositionError::CommitMismatch);
                }
                Ok(Self {
                    phase: FinalPhase::V2Active,
                    composition: Some(composition),
                    composition_digest: Some(composition_digest),
                    prepare: Some(prepare),
                    commit: Some(commit),
                    latest_time_micros: None,
                })
            }
            (None, Some(_)) => Err(CompositionError::CommitMismatch),
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
        ($ty:ident, $v:expr) => {
            $ty::new(bytes($v)).unwrap()
        };
    }

    fn deployment(seed: u8) -> DeploymentIdentity {
        DeploymentIdentity::new(
            oid!(SourceCommit, seed),
            oid!(DnaManifestDigest, seed + 1),
            oid!(IntegrityWasmSetDigest, seed + 2),
            oid!(CoordinatorWasmSetDigest, seed + 3),
            oid!(ToolchainDigest, seed + 4),
            1,
        )
        .unwrap()
    }

    fn cutover(seed: u8) -> CutoverIdentity {
        CutoverIdentity::new(
            deployment(seed),
            oid!(EvidenceId, seed + 5),
            oid!(LegacyStateDigest, seed + 6),
            7,
            oid!(EvidenceId, seed + 7),
        )
        .unwrap()
    }

    fn destination(seed: u8) -> DestinationIdentity {
        DestinationIdentity::new(
            oid!(DestinationRootDigest, seed),
            oid!(ProtectedManifestDigest, seed + 1),
            2,
            3,
            1,
        )
        .unwrap()
    }

    fn fixture(seed: u8) -> (ActivationSideReceipt, MigrationSideReceipt, FinalActivationAuthority) {
        let cutover = cutover(seed);
        let evidence_set = oid!(EvidenceSetDigest, seed + 8);
        let dest = destination(seed + 20);
        let migration_binding = oid!(MigrationBindingDigest, seed + 30);
        let activation = ActivationSideReceipt::from_verified_adapter(cutover, evidence_set, 2, true);
        let migration = MigrationSideReceipt::from_verified_adapter(
            cutover,
            oid!(EvidenceId, seed + 9),
            oid!(MigrationPlanDigest, seed + 10),
            oid!(RouteAccountingDigest, seed + 11),
            dest,
            migration_binding,
            1,
            true,
            true,
            false,
        );
        let authority = FinalActivationAuthority::from_verified_adapter(
            oid!(AuthorityRef, seed + 40),
            cutover,
            evidence_set,
            migration_binding,
            dest,
            oid!(AuthorityBindingDigest, seed + 41),
            10,
            100,
            true,
        )
        .unwrap();
        (activation, migration, authority)
    }

    #[test]
    fn mismatched_cutover_denies() {
        let (activation, mut migration, authority) = fixture(10);
        migration.cutover = cutover(50);
        assert_eq!(
            compose(activation, migration, &authority, 50),
            Err(CompositionError::CutoverMismatch)
        );
    }

    #[test]
    fn stale_or_abandoned_migration_denies() {
        let (activation, mut migration, authority) = fixture(10);
        migration.latest_active_generation = false;
        assert_eq!(
            compose(activation, migration, &authority, 50),
            Err(CompositionError::MigrationNotLatest)
        );
        let (activation, mut migration, authority) = fixture(10);
        migration.abandoned = true;
        assert_eq!(
            compose(activation, migration, &authority, 50),
            Err(CompositionError::MigrationAbandoned)
        );
    }

    #[test]
    fn authority_must_bind_exact_migration_and_destination() {
        let (activation, migration, mut authority) = fixture(10);
        authority.migration_binding = oid!(MigrationBindingDigest, 99);
        assert_eq!(
            compose(activation, migration, &authority, 50),
            Err(CompositionError::AuthorityMigrationMismatch)
        );

        let (activation, migration, mut authority) = fixture(10);
        authority.destination = destination(80);
        assert_eq!(
            compose(activation, migration, &authority, 50),
            Err(CompositionError::AuthorityDestinationMismatch)
        );
    }

    #[test]
    fn unqualified_lower_layer_denies() {
        let (mut activation, migration, authority) = fixture(10);
        activation.source_qualified = false;
        assert_eq!(
            compose(activation, migration, &authority, 50),
            Err(CompositionError::ActivationSourceNotQualified)
        );

        let (activation, mut migration, authority) = fixture(10);
        migration.source_qualified = false;
        assert_eq!(
            compose(activation, migration, &authority, 50),
            Err(CompositionError::MigrationSourceNotQualified)
        );
    }

    #[test]
    fn authority_expiry_denies_composition() {
        let (activation, migration, authority) = fixture(10);
        assert_eq!(
            compose(activation, migration, &authority, 100),
            Err(CompositionError::AuthorityExpired)
        );
    }

    #[test]
    fn prepare_must_be_durable_and_exact() {
        let (activation, migration, authority) = fixture(10);
        let mut coordinator = FinalCutoverCoordinator::new();
        let digest = oid!(CompositionDigest, 200);
        coordinator
            .admit_composition(activation, migration, &authority, digest, 50)
            .unwrap();
        let bad = DurablePrepareReceipt::from_verified_adapter(
            oid!(PrepareReceiptId, 201),
            digest,
            1,
            false,
        );
        assert_eq!(coordinator.admit_prepare(bad), Err(CompositionError::PrepareNotDurable));
    }

    #[test]
    fn commit_requires_v2_discoverable_and_legacy_read_only() {
        let (activation, migration, authority) = fixture(10);
        let digest = oid!(CompositionDigest, 200);
        let prepare = DurablePrepareReceipt::from_verified_adapter(
            oid!(PrepareReceiptId, 201),
            digest,
            1,
            true,
        );
        let mut coordinator = FinalCutoverCoordinator::new();
        coordinator
            .admit_composition(activation, migration, &authority, digest, 50)
            .unwrap();
        coordinator.admit_prepare(prepare).unwrap();
        let bad = DurableCommitReceipt::from_verified_adapter(
            oid!(CommitReceiptId, 202),
            prepare.receipt_id,
            digest,
            1,
            false,
            true,
            true,
        );
        assert_eq!(coordinator.admit_commit(bad, &authority, 60), Err(CompositionError::CommitMismatch));
    }

    #[test]
    fn authority_is_rechecked_at_commit_and_time_is_monotonic() {
        let (activation, migration, authority) = fixture(10);
        let digest = oid!(CompositionDigest, 200);
        let prepare = DurablePrepareReceipt::from_verified_adapter(
            oid!(PrepareReceiptId, 201),
            digest,
            1,
            true,
        );
        let mut coordinator = FinalCutoverCoordinator::new();
        coordinator
            .admit_composition(activation, migration, &authority, digest, 50)
            .unwrap();
        coordinator.admit_prepare(prepare).unwrap();
        let commit = DurableCommitReceipt::from_verified_adapter(
            oid!(CommitReceiptId, 202),
            prepare.receipt_id,
            digest,
            1,
            true,
            true,
            true,
        );
        assert_eq!(coordinator.admit_commit(commit, &authority, 100), Err(CompositionError::AuthorityExpired));
        assert_eq!(coordinator.admit_commit(commit, &authority, 99), Err(CompositionError::ClockRollback));
    }

    #[test]
    fn crash_recovery_never_implies_legacy_writable() {
        let (activation, migration, authority) = fixture(10);
        let composition = compose(activation, migration, &authority, 50).unwrap();
        let digest = oid!(CompositionDigest, 200);
        let prepare = DurablePrepareReceipt::from_verified_adapter(
            oid!(PrepareReceiptId, 201),
            digest,
            1,
            true,
        );
        let prepared = FinalCutoverCoordinator::recover(composition.clone(), digest, Some(prepare), None).unwrap();
        assert_eq!(prepared.phase(), FinalPhase::ActivationCommitPrepared);

        let commit = DurableCommitReceipt::from_verified_adapter(
            oid!(CommitReceiptId, 202),
            prepare.receipt_id,
            digest,
            1,
            true,
            true,
            true,
        );
        let active = FinalCutoverCoordinator::recover(composition, digest, Some(prepare), Some(commit)).unwrap();
        assert_eq!(active.phase(), FinalPhase::V2Active);
    }

    #[test]
    fn orphan_commit_is_rejected() {
        let (activation, migration, authority) = fixture(10);
        let composition = compose(activation, migration, &authority, 50).unwrap();
        let digest = oid!(CompositionDigest, 200);
        let commit = DurableCommitReceipt::from_verified_adapter(
            oid!(CommitReceiptId, 202),
            oid!(PrepareReceiptId, 201),
            digest,
            1,
            true,
            true,
            true,
        );
        assert_eq!(
            FinalCutoverCoordinator::recover(composition, digest, None, Some(commit)),
            Err(CompositionError::CommitMismatch)
        );
    }

    #[test]
    fn composition_transcript_changes_with_destination() {
        let (activation, migration, authority) = fixture(10);
        let a = compose(activation, migration, &authority, 50).unwrap();
        let (activation2, mut migration2, mut authority2) = fixture(10);
        let new_dest = destination(90);
        migration2.destination = new_dest;
        authority2.destination = new_dest;
        let b = compose(activation2, migration2, &authority2, 50).unwrap();
        assert_ne!(a.transcript_bytes(), b.transcript_bytes());
    }

    #[test]
    fn opaque_ids_reject_zero() {
        assert_eq!(EvidenceId::new([0; 32]), Err(OpaqueIdError::AllZero));
    }
}
