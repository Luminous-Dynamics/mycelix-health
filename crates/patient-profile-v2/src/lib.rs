#![forbid(unsafe_code)]
//! Protected patient-profile v2 reference semantics.
//!
//! This crate does not encrypt data or write Holochain entries. It freezes the
//! decomposition and migration rules required before the public v1 `Patient`
//! aggregate can be replaced safely.

use core::fmt;

pub const PATIENT_PROFILE_V2: u16 = 2;
pub const LEGACY_PATIENT_V1_FIELD_COUNT: usize = 18;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LegacySourceDigest([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvelopeId([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextHandle([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PolicyHandle([u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentityBindingRef([u8; 32]);

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

opaque32!(LegacySourceDigest);
opaque32!(EnvelopeId);
opaque32!(ContextHandle);
opaque32!(PolicyHandle);
opaque32!(IdentityBindingRef);

/// Every field in the current public v1 `Patient` entry.
///
/// Adding another legacy field to a future migration adapter should require a
/// deliberate routing decision rather than being silently dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LegacyPatientField {
    PatientId,
    Mrn,
    FirstName,
    LastName,
    DateOfBirth,
    BiologicalSex,
    GenderIdentity,
    BloodType,
    Contact,
    EmergencyContact,
    PrimaryLanguage,
    Allergies,
    Conditions,
    Medications,
    MycelixIdentityHash,
    MatlTrustScore,
    CreatedAt,
    UpdatedAt,
}

impl LegacyPatientField {
    pub const ALL: [Self; LEGACY_PATIENT_V1_FIELD_COUNT] = [
        Self::PatientId,
        Self::Mrn,
        Self::FirstName,
        Self::LastName,
        Self::DateOfBirth,
        Self::BiologicalSex,
        Self::GenderIdentity,
        Self::BloodType,
        Self::Contact,
        Self::EmergencyContact,
        Self::PrimaryLanguage,
        Self::Allergies,
        Self::Conditions,
        Self::Medications,
        Self::MycelixIdentityHash,
        Self::MatlTrustScore,
        Self::CreatedAt,
        Self::UpdatedAt,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldRoute {
    ProtectedIdentifier,
    ProtectedDemographic,
    ProtectedSourcedClinicalDatum,
    ProtectedCommunicationProfile,
    AllergyDomainMigration,
    ConditionDomainMigration,
    MedicationDomainMigration,
    ProtectedIdentityBindingMigration,
    LegacyReliabilityObservation,
    ProtectedProvenance,
}

/// Exhaustive v1→v2 routing policy.
pub const fn route_legacy_field(field: LegacyPatientField) -> FieldRoute {
    match field {
        LegacyPatientField::PatientId | LegacyPatientField::Mrn => {
            FieldRoute::ProtectedIdentifier
        }
        LegacyPatientField::FirstName
        | LegacyPatientField::LastName
        | LegacyPatientField::DateOfBirth
        | LegacyPatientField::BiologicalSex
        | LegacyPatientField::GenderIdentity => FieldRoute::ProtectedDemographic,
        LegacyPatientField::BloodType => FieldRoute::ProtectedSourcedClinicalDatum,
        LegacyPatientField::Contact
        | LegacyPatientField::EmergencyContact
        | LegacyPatientField::PrimaryLanguage => FieldRoute::ProtectedCommunicationProfile,
        LegacyPatientField::Allergies => FieldRoute::AllergyDomainMigration,
        LegacyPatientField::Conditions => FieldRoute::ConditionDomainMigration,
        LegacyPatientField::Medications => FieldRoute::MedicationDomainMigration,
        LegacyPatientField::MycelixIdentityHash => {
            FieldRoute::ProtectedIdentityBindingMigration
        }
        LegacyPatientField::MatlTrustScore => FieldRoute::LegacyReliabilityObservation,
        LegacyPatientField::CreatedAt | LegacyPatientField::UpdatedAt => {
            FieldRoute::ProtectedProvenance
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiologicalSexV2 {
    Male,
    Female,
    Intersex,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BloodTypeV2 {
    APositive,
    ANegative,
    BPositive,
    BNegative,
    ABPositive,
    ABNegative,
    OPositive,
    ONegative,
    Unknown,
}

/// Protected plaintext identifier. Intentionally has no `Debug` implementation.
#[derive(Clone, PartialEq, Eq)]
pub struct ScopedIdentifier {
    pub namespace: String,
    pub value: String,
    pub issuer: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PersonNameV2 {
    pub first: String,
    pub last: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ContactInfoV2 {
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub phone_primary: Option<String>,
    pub phone_secondary: Option<String>,
    pub email: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EmergencyContactV2 {
    pub name: String,
    pub relationship: String,
    pub phone: String,
    pub email: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SourcedBloodType {
    pub value: BloodTypeV2,
    /// Opaque/private provenance reference. A blood type is not promoted merely
    /// because it was copied from the old profile.
    pub provenance_ref: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProfileProvenanceV2 {
    pub created_at_micros: i64,
    pub updated_at_micros: i64,
    pub source_reference: Vec<u8>,
}

/// Protected plaintext only. This type deliberately does not implement Debug.
///
/// Clinical arrays from v1 are absent: allergies/conditions/medications are
/// migrated to their canonical domains instead of becoming duplicate profile
/// state.
#[derive(Clone, PartialEq, Eq)]
pub struct ProtectedPatientProfileV2 {
    pub schema_version: u16,
    pub identifiers: Vec<ScopedIdentifier>,
    pub name: PersonNameV2,
    pub date_of_birth: String,
    pub biological_sex: BiologicalSexV2,
    pub gender_identity: Option<String>,
    pub blood_type: Option<SourcedBloodType>,
    pub contact: ContactInfoV2,
    pub emergency_contacts: Vec<EmergencyContactV2>,
    pub primary_language: String,
    pub provenance: ProfileProvenanceV2,
}

impl ProtectedPatientProfileV2 {
    pub fn new(
        identifiers: Vec<ScopedIdentifier>,
        name: PersonNameV2,
        date_of_birth: String,
        biological_sex: BiologicalSexV2,
        gender_identity: Option<String>,
        blood_type: Option<SourcedBloodType>,
        contact: ContactInfoV2,
        emergency_contacts: Vec<EmergencyContactV2>,
        primary_language: String,
        provenance: ProfileProvenanceV2,
    ) -> Result<Self, ProfileError> {
        if identifiers.is_empty() {
            return Err(ProfileError::MissingIdentifier);
        }
        if identifiers
            .iter()
            .any(|id| id.namespace.is_empty() || id.value.is_empty())
        {
            return Err(ProfileError::InvalidIdentifier);
        }
        if name.first.is_empty() || name.last.is_empty() {
            return Err(ProfileError::MissingName);
        }
        if !valid_yyyy_mm_dd(&date_of_birth) {
            return Err(ProfileError::InvalidBirthDate);
        }
        if primary_language.is_empty() {
            return Err(ProfileError::MissingLanguage);
        }
        if provenance.updated_at_micros < provenance.created_at_micros {
            return Err(ProfileError::InvalidProvenanceTime);
        }
        Ok(Self {
            schema_version: PATIENT_PROFILE_V2,
            identifiers,
            name,
            date_of_birth,
            biological_sex,
            gender_identity,
            blood_type,
            contact,
            emergency_contacts,
            primary_language,
            provenance,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileError {
    MissingIdentifier,
    InvalidIdentifier,
    MissingName,
    InvalidBirthDate,
    MissingLanguage,
    InvalidProvenanceTime,
}

fn valid_yyyy_mm_dd(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

/// Work that must leave the profile authority domain during migration.
/// This intentionally has no Debug implementation because it may contain PHI.
#[derive(Clone, PartialEq)]
pub struct DomainMigrationQueue {
    pub legacy_allergies: Vec<String>,
    pub legacy_conditions: Vec<String>,
    pub legacy_medications: Vec<String>,
    pub legacy_identity_binding: Option<IdentityBindingRef>,
    /// Legacy MATL value is preserved only as an attributed migration artifact,
    /// not promoted into `ProtectedPatientProfileV2`.
    pub legacy_matl_trust_score: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryMode {
    ExplicitCapabilityReferenceOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatientDiscoveryPolicyV2 {
    pub mode: DiscoveryMode,
    pub global_patient_directory: bool,
    pub clear_did_reverse_lookup: bool,
}

impl PatientDiscoveryPolicyV2 {
    pub const fn privacy_preserving_default() -> Self {
        Self {
            mode: DiscoveryMode::ExplicitCapabilityReferenceOnly,
            global_patient_directory: false,
            clear_did_reverse_lookup: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtectedDestination {
    pub envelope_id: EnvelopeId,
    pub context_handle: ContextHandle,
    pub policy_handle: PolicyHandle,
    pub key_epoch: u64,
}

impl ProtectedDestination {
    pub fn new(
        envelope_id: EnvelopeId,
        context_handle: ContextHandle,
        policy_handle: PolicyHandle,
        key_epoch: u64,
    ) -> Result<Self, DestinationError> {
        if key_epoch == 0 {
            return Err(DestinationError::ZeroKeyEpoch);
        }
        Ok(Self {
            envelope_id,
            context_handle,
            policy_handle,
            key_epoch,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationError {
    ZeroKeyEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationReceipt {
    pub legacy_source_digest: LegacySourceDigest,
    pub destination: ProtectedDestination,
    pub migrated_at_micros: i64,
    pub routed_field_count: usize,
    pub unresolved_domain_routes: usize,
    /// Always true for migration from the existing public v1 entry. The API has
    /// no constructor argument that can falsely claim historical recall.
    pub legacy_public_exposure_may_persist: bool,
}

impl MigrationReceipt {
    pub fn new(
        legacy_source_digest: LegacySourceDigest,
        destination: ProtectedDestination,
        migrated_at_micros: i64,
        unresolved_domain_routes: usize,
    ) -> Self {
        Self {
            legacy_source_digest,
            destination,
            migrated_at_micros,
            routed_field_count: LEGACY_PATIENT_V1_FIELD_COUNT,
            unresolved_domain_routes,
            legacy_public_exposure_may_persist: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MigrationState {
    LegacyV1Writable,
    Prepared {
        source: LegacySourceDigest,
    },
    V2Active {
        receipt: MigrationReceipt,
    },
    LegacyV1ReadOnly {
        receipt: MigrationReceipt,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoverError {
    AlreadyPreparedOrActive,
    NotPrepared,
    SourceMismatch,
    NotV2Active,
}

pub fn prepare_migration(
    state: &mut MigrationState,
    source: LegacySourceDigest,
) -> Result<(), CutoverError> {
    match state {
        MigrationState::LegacyV1Writable => {
            *state = MigrationState::Prepared { source };
            Ok(())
        }
        _ => Err(CutoverError::AlreadyPreparedOrActive),
    }
}

pub fn activate_v2(
    state: &mut MigrationState,
    receipt: MigrationReceipt,
) -> Result<(), CutoverError> {
    let expected_source = match state {
        MigrationState::Prepared { source } => *source,
        MigrationState::LegacyV1Writable => return Err(CutoverError::NotPrepared),
        MigrationState::V2Active { .. } | MigrationState::LegacyV1ReadOnly { .. } => {
            return Err(CutoverError::AlreadyPreparedOrActive)
        }
    };
    if expected_source != receipt.legacy_source_digest {
        return Err(CutoverError::SourceMismatch);
    }
    *state = MigrationState::V2Active { receipt };
    Ok(())
}

pub fn retire_legacy_writes(state: &mut MigrationState) -> Result<(), CutoverError> {
    let receipt = match state {
        MigrationState::V2Active { receipt } => receipt.clone(),
        _ => return Err(CutoverError::NotV2Active),
    };
    *state = MigrationState::LegacyV1ReadOnly { receipt };
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileWriteTarget {
    LegacyV1,
    ProtectedV2,
}

pub fn write_target(state: &MigrationState) -> ProfileWriteTarget {
    match state {
        MigrationState::LegacyV1Writable | MigrationState::Prepared { .. } => {
            ProfileWriteTarget::LegacyV1
        }
        MigrationState::V2Active { .. } | MigrationState::LegacyV1ReadOnly { .. } => {
            ProfileWriteTarget::ProtectedV2
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileReadTarget {
    LegacyV1,
    ProtectedV2Required,
}

pub fn read_target(state: &MigrationState) -> ProfileReadTarget {
    match state {
        MigrationState::LegacyV1Writable | MigrationState::Prepared { .. } => {
            ProfileReadTarget::LegacyV1
        }
        MigrationState::V2Active { .. } | MigrationState::LegacyV1ReadOnly { .. } => {
            ProfileReadTarget::ProtectedV2Required
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectedReadFailure {
    NotFound,
    AuthorizationDenied,
    AuthenticationFailed,
    DecryptionFailed,
    InvalidEnvelope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadResolution {
    LegacyAllowed,
    ProtectedRequired,
    DenyNoLegacyFallback(ProtectedReadFailure),
}

/// Makes the no-plaintext-fallback rule explicit after v2 activation.
pub fn resolve_read(
    state: &MigrationState,
    v2_failure: Option<ProtectedReadFailure>,
) -> ReadResolution {
    match read_target(state) {
        ProfileReadTarget::LegacyV1 => ReadResolution::LegacyAllowed,
        ProfileReadTarget::ProtectedV2Required => match v2_failure {
            None => ReadResolution::ProtectedRequired,
            Some(failure) => ReadResolution::DenyNoLegacyFallback(failure),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationResult {
    Registered,
    Idempotent,
    Conflict,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MigrationRegistry {
    receipt: Option<MigrationReceipt>,
}

impl MigrationRegistry {
    pub fn register(&mut self, receipt: MigrationReceipt) -> RegistrationResult {
        match &self.receipt {
            None => {
                self.receipt = Some(receipt);
                RegistrationResult::Registered
            }
            Some(existing) if existing == &receipt => RegistrationResult::Idempotent,
            Some(_) => RegistrationResult::Conflict,
        }
    }

    pub fn receipt(&self) -> Option<&MigrationReceipt> {
        self.receipt.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(byte: u8) -> LegacySourceDigest {
        LegacySourceDigest::new([byte; 32]).unwrap()
    }

    fn destination(byte: u8) -> ProtectedDestination {
        ProtectedDestination::new(
            EnvelopeId::new([byte; 32]).unwrap(),
            ContextHandle::new([byte + 1; 32]).unwrap(),
            PolicyHandle::new([byte + 2; 32]).unwrap(),
            1,
        )
        .unwrap()
    }

    fn receipt(src: u8, dst: u8) -> MigrationReceipt {
        MigrationReceipt::new(source(src), destination(dst), 500, 5)
    }

    #[test]
    fn all_legacy_fields_are_explicitly_routed() {
        assert_eq!(LegacyPatientField::ALL.len(), LEGACY_PATIENT_V1_FIELD_COUNT);
        for field in LegacyPatientField::ALL {
            let _ = route_legacy_field(field);
        }
    }

    #[test]
    fn clinical_vectors_leave_profile_authority_domain() {
        assert_eq!(
            route_legacy_field(LegacyPatientField::Allergies),
            FieldRoute::AllergyDomainMigration
        );
        assert_eq!(
            route_legacy_field(LegacyPatientField::Conditions),
            FieldRoute::ConditionDomainMigration
        );
        assert_eq!(
            route_legacy_field(LegacyPatientField::Medications),
            FieldRoute::MedicationDomainMigration
        );
    }

    #[test]
    fn identity_and_trust_are_not_demographic_profile_fields() {
        assert_eq!(
            route_legacy_field(LegacyPatientField::MycelixIdentityHash),
            FieldRoute::ProtectedIdentityBindingMigration
        );
        assert_eq!(
            route_legacy_field(LegacyPatientField::MatlTrustScore),
            FieldRoute::LegacyReliabilityObservation
        );
    }

    #[test]
    fn discovery_default_has_no_global_directory_or_clear_did_reverse_lookup() {
        let policy = PatientDiscoveryPolicyV2::privacy_preserving_default();
        assert_eq!(policy.mode, DiscoveryMode::ExplicitCapabilityReferenceOnly);
        assert!(!policy.global_patient_directory);
        assert!(!policy.clear_did_reverse_lookup);
    }

    #[test]
    fn destination_requires_nonzero_key_epoch() {
        assert_eq!(
            ProtectedDestination::new(
                EnvelopeId::new([1; 32]).unwrap(),
                ContextHandle::new([2; 32]).unwrap(),
                PolicyHandle::new([3; 32]).unwrap(),
                0,
            ),
            Err(DestinationError::ZeroKeyEpoch)
        );
    }

    #[test]
    fn migration_receipt_cannot_claim_legacy_public_recall() {
        let r = receipt(1, 10);
        assert!(r.legacy_public_exposure_may_persist);
        assert_eq!(r.routed_field_count, 18);
    }

    #[test]
    fn cutover_is_one_way_for_write_target() {
        let mut state = MigrationState::LegacyV1Writable;
        assert_eq!(write_target(&state), ProfileWriteTarget::LegacyV1);
        prepare_migration(&mut state, source(1)).unwrap();
        assert_eq!(write_target(&state), ProfileWriteTarget::LegacyV1);
        activate_v2(&mut state, receipt(1, 10)).unwrap();
        assert_eq!(write_target(&state), ProfileWriteTarget::ProtectedV2);
        retire_legacy_writes(&mut state).unwrap();
        assert_eq!(write_target(&state), ProfileWriteTarget::ProtectedV2);
    }

    #[test]
    fn activation_requires_matching_prepared_source() {
        let mut state = MigrationState::LegacyV1Writable;
        prepare_migration(&mut state, source(1)).unwrap();
        assert_eq!(
            activate_v2(&mut state, receipt(2, 10)),
            Err(CutoverError::SourceMismatch)
        );
    }

    #[test]
    fn v2_read_failure_never_falls_back_to_legacy_plaintext() {
        let mut state = MigrationState::LegacyV1Writable;
        prepare_migration(&mut state, source(1)).unwrap();
        activate_v2(&mut state, receipt(1, 10)).unwrap();
        assert_eq!(
            resolve_read(&state, Some(ProtectedReadFailure::DecryptionFailed)),
            ReadResolution::DenyNoLegacyFallback(ProtectedReadFailure::DecryptionFailed)
        );
        assert_ne!(resolve_read(&state, None), ReadResolution::LegacyAllowed);
    }

    #[test]
    fn same_exact_migration_registration_is_idempotent() {
        let mut registry = MigrationRegistry::default();
        let r = receipt(1, 10);
        assert_eq!(registry.register(r.clone()), RegistrationResult::Registered);
        assert_eq!(registry.register(r), RegistrationResult::Idempotent);
    }

    #[test]
    fn conflicting_second_migration_is_detected() {
        let mut registry = MigrationRegistry::default();
        assert_eq!(
            registry.register(receipt(1, 10)),
            RegistrationResult::Registered
        );
        assert_eq!(
            registry.register(receipt(1, 20)),
            RegistrationResult::Conflict
        );
    }

    #[test]
    fn profile_rejects_invalid_basic_identity_data() {
        let profile = ProtectedPatientProfileV2::new(
            vec![],
            PersonNameV2 {
                first: "Alice".into(),
                last: "Example".into(),
            },
            "1990-01-01".into(),
            BiologicalSexV2::Unknown,
            None,
            None,
            ContactInfoV2 {
                address_line1: None,
                address_line2: None,
                city: None,
                state_province: None,
                postal_code: None,
                country: "ZA".into(),
                phone_primary: None,
                phone_secondary: None,
                email: None,
            },
            vec![],
            "en".into(),
            ProfileProvenanceV2 {
                created_at_micros: 1,
                updated_at_micros: 2,
                source_reference: vec![1],
            },
        );
        assert_eq!(profile, Err(ProfileError::MissingIdentifier));
    }

    #[test]
    fn profile_has_no_allergy_condition_or_medication_fields() {
        // This is intentionally a construction test rather than reflection: the
        // only profile constructor inputs are identity/demographic/contact/provenance.
        let profile = ProtectedPatientProfileV2::new(
            vec![ScopedIdentifier {
                namespace: "patient-local".into(),
                value: "P-1".into(),
                issuer: None,
            }],
            PersonNameV2 {
                first: "Alice".into(),
                last: "Example".into(),
            },
            "1990-01-01".into(),
            BiologicalSexV2::Unknown,
            None,
            None,
            ContactInfoV2 {
                address_line1: None,
                address_line2: None,
                city: None,
                state_province: None,
                postal_code: None,
                country: "ZA".into(),
                phone_primary: None,
                phone_secondary: None,
                email: None,
            },
            vec![],
            "en".into(),
            ProfileProvenanceV2 {
                created_at_micros: 1,
                updated_at_micros: 2,
                source_reference: vec![1],
            },
        )
        .unwrap();
        assert_eq!(profile.schema_version, PATIENT_PROFILE_V2);
    }

    #[test]
    fn all_zero_opaque_ids_are_rejected_and_debug_is_redacted() {
        assert_eq!(EnvelopeId::new([0; 32]), Err(OpaqueIdError::AllZero));
        let id = EnvelopeId::new([8; 32]).unwrap();
        assert_eq!(format!("{:?}", id), "EnvelopeId([redacted])");
    }
}
