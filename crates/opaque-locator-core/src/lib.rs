#![forbid(unsafe_code)]
//! Reference semantics for rotatable opaque care locators and protected manifests.
//!
//! This crate does not encrypt, hash, sign, or store Holochain entries. It
//! freezes the discovery and rotation invariants required before Patient v2 can
//! replace clear patient/DID/global-directory topology.

use core::fmt;

pub const OPAQUE_LOCATOR_CONTRACT_V1: u16 = 1;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocatorId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManifestEnvelopeId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnvelopeId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextHandle([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyHandle([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityRef([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectRef([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AudienceRef([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProvenanceRef([u8; 32]);

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

opaque32!(LocatorId);
opaque32!(ManifestEnvelopeId);
opaque32!(EnvelopeId);
opaque32!(ContextHandle);
opaque32!(PolicyHandle);
opaque32!(CapabilityRef);
opaque32!(SubjectRef);
opaque32!(AudienceRef);
opaque32!(ProvenanceRef);

/// The complete public discovery edge.
///
/// Deliberately absent: subject/patient ID, DID, purpose, audience, record kind,
/// category, exact expiry, and selected-record list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicLocatorEdgeV1 {
    pub contract_version: u16,
    pub locator: LocatorId,
    pub manifest_envelope: ManifestEnvelopeId,
}

impl PublicLocatorEdgeV1 {
    pub fn new(locator: LocatorId, manifest_envelope: ManifestEnvelopeId) -> Self {
        Self {
            contract_version: OPAQUE_LOCATOR_CONTRACT_V1,
            locator,
            manifest_envelope,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarePurpose {
    Treatment,
    CareCoordination,
    EmergencySupport,
    SpiritualCare,
    PersonalReflection,
    Research,
    ModelEvaluation,
    SymthaeaTask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestCompleteness {
    CompleteForProjection,
    Partial,
}

/// Private material delivered only through a separately authorized capability
/// path. Intentionally has no Debug implementation.
#[derive(Clone, PartialEq, Eq)]
pub struct PrivateLocatorGrantV1 {
    pub locator: LocatorId,
    pub expected_manifest: ManifestEnvelopeId,
    pub expected_context: ContextHandle,
    pub policy: PolicyHandle,
    pub capability: CapabilityRef,
    pub audience: AudienceRef,
    pub purpose: CarePurpose,
    pub key_epoch: u64,
    pub generation: u64,
    pub issued_at_micros: i64,
    pub expires_at_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantError {
    InvalidTimeWindow,
    ZeroKeyEpoch,
    ZeroGeneration,
}

impl PrivateLocatorGrantV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        locator: LocatorId,
        expected_manifest: ManifestEnvelopeId,
        expected_context: ContextHandle,
        policy: PolicyHandle,
        capability: CapabilityRef,
        audience: AudienceRef,
        purpose: CarePurpose,
        key_epoch: u64,
        generation: u64,
        issued_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, GrantError> {
        if expires_at_micros <= issued_at_micros {
            return Err(GrantError::InvalidTimeWindow);
        }
        if key_epoch == 0 {
            return Err(GrantError::ZeroKeyEpoch);
        }
        if generation == 0 {
            return Err(GrantError::ZeroGeneration);
        }
        Ok(Self {
            locator,
            expected_manifest,
            expected_context,
            policy,
            capability,
            audience,
            purpose,
            key_epoch,
            generation,
            issued_at_micros,
            expires_at_micros,
        })
    }
}

/// Public-header fields from the protected manifest envelope that the resolver
/// must bind to the private locator grant before attempting decryption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvelopeHeaderBinding {
    pub context: ContextHandle,
    pub policy: PolicyHandle,
    pub key_epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionError {
    NotYetValid,
    Expired,
    LocatorMismatch,
    ManifestMismatch,
    ContextMismatch,
    PolicyMismatch,
    KeyEpochMismatch,
}

pub fn validate_resolution(
    grant: &PrivateLocatorGrantV1,
    edge: &PublicLocatorEdgeV1,
    header: EnvelopeHeaderBinding,
    now_micros: i64,
) -> Result<(), ResolutionError> {
    if now_micros < grant.issued_at_micros {
        return Err(ResolutionError::NotYetValid);
    }
    if now_micros >= grant.expires_at_micros {
        return Err(ResolutionError::Expired);
    }
    if edge.locator != grant.locator {
        return Err(ResolutionError::LocatorMismatch);
    }
    if edge.manifest_envelope != grant.expected_manifest {
        return Err(ResolutionError::ManifestMismatch);
    }
    if header.context != grant.expected_context {
        return Err(ResolutionError::ContextMismatch);
    }
    if header.policy != grant.policy {
        return Err(ResolutionError::PolicyMismatch);
    }
    if header.key_epoch != grant.key_epoch {
        return Err(ResolutionError::KeyEpochMismatch);
    }
    Ok(())
}

/// Protected manifest plaintext. This type intentionally has no Debug
/// implementation because it contains private discovery semantics.
#[derive(Clone, PartialEq, Eq)]
pub struct ProtectedManifestV1 {
    pub subject: SubjectRef,
    pub context: ContextHandle,
    pub policy: PolicyHandle,
    pub capability: CapabilityRef,
    pub audience: AudienceRef,
    pub purpose: CarePurpose,
    pub provenance: ProvenanceRef,
    pub generation: u64,
    pub issued_at_micros: i64,
    pub expires_at_micros: i64,
    pub completeness: ManifestCompleteness,
    pub objects: Vec<EnvelopeId>,
    pub predecessor_manifest: Option<ManifestEnvelopeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestError {
    ZeroGeneration,
    InvalidTimeWindow,
    EmptySelector,
    DuplicateObject,
    GenesisHasPredecessor,
    NonGenesisMissingPredecessor,
}

impl ProtectedManifestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subject: SubjectRef,
        context: ContextHandle,
        policy: PolicyHandle,
        capability: CapabilityRef,
        audience: AudienceRef,
        purpose: CarePurpose,
        provenance: ProvenanceRef,
        generation: u64,
        issued_at_micros: i64,
        expires_at_micros: i64,
        completeness: ManifestCompleteness,
        objects: Vec<EnvelopeId>,
        predecessor_manifest: Option<ManifestEnvelopeId>,
    ) -> Result<Self, ManifestError> {
        if generation == 0 {
            return Err(ManifestError::ZeroGeneration);
        }
        if expires_at_micros <= issued_at_micros {
            return Err(ManifestError::InvalidTimeWindow);
        }
        if objects.is_empty() {
            return Err(ManifestError::EmptySelector);
        }
        for (index, object) in objects.iter().enumerate() {
            if objects[index + 1..].contains(object) {
                return Err(ManifestError::DuplicateObject);
            }
        }
        if generation == 1 && predecessor_manifest.is_some() {
            return Err(ManifestError::GenesisHasPredecessor);
        }
        if generation > 1 && predecessor_manifest.is_none() {
            return Err(ManifestError::NonGenesisMissingPredecessor);
        }
        Ok(Self {
            subject,
            context,
            policy,
            capability,
            audience,
            purpose,
            provenance,
            generation,
            issued_at_micros,
            expires_at_micros,
            completeness,
            objects,
            predecessor_manifest,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocatorRegistration {
    Created,
    Idempotent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocatorRegistryError {
    CrossContextReuse,
}

#[derive(Default)]
pub struct LocatorRegistry {
    registrations: Vec<(LocatorId, ContextHandle)>,
}

impl LocatorRegistry {
    pub fn register(
        &mut self,
        locator: LocatorId,
        context: ContextHandle,
    ) -> Result<LocatorRegistration, LocatorRegistryError> {
        if let Some((_, existing_context)) = self
            .registrations
            .iter()
            .find(|(existing_locator, _)| *existing_locator == locator)
        {
            return if *existing_context == context {
                Ok(LocatorRegistration::Idempotent)
            } else {
                Err(LocatorRegistryError::CrossContextReuse)
            };
        }
        self.registrations.push((locator, context));
        Ok(LocatorRegistration::Created)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestRegistration {
    Created,
    Idempotent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestRegistryError {
    ConflictingGeneration,
    MissingPriorGeneration,
    PredecessorMismatch,
}

#[derive(Default)]
pub struct ManifestRegistry {
    registrations: Vec<(ContextHandle, u64, ManifestEnvelopeId)>,
}

impl ManifestRegistry {
    pub fn register(
        &mut self,
        manifest_id: ManifestEnvelopeId,
        manifest: &ProtectedManifestV1,
    ) -> Result<ManifestRegistration, ManifestRegistryError> {
        if let Some((_, _, existing_id)) = self.registrations.iter().find(|(context, generation, _)| {
            *context == manifest.context && *generation == manifest.generation
        }) {
            return if *existing_id == manifest_id {
                Ok(ManifestRegistration::Idempotent)
            } else {
                Err(ManifestRegistryError::ConflictingGeneration)
            };
        }

        if manifest.generation > 1 {
            let prior = self.registrations.iter().find(|(context, generation, _)| {
                *context == manifest.context && *generation == manifest.generation - 1
            });
            let Some((_, _, prior_id)) = prior else {
                return Err(ManifestRegistryError::MissingPriorGeneration);
            };
            if manifest.predecessor_manifest != Some(*prior_id) {
                return Err(ManifestRegistryError::PredecessorMismatch);
            }
        }

        self.registrations
            .push((manifest.context, manifest.generation, manifest_id));
        Ok(ManifestRegistration::Created)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationError {
    GenerationNotIncreasing,
    ExpiryExtended,
    InvalidGrant,
}

/// Rotate within the same authorization context. The API deliberately does not
/// accept new context/policy/capability/audience/purpose values, so changing any
/// of those requires a fresh authority proof and a new grant.
pub fn rotate_locator(
    current: &PrivateLocatorGrantV1,
    new_locator: LocatorId,
    new_manifest: ManifestEnvelopeId,
    new_generation: u64,
    issued_at_micros: i64,
    expires_at_micros: i64,
) -> Result<PrivateLocatorGrantV1, RotationError> {
    if new_generation <= current.generation {
        return Err(RotationError::GenerationNotIncreasing);
    }
    if expires_at_micros > current.expires_at_micros {
        return Err(RotationError::ExpiryExtended);
    }
    PrivateLocatorGrantV1::new(
        new_locator,
        new_manifest,
        current.expected_context,
        current.policy,
        current.capability,
        current.audience,
        current.purpose,
        current.key_epoch,
        new_generation,
        issued_at_micros,
        expires_at_micros,
    )
    .map_err(|_| RotationError::InvalidGrant)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(value: u8) -> [u8; 32] {
        [value; 32]
    }
    fn locator(value: u8) -> LocatorId {
        LocatorId::new(bytes(value)).unwrap()
    }
    fn manifest_id(value: u8) -> ManifestEnvelopeId {
        ManifestEnvelopeId::new(bytes(value)).unwrap()
    }
    fn envelope(value: u8) -> EnvelopeId {
        EnvelopeId::new(bytes(value)).unwrap()
    }
    fn context(value: u8) -> ContextHandle {
        ContextHandle::new(bytes(value)).unwrap()
    }
    fn policy(value: u8) -> PolicyHandle {
        PolicyHandle::new(bytes(value)).unwrap()
    }
    fn capability(value: u8) -> CapabilityRef {
        CapabilityRef::new(bytes(value)).unwrap()
    }
    fn subject(value: u8) -> SubjectRef {
        SubjectRef::new(bytes(value)).unwrap()
    }
    fn audience(value: u8) -> AudienceRef {
        AudienceRef::new(bytes(value)).unwrap()
    }
    fn provenance(value: u8) -> ProvenanceRef {
        ProvenanceRef::new(bytes(value)).unwrap()
    }

    fn grant() -> PrivateLocatorGrantV1 {
        PrivateLocatorGrantV1::new(
            locator(1),
            manifest_id(2),
            context(3),
            policy(4),
            capability(5),
            audience(6),
            CarePurpose::Treatment,
            7,
            1,
            100,
            1000,
        )
        .unwrap()
    }

    fn manifest(generation: u64, predecessor: Option<ManifestEnvelopeId>) -> ProtectedManifestV1 {
        ProtectedManifestV1::new(
            subject(10),
            context(3),
            policy(4),
            capability(5),
            audience(6),
            CarePurpose::Treatment,
            provenance(11),
            generation,
            100,
            1000,
            ManifestCompleteness::CompleteForProjection,
            vec![envelope(12), envelope(13)],
            predecessor,
        )
        .unwrap()
    }

    #[test]
    fn public_edge_is_minimal_and_versioned() {
        let edge = PublicLocatorEdgeV1::new(locator(1), manifest_id(2));
        assert_eq!(edge.contract_version, OPAQUE_LOCATOR_CONTRACT_V1);
        assert_eq!(edge.locator, locator(1));
        assert_eq!(edge.manifest_envelope, manifest_id(2));
    }

    #[test]
    fn all_zero_opaque_ids_are_rejected_and_debug_is_redacted() {
        assert_eq!(LocatorId::new([0; 32]), Err(OpaqueIdError::AllZero));
        assert_eq!(format!("{:?}", locator(1)), "LocatorId([redacted])");
        assert_eq!(format!("{:?}", context(3)), "ContextHandle([redacted])");
    }

    #[test]
    fn exact_resolution_succeeds() {
        let grant = grant();
        let edge = PublicLocatorEdgeV1::new(grant.locator, grant.expected_manifest);
        let header = EnvelopeHeaderBinding {
            context: grant.expected_context,
            policy: grant.policy,
            key_epoch: grant.key_epoch,
        };
        assert_eq!(validate_resolution(&grant, &edge, header, 500), Ok(()));
    }

    #[test]
    fn substituted_locator_or_manifest_is_rejected() {
        let grant = grant();
        let wrong_locator = PublicLocatorEdgeV1::new(locator(9), grant.expected_manifest);
        let header = EnvelopeHeaderBinding {
            context: grant.expected_context,
            policy: grant.policy,
            key_epoch: grant.key_epoch,
        };
        assert_eq!(
            validate_resolution(&grant, &wrong_locator, header, 500),
            Err(ResolutionError::LocatorMismatch)
        );
        let wrong_manifest = PublicLocatorEdgeV1::new(grant.locator, manifest_id(9));
        assert_eq!(
            validate_resolution(&grant, &wrong_manifest, header, 500),
            Err(ResolutionError::ManifestMismatch)
        );
    }

    #[test]
    fn substituted_context_policy_or_epoch_is_rejected() {
        let grant = grant();
        let edge = PublicLocatorEdgeV1::new(grant.locator, grant.expected_manifest);
        assert_eq!(
            validate_resolution(
                &grant,
                &edge,
                EnvelopeHeaderBinding {
                    context: context(9),
                    policy: grant.policy,
                    key_epoch: grant.key_epoch,
                },
                500,
            ),
            Err(ResolutionError::ContextMismatch)
        );
        assert_eq!(
            validate_resolution(
                &grant,
                &edge,
                EnvelopeHeaderBinding {
                    context: grant.expected_context,
                    policy: policy(9),
                    key_epoch: grant.key_epoch,
                },
                500,
            ),
            Err(ResolutionError::PolicyMismatch)
        );
        assert_eq!(
            validate_resolution(
                &grant,
                &edge,
                EnvelopeHeaderBinding {
                    context: grant.expected_context,
                    policy: grant.policy,
                    key_epoch: 99,
                },
                500,
            ),
            Err(ResolutionError::KeyEpochMismatch)
        );
    }

    #[test]
    fn locator_grant_is_time_bounded() {
        let grant = grant();
        let edge = PublicLocatorEdgeV1::new(grant.locator, grant.expected_manifest);
        let header = EnvelopeHeaderBinding {
            context: grant.expected_context,
            policy: grant.policy,
            key_epoch: grant.key_epoch,
        };
        assert_eq!(
            validate_resolution(&grant, &edge, header, 99),
            Err(ResolutionError::NotYetValid)
        );
        assert_eq!(
            validate_resolution(&grant, &edge, header, 1000),
            Err(ResolutionError::Expired)
        );
    }

    #[test]
    fn manifest_rejects_empty_and_duplicate_selectors() {
        let empty = ProtectedManifestV1::new(
            subject(10), context(3), policy(4), capability(5), audience(6),
            CarePurpose::Treatment, provenance(11), 1, 100, 1000,
            ManifestCompleteness::CompleteForProjection, vec![], None,
        );
        assert!(matches!(empty, Err(ManifestError::EmptySelector)));

        let duplicate = ProtectedManifestV1::new(
            subject(10), context(3), policy(4), capability(5), audience(6),
            CarePurpose::Treatment, provenance(11), 1, 100, 1000,
            ManifestCompleteness::CompleteForProjection,
            vec![envelope(12), envelope(12)], None,
        );
        assert!(matches!(duplicate, Err(ManifestError::DuplicateObject)));
    }

    #[test]
    fn manifest_lineage_requires_predecessor_after_genesis() {
        let bad_genesis = ProtectedManifestV1::new(
            subject(10), context(3), policy(4), capability(5), audience(6),
            CarePurpose::Treatment, provenance(11), 1, 100, 1000,
            ManifestCompleteness::CompleteForProjection,
            vec![envelope(12)], Some(manifest_id(1)),
        );
        assert!(matches!(bad_genesis, Err(ManifestError::GenesisHasPredecessor)));

        let bad_second = ProtectedManifestV1::new(
            subject(10), context(3), policy(4), capability(5), audience(6),
            CarePurpose::Treatment, provenance(11), 2, 100, 1000,
            ManifestCompleteness::CompleteForProjection,
            vec![envelope(12)], None,
        );
        assert!(matches!(bad_second, Err(ManifestError::NonGenesisMissingPredecessor)));
    }

    #[test]
    fn locator_cannot_be_reused_across_contexts() {
        let mut registry = LocatorRegistry::default();
        assert_eq!(registry.register(locator(1), context(3)), Ok(LocatorRegistration::Created));
        assert_eq!(registry.register(locator(1), context(3)), Ok(LocatorRegistration::Idempotent));
        assert_eq!(
            registry.register(locator(1), context(9)),
            Err(LocatorRegistryError::CrossContextReuse)
        );
    }

    #[test]
    fn manifest_registration_is_idempotent_but_conflicts_are_rejected() {
        let mut registry = ManifestRegistry::default();
        let first = manifest(1, None);
        assert_eq!(
            registry.register(manifest_id(20), &first),
            Ok(ManifestRegistration::Created)
        );
        assert_eq!(
            registry.register(manifest_id(20), &first),
            Ok(ManifestRegistration::Idempotent)
        );
        assert_eq!(
            registry.register(manifest_id(21), &first),
            Err(ManifestRegistryError::ConflictingGeneration)
        );
    }

    #[test]
    fn manifest_generation_requires_exact_prior_manifest() {
        let mut registry = ManifestRegistry::default();
        let first = manifest(1, None);
        registry.register(manifest_id(20), &first).unwrap();

        let wrong_second = manifest(2, Some(manifest_id(99)));
        assert_eq!(
            registry.register(manifest_id(21), &wrong_second),
            Err(ManifestRegistryError::PredecessorMismatch)
        );

        let good_second = manifest(2, Some(manifest_id(20)));
        assert_eq!(
            registry.register(manifest_id(21), &good_second),
            Ok(ManifestRegistration::Created)
        );
    }

    #[test]
    fn out_of_order_manifest_generation_is_rejected() {
        let mut registry = ManifestRegistry::default();
        let second = manifest(2, Some(manifest_id(20)));
        assert_eq!(
            registry.register(manifest_id(21), &second),
            Err(ManifestRegistryError::MissingPriorGeneration)
        );
    }

    #[test]
    fn rotation_preserves_authority_context_and_cannot_extend_expiry() {
        let current = grant();
        let rotated = rotate_locator(&current, locator(8), manifest_id(9), 2, 200, 900).unwrap();
        assert_eq!(rotated.expected_context, current.expected_context);
        assert_eq!(rotated.policy, current.policy);
        assert_eq!(rotated.capability, current.capability);
        assert_eq!(rotated.audience, current.audience);
        assert_eq!(rotated.purpose, current.purpose);
        assert_eq!(rotated.key_epoch, current.key_epoch);
        assert_eq!(rotated.generation, 2);
        assert_eq!(
            rotate_locator(&current, locator(8), manifest_id(9), 2, 200, 1001),
            Err(RotationError::ExpiryExtended)
        );
    }

    #[test]
    fn rotation_generation_must_increase() {
        let current = grant();
        assert_eq!(
            rotate_locator(&current, locator(8), manifest_id(9), 1, 200, 900),
            Err(RotationError::GenerationNotIncreasing)
        );
    }

    #[test]
    fn private_grant_and_manifest_do_not_implement_debug_by_design() {
        // Compile-time property: neither private type derives Debug. This test
        // verifies the public opaque IDs remain redacted instead of requiring
        // sensitive private structures to be formattable.
        assert_eq!(format!("{:?}", manifest_id(2)), "ManifestEnvelopeId([redacted])");
    }

    #[test]
    fn same_person_can_use_distinct_locators_in_distinct_contexts() {
        let mut registry = LocatorRegistry::default();
        assert_eq!(registry.register(locator(1), context(3)), Ok(LocatorRegistration::Created));
        assert_eq!(registry.register(locator(2), context(4)), Ok(LocatorRegistration::Created));
    }
}
