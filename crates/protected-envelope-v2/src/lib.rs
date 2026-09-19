#![forbid(unsafe_code)]
//! Metadata-minimized protected-envelope reference semantics.
//!
//! This crate deliberately does **not** implement encryption, key encapsulation,
//! consent evaluation, or Holochain storage. It freezes the representation
//! boundary proposed by PHI-ENC-002 (#159) so those mechanisms can be qualified
//! independently without allowing sensitive semantic metadata to leak into the
//! DHT-visible envelope by construction.
//!
//! Security boundary:
//! - [`PublicHeaderV2`] is the only metadata intended to be DHT-clear.
//! - [`ProtectedInnerV2`] is intended to be serialized *inside* authenticated
//!   encryption.
//! - [`AccessCapsuleV1`] is a separate key-distribution object and is **not**
//!   declared safe for public-DHT storage by this crate.
//! - This crate makes no cryptographic, clinical, legal, or regulatory claim.

use core::fmt;

pub const PROTECTED_ENVELOPE_V2: u16 = 2;
pub const ACCESS_CAPSULE_V1: u16 = 1;
pub const PUBLIC_AAD_DOMAIN: &[u8] = b"mycelix-health/protected-envelope-v2/aad\0";

/// A cryptographic-suite registry identifier.
///
/// Numeric on purpose: human-readable algorithm/configuration descriptions
/// belong in a separately reviewed registry, not in DHT-clear record metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SuiteId(pub u16);

/// Random/opaque envelope identity. It must not encode patient identity,
/// diagnosis, record type, or other semantic information.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvelopeId([u8; 32]);

/// Opaque, context-scoped lookup handle.
///
/// This is intentionally **not** a patient hash. A production derivation should
/// be scoped/rotatable so unrelated contexts cannot trivially correlate a
/// person's entire care history.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextHandle([u8; 32]);

/// Opaque policy locator/commitment.
///
/// It must not be a clear consent category such as `MentalHealth`,
/// `PsychotherapyNote`, or `SubstanceUseCounselingNote`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PolicyHandle([u8; 32]);

/// Opaque grant/capsule identity.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GrantId([u8; 32]);

/// Opaque provenance identity stored inside the encrypted inner object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProvenanceId(pub [u8; 32]);

/// Opaque consent/policy identity stored inside the encrypted inner object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConsentId(pub [u8; 32]);

macro_rules! opaque32 {
    ($ty:ident) => {
        impl $ty {
            pub fn new(bytes: [u8; 32]) -> Result<Self, HandleError> {
                if bytes == [0u8; 32] {
                    return Err(HandleError::AllZero);
                }
                Ok(Self(bytes))
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                // Avoid accidentally dumping stable opaque identifiers into logs.
                f.write_str(concat!(stringify!($ty), "([redacted])"))
            }
        }
    };
}

opaque32!(EnvelopeId);
opaque32!(ContextHandle);
opaque32!(PolicyHandle);
opaque32!(GrantId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleError {
    AllZero,
}

/// The *entire* DHT-clear metadata contract for a v2 protected envelope.
///
/// Deliberately absent:
/// - patient / agent identity;
/// - record kind or diagnosis;
/// - care domain or sensitivity label;
/// - clinician / recipient identity;
/// - consent purpose;
/// - human-readable timestamp;
/// - stable key fingerprint.
///
/// All fields that influence security/routing are included in canonical AAD by
/// [`PublicHeaderV2::aad_bytes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicHeaderV2 {
    pub envelope_version: u16,
    pub suite_id: SuiteId,
    pub envelope_id: EnvelopeId,
    pub context_handle: ContextHandle,
    pub policy_handle: PolicyHandle,
    pub key_epoch: u64,
    pub nonce: [u8; 24],
}

impl PublicHeaderV2 {
    pub fn new(
        suite_id: SuiteId,
        envelope_id: EnvelopeId,
        context_handle: ContextHandle,
        policy_handle: PolicyHandle,
        key_epoch: u64,
        nonce: [u8; 24],
    ) -> Result<Self, PublicHeaderError> {
        if suite_id.0 == 0 {
            return Err(PublicHeaderError::UnspecifiedSuite);
        }
        if key_epoch == 0 {
            return Err(PublicHeaderError::ZeroKeyEpoch);
        }
        if nonce == [0u8; 24] {
            return Err(PublicHeaderError::AllZeroNonce);
        }
        Ok(Self {
            envelope_version: PROTECTED_ENVELOPE_V2,
            suite_id,
            envelope_id,
            context_handle,
            policy_handle,
            key_epoch,
            nonce,
        })
    }

    /// Canonical, serializer-independent AAD encoding for all clear fields.
    ///
    /// The nonce is included even though a correct AEAD already consumes it as
    /// a separate parameter; binding it here makes the complete public routing
    /// header explicit in the transcript/evidence contract.
    pub fn aad_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(PUBLIC_AAD_DOMAIN.len() + 2 + 2 + 32 + 32 + 32 + 8 + 24);
        out.extend_from_slice(PUBLIC_AAD_DOMAIN);
        out.extend_from_slice(&self.envelope_version.to_be_bytes());
        out.extend_from_slice(&self.suite_id.0.to_be_bytes());
        out.extend_from_slice(self.envelope_id.as_bytes());
        out.extend_from_slice(self.context_handle.as_bytes());
        out.extend_from_slice(self.policy_handle.as_bytes());
        out.extend_from_slice(&self.key_epoch.to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicHeaderError {
    UnspecifiedSuite,
    ZeroKeyEpoch,
    AllZeroNonce,
}

/// DHT-storable encrypted envelope.
///
/// `ciphertext` is opaque to this crate. Production code must use an independently
/// qualified AEAD suite and authenticate [`PublicHeaderV2::aad_bytes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedEnvelopeV2 {
    pub public: PublicHeaderV2,
    pub ciphertext: Vec<u8>,
}

impl ProtectedEnvelopeV2 {
    pub fn new(public: PublicHeaderV2, ciphertext: Vec<u8>) -> Result<Self, EnvelopeError> {
        if ciphertext.is_empty() {
            return Err(EnvelopeError::EmptyCiphertext);
        }
        Ok(Self { public, ciphertext })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    EmptyCiphertext,
}

/// Fine semantic record kind. This type belongs *inside* protected plaintext.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtectedRecordKind {
    GeneralClinical,
    MentalHealthScreening,
    PsychotherapyNote,
    CrisisPlan,
    CrisisEvent,
    SubstanceUseTreatment,
    SubstanceUseCounselingNote,
    PeerSupport,
    PersonalReflection,
    SpiritualCarePreference,
    SpiritualCareEncounter,
    PastoralCareNote,
    Other(String),
}

/// Sensitivity/handling class. Also encrypted; publishing this enum as DHT-clear
/// metadata would defeat the purpose of this v2 boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SensitivityClass {
    ProtectedHealth,
    Psychotherapy,
    SubstanceUse,
    Crisis,
    PeerSupport,
    PersonalReflection,
    SpiritualOrPastoral,
    Other(String),
}

/// Purpose carried with the protected object for provenance/audit. Authorization
/// remains a separate proof line; placing a purpose here does not grant access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtectedPurpose {
    Care,
    CareCoordination,
    CrisisSupport,
    Research,
    ModelEvaluation,
    ModelTraining,
    SpiritualCare,
    PeerSupport,
    Other(String),
}

/// Metadata and payload that must be serialized within authenticated encryption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedInnerV2 {
    pub schema_version: u16,
    pub subject_reference: Vec<u8>,
    pub record_kind: ProtectedRecordKind,
    pub sensitivity: SensitivityClass,
    pub purpose: ProtectedPurpose,
    pub consent: Option<ConsentId>,
    pub provenance: ProvenanceId,
    pub effective_time_micros: Option<i64>,
    pub payload: Vec<u8>,
}

impl ProtectedInnerV2 {
    pub fn validate(&self) -> Result<(), InnerError> {
        if self.schema_version == 0 {
            return Err(InnerError::ZeroSchemaVersion);
        }
        if self.subject_reference.is_empty() {
            return Err(InnerError::EmptySubjectReference);
        }
        if self.payload.is_empty() {
            return Err(InnerError::EmptyPayload);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InnerError {
    ZeroSchemaVersion,
    EmptySubjectReference,
    EmptyPayload,
}

/// A separately qualified key/capability object.
///
/// This type is deliberately not nested inside [`ProtectedEnvelopeV2`]. A
/// production implementation must decide how capsules are privately delivered
/// or locator-hidden; this crate does not declare them safe for public DHT use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessCapsuleV1 {
    pub capsule_version: u16,
    pub envelope_id: EnvelopeId,
    pub policy_handle: PolicyHandle,
    pub grant_id: GrantId,
    pub key_epoch: u64,
    pub wrapped_content_key: Vec<u8>,
    pub expires_at_micros: Option<i64>,
}

impl AccessCapsuleV1 {
    pub fn new(
        envelope_id: EnvelopeId,
        policy_handle: PolicyHandle,
        grant_id: GrantId,
        key_epoch: u64,
        wrapped_content_key: Vec<u8>,
        expires_at_micros: Option<i64>,
    ) -> Result<Self, AccessCapsuleError> {
        if key_epoch == 0 {
            return Err(AccessCapsuleError::ZeroKeyEpoch);
        }
        if wrapped_content_key.is_empty() {
            return Err(AccessCapsuleError::EmptyWrappedKey);
        }
        Ok(Self {
            capsule_version: ACCESS_CAPSULE_V1,
            envelope_id,
            policy_handle,
            grant_id,
            key_epoch,
            wrapped_content_key,
            expires_at_micros,
        })
    }

    pub fn matches(&self, envelope: &ProtectedEnvelopeV2) -> bool {
        self.envelope_id == envelope.public.envelope_id
            && self.policy_handle == envelope.public.policy_handle
            && self.key_epoch == envelope.public.key_epoch
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessCapsuleError {
    ZeroKeyEpoch,
    EmptyWrappedKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn header() -> PublicHeaderV2 {
        PublicHeaderV2::new(
            SuiteId(1),
            EnvelopeId::new(id(1)).unwrap(),
            ContextHandle::new(id(2)).unwrap(),
            PolicyHandle::new(id(3)).unwrap(),
            7,
            [4u8; 24],
        )
        .unwrap()
    }

    #[test]
    fn public_aad_is_fixed_width_domain_separated_and_deterministic() {
        let h = header();
        let a = h.aad_bytes();
        let b = h.aad_bytes();
        assert_eq!(a, b);
        assert!(a.starts_with(PUBLIC_AAD_DOMAIN));
        assert_eq!(
            a.len(),
            PUBLIC_AAD_DOMAIN.len() + 2 + 2 + 32 + 32 + 32 + 8 + 24
        );
    }

    #[test]
    fn changing_any_public_security_field_changes_aad() {
        let baseline = header();
        let original = baseline.aad_bytes();

        let mut suite = baseline.clone();
        suite.suite_id = SuiteId(2);
        assert_ne!(original, suite.aad_bytes());

        let mut env = baseline.clone();
        env.envelope_id = EnvelopeId::new(id(9)).unwrap();
        assert_ne!(original, env.aad_bytes());

        let mut context = baseline.clone();
        context.context_handle = ContextHandle::new(id(8)).unwrap();
        assert_ne!(original, context.aad_bytes());

        let mut policy = baseline.clone();
        policy.policy_handle = PolicyHandle::new(id(7)).unwrap();
        assert_ne!(original, policy.aad_bytes());

        let mut epoch = baseline.clone();
        epoch.key_epoch += 1;
        assert_ne!(original, epoch.aad_bytes());

        let mut nonce = baseline;
        nonce.nonce[0] ^= 1;
        assert_ne!(original, nonce.aad_bytes());
    }

    #[test]
    fn fine_semantics_do_not_participate_in_public_aad() {
        let h = header();
        let before = h.aad_bytes();

        let psychotherapy = ProtectedInnerV2 {
            schema_version: 1,
            subject_reference: b"subject-a".to_vec(),
            record_kind: ProtectedRecordKind::PsychotherapyNote,
            sensitivity: SensitivityClass::Psychotherapy,
            purpose: ProtectedPurpose::Care,
            consent: Some(ConsentId(id(5))),
            provenance: ProvenanceId(id(6)),
            effective_time_micros: Some(1_234_567),
            payload: b"secret note".to_vec(),
        };
        let spiritual = ProtectedInnerV2 {
            record_kind: ProtectedRecordKind::PastoralCareNote,
            sensitivity: SensitivityClass::SpiritualOrPastoral,
            purpose: ProtectedPurpose::SpiritualCare,
            ..psychotherapy.clone()
        };

        psychotherapy.validate().unwrap();
        spiritual.validate().unwrap();
        assert_eq!(before, h.aad_bytes());
    }

    #[test]
    fn opaque_identifiers_refuse_zero_sentinel() {
        assert_eq!(EnvelopeId::new([0; 32]), Err(HandleError::AllZero));
        assert_eq!(ContextHandle::new([0; 32]), Err(HandleError::AllZero));
        assert_eq!(PolicyHandle::new([0; 32]), Err(HandleError::AllZero));
        assert_eq!(GrantId::new([0; 32]), Err(HandleError::AllZero));
    }

    #[test]
    fn opaque_identifiers_redact_debug_output() {
        let env = EnvelopeId::new(id(1)).unwrap();
        let rendered = format!("{env:?}");
        assert_eq!(rendered, "EnvelopeId([redacted])");
        assert!(!rendered.contains("0101"));
    }

    #[test]
    fn envelope_refuses_empty_ciphertext() {
        assert_eq!(
            ProtectedEnvelopeV2::new(header(), vec![]),
            Err(EnvelopeError::EmptyCiphertext)
        );
    }

    #[test]
    fn capsule_is_bound_to_envelope_policy_and_epoch() {
        let envelope = ProtectedEnvelopeV2::new(header(), vec![9]).unwrap();
        let capsule = AccessCapsuleV1::new(
            envelope.public.envelope_id,
            envelope.public.policy_handle,
            GrantId::new(id(6)).unwrap(),
            envelope.public.key_epoch,
            vec![7; 32],
            Some(9_999_999),
        )
        .unwrap();
        assert!(capsule.matches(&envelope));

        let mut wrong_epoch = capsule.clone();
        wrong_epoch.key_epoch += 1;
        assert!(!wrong_epoch.matches(&envelope));

        let mut wrong_policy = capsule;
        wrong_policy.policy_handle = PolicyHandle::new(id(8)).unwrap();
        assert!(!wrong_policy.matches(&envelope));
    }

    #[test]
    fn no_recipient_identity_exists_in_public_header_contract() {
        // Structural canary: constructing the entire public header requires only
        // opaque envelope/context/policy handles, suite, epoch, and nonce. There
        // is no patient, clinician, recipient, record-kind, category, purpose,
        // or human-readable timestamp argument to smuggle into the public API.
        let _ = header();
    }

    #[test]
    fn header_rejects_unspecified_suite_epoch_and_zero_nonce() {
        let envelope_id = EnvelopeId::new(id(1)).unwrap();
        let context = ContextHandle::new(id(2)).unwrap();
        let policy = PolicyHandle::new(id(3)).unwrap();

        assert_eq!(
            PublicHeaderV2::new(SuiteId(0), envelope_id, context, policy, 1, [4; 24]),
            Err(PublicHeaderError::UnspecifiedSuite)
        );
        assert_eq!(
            PublicHeaderV2::new(SuiteId(1), envelope_id, context, policy, 0, [4; 24]),
            Err(PublicHeaderError::ZeroKeyEpoch)
        );
        assert_eq!(
            PublicHeaderV2::new(SuiteId(1), envelope_id, context, policy, 1, [0; 24]),
            Err(PublicHeaderError::AllZeroNonce)
        );
    }
}
