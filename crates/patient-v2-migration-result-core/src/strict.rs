use crate::model;

pub use crate::model::{
    verify_migration, AbortReasonDigest, ActivationMigrationBinding, AuthorityRef,
    CoordinatorWasmSetDigest, DeploymentIdentity, DeploymentIdentityError, DestinationIdentity,
    DestinationIdentityError, DestinationRootDigest, DnaManifestDigest, EvidenceId, EvidenceState,
    IntegrityWasmSetDigest, LegacyStateDigest, MigrationContext, MigrationContextError,
    MigrationPlanError, MigrationPlanRef, MigrationSchemaDigest, MigrationStatus,
    MigrationVerificationError, OpaqueIdError, PostMigrationAbortReceipt,
    PostMigrationAbortReceiptError, ProductionMigrationReceipt, ProtectedManifestDigest,
    QuarantineCommitment, RegisterOutcome, RegistryError, RouteAccounting, RouteAccountingDigest,
    RouteAccountingError, SourceCommit, ToolchainDigest, VerifiedMigration,
    MIGRATION_RESULT_CONTRACT_V1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrictRegistryError {
    ContextMismatch,
    FrozenLineageAbandoned,
    Inner(RegistryError),
}

impl From<RegistryError> for StrictRegistryError {
    fn from(value: RegistryError) -> Self {
        Self::Inner(value)
    }
}

/// A registry is intentionally scoped to exactly one frozen migration context.
/// This prevents one freeze/state digest from quietly accumulating alternate
/// rehearsal/plan lineages through a generic multi-context registry.
#[derive(Clone, Debug)]
pub struct MigrationRegistry {
    context: MigrationContext,
    inner: model::MigrationRegistry,
    abandoned: bool,
}

impl MigrationRegistry {
    pub fn for_context(context: MigrationContext) -> Self {
        Self {
            context,
            inner: model::MigrationRegistry::new(),
            abandoned: false,
        }
    }

    pub fn context(&self) -> MigrationContext {
        self.context
    }

    pub fn is_abandoned(&self) -> bool {
        self.abandoned
    }

    pub fn register(
        &mut self,
        migration: VerifiedMigration,
    ) -> Result<RegisterOutcome, StrictRegistryError> {
        if self.abandoned {
            return Err(StrictRegistryError::FrozenLineageAbandoned);
        }
        if migration.context() != self.context {
            return Err(StrictRegistryError::ContextMismatch);
        }
        self.inner.register(migration).map_err(Into::into)
    }

    pub fn activation_binding(
        &self,
        evidence_id: EvidenceId,
    ) -> Result<ActivationMigrationBinding, StrictRegistryError> {
        if self.abandoned {
            return Err(StrictRegistryError::FrozenLineageAbandoned);
        }
        self.inner
            .activation_binding(self.context, evidence_id)
            .map_err(Into::into)
    }

    pub fn abandon_after_migration(
        &mut self,
        receipt: PostMigrationAbortReceipt,
        now_micros: i64,
    ) -> Result<(), StrictRegistryError> {
        if self.abandoned {
            return Err(StrictRegistryError::FrozenLineageAbandoned);
        }
        self.inner
            .abandon_after_migration(receipt, now_micros)
            .map_err(StrictRegistryError::Inner)?;
        self.abandoned = true;
        Ok(())
    }

    pub fn status(&self, evidence_id: EvidenceId) -> Option<MigrationStatus> {
        self.inner.status(evidence_id)
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

    fn context(seed: u8) -> MigrationContext {
        MigrationContext::new(
            deployment(seed),
            eid(20),
            LegacyStateDigest::new(b(21)).unwrap(),
            7,
            eid(22),
            MigrationPlanRef::new(1, MigrationSchemaDigest::new(b(30)).unwrap()).unwrap(),
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
            predecessor.map(|v| DestinationRootDigest::new(b(v)).unwrap()),
        )
        .unwrap()
    }

    fn migration(context: MigrationContext, evidence: u8, destination: DestinationIdentity) -> VerifiedMigration {
        let receipt = ProductionMigrationReceipt::from_verified_adapter(
            eid(evidence),
            EvidenceState::Qualified,
            context,
            destination,
            RouteAccounting::new(RouteAccountingDigest::new(b(40)).unwrap(), 7, 7, 0).unwrap(),
            true,
            true,
            true,
            true,
            true,
        );
        verify_migration(context, receipt).unwrap()
    }

    fn abort_receipt(migration: VerifiedMigration) -> PostMigrationAbortReceipt {
        PostMigrationAbortReceipt::from_verified_adapter(
            EvidenceState::Qualified,
            AuthorityRef::new(b(70)).unwrap(),
            migration.context().deployment(),
            migration.evidence_id(),
            migration.context(),
            migration.destination(),
            QuarantineCommitment::new(b(71)).unwrap(),
            AbortReasonDigest::new(b(72)).unwrap(),
            true,
            true,
            true,
            true,
            true,
            10,
            100,
        )
        .unwrap()
    }

    #[test]
    fn alternate_context_is_rejected() {
        let context_a = context(10);
        let context_b = context(11);
        let mut registry = MigrationRegistry::for_context(context_a);
        let other = migration(context_b, 50, destination(1, 50, None));
        assert_eq!(registry.register(other), Err(StrictRegistryError::ContextMismatch));
    }

    #[test]
    fn abandoned_lineage_never_reveals_older_generation_as_activatable() {
        let context = context(10);
        let gen1 = migration(context, 50, destination(1, 50, None));
        let gen2 = migration(context, 51, destination(2, 60, Some(50)));
        let mut registry = MigrationRegistry::for_context(context);
        registry.register(gen1).unwrap();
        registry.register(gen2).unwrap();
        registry
            .abandon_after_migration(abort_receipt(gen2), 50)
            .unwrap();

        assert!(registry.is_abandoned());
        assert_eq!(
            registry.activation_binding(gen1.evidence_id()),
            Err(StrictRegistryError::FrozenLineageAbandoned)
        );
        assert_eq!(
            registry.activation_binding(gen2.evidence_id()),
            Err(StrictRegistryError::FrozenLineageAbandoned)
        );
    }

    #[test]
    fn abandoned_lineage_cannot_register_any_further_generation() {
        let context = context(10);
        let gen1 = migration(context, 50, destination(1, 50, None));
        let mut registry = MigrationRegistry::for_context(context);
        registry.register(gen1).unwrap();
        registry
            .abandon_after_migration(abort_receipt(gen1), 50)
            .unwrap();

        let retry = migration(context, 50, destination(1, 50, None));
        assert_eq!(
            registry.register(retry),
            Err(StrictRegistryError::FrozenLineageAbandoned)
        );
    }
}
