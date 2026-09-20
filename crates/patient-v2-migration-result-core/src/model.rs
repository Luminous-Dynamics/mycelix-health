use core::fmt;

pub const MIGRATION_RESULT_CONTRACT_V1: u16 = 1;

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
opaque32!(LegacyStateDigest);
opaque32!(MigrationSchemaDigest);
opaque32!(DestinationRootDigest);
opaque32!(ProtectedManifestDigest);
opaque32!(RouteAccountingDigest);
opaque32!(QuarantineCommitment);
opaque32!(AuthorityRef);
opaque32!(AbortReasonDigest);

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrationPlanRef {
    version: u16,
    schema_digest: MigrationSchemaDigest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationPlanError {
    ZeroVersion,
}

impl MigrationPlanRef {
    pub fn new(version: u16, schema_digest: MigrationSchemaDigest) -> Result<Self, MigrationPlanError> {
        if version == 0 {
            return Err(MigrationPlanError::ZeroVersion);
        }
        Ok(Self { version, schema_digest })
    }

    pub fn version(&self) -> u16 {
        self.version
    }
}

/// Exact source-side context that production migration is allowed to transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MigrationContext {
    deployment: DeploymentIdentity,
    freeze_evidence_id: EvidenceId,
    legacy_state_digest: LegacyStateDigest,
    freeze_epoch: u64,
    rehearsal_evidence_id: EvidenceId,
    plan: MigrationPlanRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationContextError {
    ZeroFreezeEpoch,
    DuplicateEvidenceId,
}

impl MigrationContext {
    pub fn new(
        deployment: DeploymentIdentity,
        freeze_evidence_id: EvidenceId,
        legacy_state_digest: LegacyStateDigest,
        freeze_epoch: u64,
        rehearsal_evidence_id: EvidenceId,
        plan: MigrationPlanRef,
    ) -> Result<Self, MigrationContextError> {
        if freeze_epoch == 0 {
            return Err(MigrationContextError::ZeroFreezeEpoch);
        }
        if freeze_evidence_id == rehearsal_evidence_id {
            return Err(MigrationContextError::DuplicateEvidenceId);
        }
        Ok(Self {
            deployment,
            freeze_evidence_id,
            legacy_state_digest,
            freeze_epoch,
            rehearsal_evidence_id,
            plan,
        })
    }

    pub fn deployment(&self) -> DeploymentIdentity {
        self.deployment
    }

    pub fn freeze_epoch(&self) -> u64 {
        self.freeze_epoch
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DestinationIdentity {
    root_digest: DestinationRootDigest,
    protected_manifest_digest: ProtectedManifestDigest,
    key_epoch: u64,
    policy_epoch: u64,
    generation: u64,
    predecessor_root: Option<DestinationRootDigest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationIdentityError {
    ZeroKeyEpoch,
    ZeroPolicyEpoch,
    ZeroGeneration,
    GenesisHasPredecessor,
    MissingPredecessor,
}

impl DestinationIdentity {
    pub fn new(
        root_digest: DestinationRootDigest,
        protected_manifest_digest: ProtectedManifestDigest,
        key_epoch: u64,
        policy_epoch: u64,
        generation: u64,
        predecessor_root: Option<DestinationRootDigest>,
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
        if generation == 1 && predecessor_root.is_some() {
            return Err(DestinationIdentityError::GenesisHasPredecessor);
        }
        if generation > 1 && predecessor_root.is_none() {
            return Err(DestinationIdentityError::MissingPredecessor);
        }
        Ok(Self {
            root_digest,
            protected_manifest_digest,
            key_epoch,
            policy_epoch,
            generation,
            predecessor_root,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn root_digest(&self) -> DestinationRootDigest {
        self.root_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteAccounting {
    commitment: RouteAccountingDigest,
    required_routes: u32,
    migrated_routes: u32,
    unresolved_routes: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteAccountingError {
    ZeroRequiredRoutes,
    CountOverflow,
}

impl RouteAccounting {
    pub fn new(
        commitment: RouteAccountingDigest,
        required_routes: u32,
        migrated_routes: u32,
        unresolved_routes: u32,
    ) -> Result<Self, RouteAccountingError> {
        if required_routes == 0 {
            return Err(RouteAccountingError::ZeroRequiredRoutes);
        }
        if migrated_routes > required_routes || unresolved_routes > required_routes {
            return Err(RouteAccountingError::CountOverflow);
        }
        Ok(Self {
            commitment,
            required_routes,
            migrated_routes,
            unresolved_routes,
        })
    }

    fn is_complete(&self) -> bool {
        self.unresolved_routes == 0 && self.migrated_routes == self.required_routes
    }
}

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

/// Adapter-supplied result describing the actual frozen-source -> protected-
/// destination migration. Construction is not verification; `verify_migration`
/// is the structural admission boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductionMigrationReceipt {
    evidence_id: EvidenceId,
    state: EvidenceState,
    context: MigrationContext,
    destination: DestinationIdentity,
    routes: RouteAccounting,
    idempotency_verified: bool,
    conflict_free_verified: bool,
    source_destination_verified: bool,
    payload_verified: bool,
    topology_verified: bool,
    historical_public_exposure_may_persist: bool,
}

impl ProductionMigrationReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_adapter(
        evidence_id: EvidenceId,
        state: EvidenceState,
        context: MigrationContext,
        destination: DestinationIdentity,
        routes: RouteAccounting,
        idempotency_verified: bool,
        conflict_free_verified: bool,
        source_destination_verified: bool,
        payload_verified: bool,
        topology_verified: bool,
    ) -> Self {
        Self {
            evidence_id,
            state,
            context,
            destination,
            routes,
            idempotency_verified,
            conflict_free_verified,
            source_destination_verified,
            payload_verified,
            topology_verified,
            historical_public_exposure_may_persist: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationVerificationError {
    ContextMismatch,
    NotQualified(EvidenceState),
    ReusesFreezeEvidenceId,
    ReusesRehearsalEvidenceId,
    IncompleteRouteAccounting,
    IdempotencyNotVerified,
    ConflictFreedomNotVerified,
    SourceDestinationNotVerified,
    PayloadNotVerified,
    TopologyNotVerified,
    HistoricalExposureNotAcknowledged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedMigration {
    evidence_id: EvidenceId,
    context: MigrationContext,
    destination: DestinationIdentity,
    routes: RouteAccounting,
}

impl VerifiedMigration {
    pub fn evidence_id(&self) -> EvidenceId {
        self.evidence_id
    }

    pub fn context(&self) -> MigrationContext {
        self.context
    }

    pub fn destination(&self) -> DestinationIdentity {
        self.destination
    }
}

pub fn verify_migration(
    expected: MigrationContext,
    receipt: ProductionMigrationReceipt,
) -> Result<VerifiedMigration, MigrationVerificationError> {
    if receipt.context != expected {
        return Err(MigrationVerificationError::ContextMismatch);
    }
    if receipt.state != EvidenceState::Qualified {
        return Err(MigrationVerificationError::NotQualified(receipt.state));
    }
    if receipt.evidence_id == expected.freeze_evidence_id {
        return Err(MigrationVerificationError::ReusesFreezeEvidenceId);
    }
    if receipt.evidence_id == expected.rehearsal_evidence_id {
        return Err(MigrationVerificationError::ReusesRehearsalEvidenceId);
    }
    if !receipt.routes.is_complete() {
        return Err(MigrationVerificationError::IncompleteRouteAccounting);
    }
    if !receipt.idempotency_verified {
        return Err(MigrationVerificationError::IdempotencyNotVerified);
    }
    if !receipt.conflict_free_verified {
        return Err(MigrationVerificationError::ConflictFreedomNotVerified);
    }
    if !receipt.source_destination_verified {
        return Err(MigrationVerificationError::SourceDestinationNotVerified);
    }
    if !receipt.payload_verified {
        return Err(MigrationVerificationError::PayloadNotVerified);
    }
    if !receipt.topology_verified {
        return Err(MigrationVerificationError::TopologyNotVerified);
    }
    if !receipt.historical_public_exposure_may_persist {
        return Err(MigrationVerificationError::HistoricalExposureNotAcknowledged);
    }
    Ok(VerifiedMigration {
        evidence_id: receipt.evidence_id,
        context: receipt.context,
        destination: receipt.destination,
        routes: receipt.routes,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationStatus {
    Active,
    Abandoned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RegistryEntry {
    migration: VerifiedMigration,
    status: MigrationStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegisterOutcome {
    Inserted,
    ExactIdempotentReplay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryError {
    EvidenceIdAlreadyUsed,
    SameGenerationConflict,
    MissingPreviousGeneration,
    PreviousGenerationAbandoned,
    PredecessorMismatch,
    FrozenLineageAbandoned,
    MigrationNotFound,
    MigrationAbandoned,
    NotLatestGeneration,
    ClockRollback,
    AbortNotQualified,
    AbortDeploymentMismatch,
    AbortMigrationMismatch,
    AbortDestinationMismatch,
    AbortNotYetValid,
    AbortExpired,
    AbortCleanupIncomplete,
    AbortHistoricalExposureNotAcknowledged,
}

#[derive(Clone, Debug, Default)]
pub struct MigrationRegistry {
    entries: Vec<RegistryEntry>,
    retired_freezes: Vec<(DeploymentIdentity, EvidenceId, LegacyStateDigest, u64)>,
    last_security_time_micros: Option<i64>,
}

impl MigrationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn freeze_key(context: MigrationContext) -> (DeploymentIdentity, EvidenceId, LegacyStateDigest, u64) {
        (
            context.deployment,
            context.freeze_evidence_id,
            context.legacy_state_digest,
            context.freeze_epoch,
        )
    }

    fn same_generation(a: VerifiedMigration, b: VerifiedMigration) -> bool {
        a.context == b.context && a.destination.generation == b.destination.generation
    }

    pub fn register(&mut self, migration: VerifiedMigration) -> Result<RegisterOutcome, RegistryError> {
        let freeze_key = Self::freeze_key(migration.context);
        if self.retired_freezes.contains(&freeze_key) {
            return Err(RegistryError::FrozenLineageAbandoned);
        }

        for entry in &self.entries {
            if entry.migration.evidence_id == migration.evidence_id && entry.migration != migration {
                return Err(RegistryError::EvidenceIdAlreadyUsed);
            }
            if Self::same_generation(entry.migration, migration) {
                if entry.status == MigrationStatus::Abandoned {
                    return Err(RegistryError::FrozenLineageAbandoned);
                }
                if entry.migration == migration {
                    return Ok(RegisterOutcome::ExactIdempotentReplay);
                }
                return Err(RegistryError::SameGenerationConflict);
            }
        }

        if migration.destination.generation > 1 {
            let previous_generation = migration.destination.generation - 1;
            let previous = self.entries.iter().find(|entry| {
                entry.migration.context == migration.context
                    && entry.migration.destination.generation == previous_generation
            });
            let Some(previous) = previous else {
                return Err(RegistryError::MissingPreviousGeneration);
            };
            if previous.status == MigrationStatus::Abandoned {
                return Err(RegistryError::PreviousGenerationAbandoned);
            }
            if migration.destination.predecessor_root != Some(previous.migration.destination.root_digest) {
                return Err(RegistryError::PredecessorMismatch);
            }
        }

        self.entries.push(RegistryEntry {
            migration,
            status: MigrationStatus::Active,
        });
        Ok(RegisterOutcome::Inserted)
    }

    fn latest_active_for_context(&self, context: MigrationContext) -> Option<&RegistryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.migration.context == context && entry.status == MigrationStatus::Active)
            .max_by_key(|entry| entry.migration.destination.generation)
    }

    pub fn activation_binding(
        &self,
        context: MigrationContext,
        evidence_id: EvidenceId,
    ) -> Result<ActivationMigrationBinding, RegistryError> {
        let Some(entry) = self.entries.iter().find(|entry| entry.migration.evidence_id == evidence_id) else {
            return Err(RegistryError::MigrationNotFound);
        };
        if entry.status == MigrationStatus::Abandoned {
            return Err(RegistryError::MigrationAbandoned);
        }
        if entry.migration.context != context {
            return Err(RegistryError::MigrationNotFound);
        }
        let latest = self.latest_active_for_context(context).ok_or(RegistryError::MigrationNotFound)?;
        if latest.migration.evidence_id != evidence_id {
            return Err(RegistryError::NotLatestGeneration);
        }
        Ok(ActivationMigrationBinding {
            migration_evidence_id: entry.migration.evidence_id,
            context: entry.migration.context,
            destination: entry.migration.destination,
            route_accounting_commitment: entry.migration.routes.commitment,
            contract_version: MIGRATION_RESULT_CONTRACT_V1,
        })
    }

    fn observe_security_time(&mut self, now_micros: i64) -> Result<(), RegistryError> {
        if self.last_security_time_micros.is_some_and(|last| now_micros < last) {
            return Err(RegistryError::ClockRollback);
        }
        self.last_security_time_micros = Some(now_micros);
        Ok(())
    }

    /// Mark an already materialized protected destination as abandoned. This is a
    /// quarantine/reachability theorem, not erasure: peer-retained ciphertext or
    /// historical DHT observations are explicitly outside the claim.
    pub fn abandon_after_migration(
        &mut self,
        receipt: PostMigrationAbortReceipt,
        now_micros: i64,
    ) -> Result<(), RegistryError> {
        self.observe_security_time(now_micros)?;
        if receipt.state != EvidenceState::Qualified {
            return Err(RegistryError::AbortNotQualified);
        }
        if now_micros < receipt.issued_at_micros {
            return Err(RegistryError::AbortNotYetValid);
        }
        if now_micros >= receipt.expires_at_micros {
            return Err(RegistryError::AbortExpired);
        }

        let Some(index) = self.entries.iter().position(|entry| {
            entry.migration.evidence_id == receipt.migration_evidence_id
        }) else {
            return Err(RegistryError::MigrationNotFound);
        };
        let migration = self.entries[index].migration;
        if receipt.deployment != migration.context.deployment {
            return Err(RegistryError::AbortDeploymentMismatch);
        }
        if receipt.migration_context != migration.context {
            return Err(RegistryError::AbortMigrationMismatch);
        }
        if receipt.destination != migration.destination {
            return Err(RegistryError::AbortDestinationMismatch);
        }
        if !(receipt.destination_quarantined
            && receipt.discovery_disabled
            && receipt.locator_inactive
            && receipt.capabilities_inactive
            && receipt.legacy_reenable_authorized)
        {
            return Err(RegistryError::AbortCleanupIncomplete);
        }
        if !receipt.historical_public_exposure_may_persist {
            return Err(RegistryError::AbortHistoricalExposureNotAcknowledged);
        }

        self.entries[index].status = MigrationStatus::Abandoned;
        let freeze_key = Self::freeze_key(migration.context);
        if !self.retired_freezes.contains(&freeze_key) {
            self.retired_freezes.push(freeze_key);
        }
        Ok(())
    }

    pub fn status(&self, evidence_id: EvidenceId) -> Option<MigrationStatus> {
        self.entries
            .iter()
            .find(|entry| entry.migration.evidence_id == evidence_id)
            .map(|entry| entry.status)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationMigrationBinding {
    migration_evidence_id: EvidenceId,
    context: MigrationContext,
    destination: DestinationIdentity,
    route_accounting_commitment: RouteAccountingDigest,
    contract_version: u16,
}

impl ActivationMigrationBinding {
    pub fn migration_evidence_id(&self) -> EvidenceId {
        self.migration_evidence_id
    }

    pub fn destination(&self) -> DestinationIdentity {
        self.destination
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostMigrationAbortReceipt {
    state: EvidenceState,
    authority_ref: AuthorityRef,
    deployment: DeploymentIdentity,
    migration_evidence_id: EvidenceId,
    migration_context: MigrationContext,
    destination: DestinationIdentity,
    quarantine_commitment: QuarantineCommitment,
    abort_reason_digest: AbortReasonDigest,
    destination_quarantined: bool,
    discovery_disabled: bool,
    locator_inactive: bool,
    capabilities_inactive: bool,
    legacy_reenable_authorized: bool,
    issued_at_micros: i64,
    expires_at_micros: i64,
    historical_public_exposure_may_persist: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostMigrationAbortReceiptError {
    InvalidTimeWindow,
}

impl PostMigrationAbortReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_adapter(
        state: EvidenceState,
        authority_ref: AuthorityRef,
        deployment: DeploymentIdentity,
        migration_evidence_id: EvidenceId,
        migration_context: MigrationContext,
        destination: DestinationIdentity,
        quarantine_commitment: QuarantineCommitment,
        abort_reason_digest: AbortReasonDigest,
        destination_quarantined: bool,
        discovery_disabled: bool,
        locator_inactive: bool,
        capabilities_inactive: bool,
        legacy_reenable_authorized: bool,
        issued_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, PostMigrationAbortReceiptError> {
        if expires_at_micros <= issued_at_micros {
            return Err(PostMigrationAbortReceiptError::InvalidTimeWindow);
        }
        Ok(Self {
            state,
            authority_ref,
            deployment,
            migration_evidence_id,
            migration_context,
            destination,
            quarantine_commitment,
            abort_reason_digest,
            destination_quarantined,
            discovery_disabled,
            locator_inactive,
            capabilities_inactive,
            legacy_reenable_authorized,
            issued_at_micros,
            expires_at_micros,
            historical_public_exposure_may_persist: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn eid(value: u8) -> EvidenceId {
        EvidenceId::new(b(value)).unwrap()
    }

    fn deployment(seed: u8) -> DeploymentIdentity {
        DeploymentIdentity::new(
            SourceCommit::new(b(seed)).unwrap(),
            DnaManifestDigest::new(b(seed + 1)).unwrap(),
            IntegrityWasmSetDigest::new(b(seed + 2)).unwrap(),
            CoordinatorWasmSetDigest::new(b(seed + 3)).unwrap(),
            ToolchainDigest::new(b(seed + 4)).unwrap(),
            1,
        )
        .unwrap()
    }

    fn plan() -> MigrationPlanRef {
        MigrationPlanRef::new(1, MigrationSchemaDigest::new(b(30)).unwrap()).unwrap()
    }

    fn context() -> MigrationContext {
        MigrationContext::new(
            deployment(10),
            eid(20),
            LegacyStateDigest::new(b(21)).unwrap(),
            7,
            eid(22),
            plan(),
        )
        .unwrap()
    }

    fn destination(generation: u64, root: u8, predecessor: Option<u8>) -> DestinationIdentity {
        DestinationIdentity::new(
            DestinationRootDigest::new(b(root)).unwrap(),
            ProtectedManifestDigest::new(b(root + 1)).unwrap(),
            5,
            9,
            generation,
            predecessor.map(|value| DestinationRootDigest::new(b(value)).unwrap()),
        )
        .unwrap()
    }

    fn routes() -> RouteAccounting {
        RouteAccounting::new(RouteAccountingDigest::new(b(40)).unwrap(), 7, 7, 0).unwrap()
    }

    fn receipt(
        evidence: u8,
        context: MigrationContext,
        destination: DestinationIdentity,
    ) -> ProductionMigrationReceipt {
        ProductionMigrationReceipt::from_verified_adapter(
            eid(evidence),
            EvidenceState::Qualified,
            context,
            destination,
            routes(),
            true,
            true,
            true,
            true,
            true,
        )
    }

    fn verified(evidence: u8, destination: DestinationIdentity) -> VerifiedMigration {
        verify_migration(context(), receipt(evidence, context(), destination)).unwrap()
    }

    fn abort_receipt(
        migration: VerifiedMigration,
        state: EvidenceState,
        all_cleanup: bool,
        start: i64,
        end: i64,
    ) -> PostMigrationAbortReceipt {
        PostMigrationAbortReceipt::from_verified_adapter(
            state,
            AuthorityRef::new(b(70)).unwrap(),
            migration.context.deployment,
            migration.evidence_id,
            migration.context,
            migration.destination,
            QuarantineCommitment::new(b(71)).unwrap(),
            AbortReasonDigest::new(b(72)).unwrap(),
            all_cleanup,
            all_cleanup,
            all_cleanup,
            all_cleanup,
            all_cleanup,
            start,
            end,
        )
        .unwrap()
    }

    #[test]
    fn nonqualified_receipt_denies() {
        let mut value = receipt(50, context(), destination(1, 50, None));
        value.state = EvidenceState::QueuedInfrastructure;
        assert_eq!(
            verify_migration(context(), value),
            Err(MigrationVerificationError::NotQualified(EvidenceState::QueuedInfrastructure))
        );
    }

    #[test]
    fn source_context_mismatch_denies() {
        let other = MigrationContext::new(
            deployment(11),
            eid(20),
            LegacyStateDigest::new(b(21)).unwrap(),
            7,
            eid(22),
            plan(),
        )
        .unwrap();
        assert_eq!(
            verify_migration(context(), receipt(50, other, destination(1, 50, None))),
            Err(MigrationVerificationError::ContextMismatch)
        );
    }

    #[test]
    fn unresolved_or_partial_routes_deny() {
        let mut value = receipt(50, context(), destination(1, 50, None));
        value.routes = RouteAccounting::new(RouteAccountingDigest::new(b(41)).unwrap(), 7, 6, 1).unwrap();
        assert_eq!(
            verify_migration(context(), value),
            Err(MigrationVerificationError::IncompleteRouteAccounting)
        );
    }

    #[test]
    fn payload_and_topology_are_independent_required_checks() {
        let mut payload = receipt(50, context(), destination(1, 50, None));
        payload.payload_verified = false;
        assert_eq!(verify_migration(context(), payload), Err(MigrationVerificationError::PayloadNotVerified));

        let mut topology = receipt(51, context(), destination(1, 50, None));
        topology.topology_verified = false;
        assert_eq!(verify_migration(context(), topology), Err(MigrationVerificationError::TopologyNotVerified));
    }

    #[test]
    fn exact_registration_retry_is_idempotent() {
        let migration = verified(50, destination(1, 50, None));
        let mut registry = MigrationRegistry::new();
        assert_eq!(registry.register(migration), Ok(RegisterOutcome::Inserted));
        assert_eq!(registry.register(migration), Ok(RegisterOutcome::ExactIdempotentReplay));
    }

    #[test]
    fn different_destination_same_generation_is_conflict() {
        let mut registry = MigrationRegistry::new();
        registry.register(verified(50, destination(1, 50, None))).unwrap();
        let conflict = verified(51, destination(1, 60, None));
        assert_eq!(registry.register(conflict), Err(RegistryError::SameGenerationConflict));
    }

    #[test]
    fn later_generation_requires_exact_active_predecessor() {
        let mut registry = MigrationRegistry::new();
        registry.register(verified(50, destination(1, 50, None))).unwrap();

        let wrong = verified(51, destination(2, 60, Some(55)));
        assert_eq!(registry.register(wrong), Err(RegistryError::PredecessorMismatch));

        let good = verified(52, destination(2, 60, Some(50)));
        assert_eq!(registry.register(good), Ok(RegisterOutcome::Inserted));
    }

    #[test]
    fn only_latest_generation_is_activation_eligible() {
        let context = context();
        let mut registry = MigrationRegistry::new();
        registry.register(verified(50, destination(1, 50, None))).unwrap();
        registry.register(verified(52, destination(2, 60, Some(50)))).unwrap();

        assert_eq!(
            registry.activation_binding(context, eid(50)),
            Err(RegistryError::NotLatestGeneration)
        );
        assert_eq!(
            registry.activation_binding(context, eid(52)).unwrap().migration_evidence_id(),
            eid(52)
        );
    }

    #[test]
    fn post_migration_abort_requires_cleanup_and_exact_binding() {
        let migration = verified(50, destination(1, 50, None));
        let mut registry = MigrationRegistry::new();
        registry.register(migration).unwrap();

        let incomplete = abort_receipt(migration, EvidenceState::Qualified, false, 10, 100);
        assert_eq!(
            registry.abandon_after_migration(incomplete, 50),
            Err(RegistryError::AbortCleanupIncomplete)
        );

        let exact = abort_receipt(migration, EvidenceState::Qualified, true, 10, 100);
        registry.abandon_after_migration(exact, 50).unwrap();
        assert_eq!(registry.status(eid(50)), Some(MigrationStatus::Abandoned));
        assert_eq!(
            registry.activation_binding(context(), eid(50)),
            Err(RegistryError::MigrationAbandoned)
        );
    }

    #[test]
    fn abandoned_freeze_lineage_cannot_be_reused() {
        let migration = verified(50, destination(1, 50, None));
        let mut registry = MigrationRegistry::new();
        registry.register(migration).unwrap();
        registry
            .abandon_after_migration(
                abort_receipt(migration, EvidenceState::Qualified, true, 10, 100),
                50,
            )
            .unwrap();

        assert_eq!(
            registry.register(migration),
            Err(RegistryError::FrozenLineageAbandoned)
        );
        let new_destination = verified(51, destination(1, 60, None));
        assert_eq!(
            registry.register(new_destination),
            Err(RegistryError::FrozenLineageAbandoned)
        );
    }

    #[test]
    fn abort_expiry_cannot_be_bypassed_by_clock_rollback() {
        let migration = verified(50, destination(1, 50, None));
        let mut registry = MigrationRegistry::new();
        registry.register(migration).unwrap();
        let receipt = abort_receipt(migration, EvidenceState::Qualified, true, 10, 100);

        assert_eq!(registry.abandon_after_migration(receipt, 100), Err(RegistryError::AbortExpired));
        assert_eq!(registry.abandon_after_migration(receipt, 99), Err(RegistryError::ClockRollback));
    }

    #[test]
    fn migration_evidence_cannot_reuse_freeze_or_rehearsal_identity() {
        let mut freeze = receipt(20, context(), destination(1, 50, None));
        freeze.evidence_id = context().freeze_evidence_id;
        assert_eq!(verify_migration(context(), freeze), Err(MigrationVerificationError::ReusesFreezeEvidenceId));

        let rehearsal = receipt(22, context(), destination(1, 50, None));
        assert_eq!(verify_migration(context(), rehearsal), Err(MigrationVerificationError::ReusesRehearsalEvidenceId));
    }

    #[test]
    fn opaque_identifiers_reject_zero_and_redact() {
        assert_eq!(DestinationRootDigest::new([0; 32]), Err(OpaqueIdError::AllZero));
        assert_eq!(format!("{:?}", DestinationRootDigest::new(b(9)).unwrap()), "DestinationRootDigest([redacted])");
    }
}
