#![forbid(unsafe_code)]
//! Reference semantics for private, attenuable care capabilities.
//!
//! This crate freezes the non-cryptographic authorization theorem from
//! CARE-CAP-001 (#163). It deliberately does **not** implement signatures,
//! encryption, key wrapping, proxy re-encryption, Holochain storage, identity
//! verification, or legal consent. Those mechanisms must bind to these semantics
//! through separately qualified adapters.
//!
//! Security boundary:
//! - [`CareCapabilityV1`] is detailed private authorization material and is not
//!   declared safe for public DHT storage.
//! - [`CapabilityCommitmentV1`] is the metadata-minimized public shape.
//! - [`AccessCapsuleBindingV1`] proves only structural authorization to issue a
//!   future cryptographic key capsule; it contains no key material itself.
//! - delegation is monotonic/subtractive;
//! - revocation prevents future capsule authorization in this state machine but
//!   cannot recall plaintext or keys already disclosed.

use core::fmt;
use std::collections::BTreeSet;

pub const CARE_CAPABILITY_V1: u16 = 1;
pub const CAPABILITY_COMMITMENT_V1: u16 = 1;
pub const ACCESS_CAPSULE_BINDING_V1: u16 = 1;
pub const MAX_EMERGENCY_TTL_MILLIS: u64 = 60 * 60 * 1_000;

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
                if bytes == [0u8; 32] {
                    return Err(OpaqueIdError::AllZero);
                }
                Ok(Self(bytes))
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
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

opaque32!(CapabilityId);
opaque32!(PrincipalHandle);
opaque32!(SubjectContextHandle);
opaque32!(PolicyHandle);
opaque32!(ClaimRef);
opaque32!(IssuerProofRef);
opaque32!(RevocationHandle);
opaque32!(CommitmentDigest);
opaque32!(EnvelopeId);
opaque32!(EmergencyReasonCommitment);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CarePurpose {
    DirectCare,
    CareCoordination,
    MentalHealthCare,
    MedicationManagement,
    SpiritualCare,
    PeerSupport,
    CrisisPlanning,
    CrisisResponse,
    Research,
    ModelEvaluation,
    ModelTraining,
    PopulationAnalytics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CareAction {
    View,
    UseForCare,
    Annotate,
    AmendOrCorrect,
    Share,
    Export,
    UseForResearch,
    UseForModelEvaluation,
    UseForModelTraining,
    UseForPopulationAnalytics,
    ContactForCare,
    ContactTrustedPerson,
}

/// Conservative semantic compatibility. A cryptographic/identity adapter may be
/// more restrictive; it must never be more permissive.
pub fn purpose_allows_action(purpose: CarePurpose, action: CareAction) -> bool {
    use CareAction::*;
    use CarePurpose::*;

    match purpose {
        DirectCare => matches!(
            action,
            View | UseForCare | Annotate | AmendOrCorrect | ContactForCare
        ),
        CareCoordination => matches!(
            action,
            View | UseForCare | Share | ContactForCare | ContactTrustedPerson
        ),
        MentalHealthCare => matches!(
            action,
            View | UseForCare | Annotate | AmendOrCorrect | ContactForCare
        ),
        MedicationManagement => matches!(
            action,
            View | UseForCare | Annotate | AmendOrCorrect | ContactForCare
        ),
        SpiritualCare => matches!(action, View | UseForCare | Annotate | ContactForCare),
        PeerSupport => matches!(action, View | UseForCare | ContactForCare),
        CrisisPlanning => matches!(
            action,
            View | UseForCare | Annotate | Share | ContactForCare | ContactTrustedPerson
        ),
        CrisisResponse => matches!(
            action,
            View | UseForCare | Share | ContactForCare | ContactTrustedPerson
        ),
        Research => matches!(action, View | UseForResearch | Export),
        ModelEvaluation => matches!(action, View | UseForModelEvaluation),
        ModelTraining => matches!(action, View | UseForModelTraining),
        PopulationAnalytics => matches!(action, View | UseForPopulationAnalytics),
    }
}

/// Explicit selector. There is intentionally no `All`, wildcard, category string,
/// or implicit patient-wide selector in this v1 reference type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimSelector {
    claims: Vec<ClaimRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorError {
    Empty,
    DuplicateClaim,
}

impl ClaimSelector {
    pub fn new(claims: Vec<ClaimRef>) -> Result<Self, SelectorError> {
        if claims.is_empty() {
            return Err(SelectorError::Empty);
        }
        let unique: BTreeSet<_> = claims.iter().copied().collect();
        if unique.len() != claims.len() {
            return Err(SelectorError::DuplicateClaim);
        }
        Ok(Self { claims })
    }

    pub fn claims(&self) -> &[ClaimRef] {
        &self.claims
    }

    pub fn contains(&self, claim: ClaimRef) -> bool {
        self.claims.contains(&claim)
    }

    pub fn is_subset_of(&self, parent: &Self) -> bool {
        self.claims.iter().all(|claim| parent.contains(*claim))
    }
}

/// Detailed private authorization object.
///
/// `issuer_proof` means "the adapter says this capability is bound to an issuer
/// proof with this opaque identity". This crate does not verify signatures and
/// therefore makes no forged-signature resistance claim by itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareCapabilityV1 {
    pub version: u16,
    pub capability_id: CapabilityId,
    pub issuer: PrincipalHandle,
    pub grantee: PrincipalHandle,
    pub subject_context: SubjectContextHandle,
    pub policy_handle: PolicyHandle,
    pub issuer_proof: IssuerProofRef,
    pub purpose: CarePurpose,
    pub actions: Vec<CareAction>,
    pub selector: ClaimSelector,
    pub issued_at_millis: u64,
    pub not_before_millis: u64,
    pub expires_at_millis: u64,
    /// Exact content-key epoch this grant can authorize. Rotation therefore
    /// requires a fresh/updated authorization proof rather than silently carrying
    /// an old grant forward.
    pub key_epoch: u64,
    /// Number of further delegation hops permitted.
    pub delegation_remaining: u8,
    /// Once true, attenuation cannot turn this restriction off.
    pub no_further_disclosure: bool,
    pub revocation_handle: RevocationHandle,
    pub parent: Option<CapabilityId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityError {
    EmptyActions,
    DuplicateAction,
    ActionIncompatibleWithPurpose,
    NotBeforePrecedesIssue,
    NonPositiveValidityWindow,
    ZeroKeyEpoch,
    IssuerEqualsGrantee,
}

impl CareCapabilityV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        capability_id: CapabilityId,
        issuer: PrincipalHandle,
        grantee: PrincipalHandle,
        subject_context: SubjectContextHandle,
        policy_handle: PolicyHandle,
        issuer_proof: IssuerProofRef,
        purpose: CarePurpose,
        actions: Vec<CareAction>,
        selector: ClaimSelector,
        issued_at_millis: u64,
        not_before_millis: u64,
        expires_at_millis: u64,
        key_epoch: u64,
        delegation_remaining: u8,
        no_further_disclosure: bool,
        revocation_handle: RevocationHandle,
    ) -> Result<Self, CapabilityError> {
        validate_capability_parts(
            issuer,
            grantee,
            purpose,
            &actions,
            issued_at_millis,
            not_before_millis,
            expires_at_millis,
            key_epoch,
        )?;

        Ok(Self {
            version: CARE_CAPABILITY_V1,
            capability_id,
            issuer,
            grantee,
            subject_context,
            policy_handle,
            issuer_proof,
            purpose,
            actions,
            selector,
            issued_at_millis,
            not_before_millis,
            expires_at_millis,
            key_epoch,
            delegation_remaining,
            no_further_disclosure,
            revocation_handle,
            parent: None,
        })
    }

    pub fn allows_action(&self, action: CareAction) -> bool {
        self.actions.contains(&action)
    }

    pub fn active_at(&self, now_millis: u64) -> bool {
        now_millis >= self.not_before_millis && now_millis < self.expires_at_millis
    }

    /// Delegate only by attenuation.
    ///
    /// V1 intentionally requires the child purpose, policy handle, subject context
    /// and key epoch to remain identical. Changing purpose or key epoch requires a
    /// fresh authority proof rather than being hidden inside delegation.
    #[allow(clippy::too_many_arguments)]
    pub fn attenuate_to(
        &self,
        child_id: CapabilityId,
        child_grantee: PrincipalHandle,
        child_issuer_proof: IssuerProofRef,
        child_actions: Vec<CareAction>,
        child_selector: ClaimSelector,
        child_issued_at_millis: u64,
        child_not_before_millis: u64,
        child_expires_at_millis: u64,
        child_delegation_remaining: u8,
        child_no_further_disclosure: bool,
        child_revocation_handle: RevocationHandle,
    ) -> Result<Self, AttenuationError> {
        if self.delegation_remaining == 0 {
            return Err(AttenuationError::DelegationExhausted);
        }
        if child_grantee == self.grantee {
            return Err(AttenuationError::ChildGranteeMustDifferFromDelegatingIssuer);
        }
        if child_issued_at_millis < self.not_before_millis
            || child_issued_at_millis >= self.expires_at_millis
        {
            return Err(AttenuationError::IssuedOutsideParentWindow);
        }
        if child_not_before_millis < self.not_before_millis {
            return Err(AttenuationError::StartsBeforeParent);
        }
        if child_expires_at_millis > self.expires_at_millis {
            return Err(AttenuationError::OutlivesParent);
        }
        if child_delegation_remaining >= self.delegation_remaining {
            return Err(AttenuationError::DelegationDepthNotReduced);
        }
        if child_actions
            .iter()
            .any(|action| !self.actions.contains(action))
        {
            return Err(AttenuationError::AddsAction);
        }
        if !child_selector.is_subset_of(&self.selector) {
            return Err(AttenuationError::AddsClaim);
        }
        if self.no_further_disclosure && !child_no_further_disclosure {
            return Err(AttenuationError::WeakensRedisclosureRestriction);
        }

        validate_capability_parts(
            self.grantee,
            child_grantee,
            self.purpose,
            &child_actions,
            child_issued_at_millis,
            child_not_before_millis,
            child_expires_at_millis,
            self.key_epoch,
        )
        .map_err(AttenuationError::InvalidChild)?;

        Ok(Self {
            version: CARE_CAPABILITY_V1,
            capability_id: child_id,
            issuer: self.grantee,
            grantee: child_grantee,
            subject_context: self.subject_context,
            policy_handle: self.policy_handle,
            issuer_proof: child_issuer_proof,
            purpose: self.purpose,
            actions: child_actions,
            selector: child_selector,
            issued_at_millis: child_issued_at_millis,
            not_before_millis: child_not_before_millis,
            expires_at_millis: child_expires_at_millis,
            key_epoch: self.key_epoch,
            delegation_remaining: child_delegation_remaining,
            no_further_disclosure: child_no_further_disclosure,
            revocation_handle: child_revocation_handle,
            parent: Some(self.capability_id),
        })
    }
}

fn validate_capability_parts(
    issuer: PrincipalHandle,
    grantee: PrincipalHandle,
    purpose: CarePurpose,
    actions: &[CareAction],
    issued_at_millis: u64,
    not_before_millis: u64,
    expires_at_millis: u64,
    key_epoch: u64,
) -> Result<(), CapabilityError> {
    if issuer == grantee {
        return Err(CapabilityError::IssuerEqualsGrantee);
    }
    if actions.is_empty() {
        return Err(CapabilityError::EmptyActions);
    }
    let unique: BTreeSet<_> = actions.iter().copied().collect();
    if unique.len() != actions.len() {
        return Err(CapabilityError::DuplicateAction);
    }
    if actions
        .iter()
        .copied()
        .any(|action| !purpose_allows_action(purpose, action))
    {
        return Err(CapabilityError::ActionIncompatibleWithPurpose);
    }
    if not_before_millis < issued_at_millis {
        return Err(CapabilityError::NotBeforePrecedesIssue);
    }
    if expires_at_millis <= not_before_millis {
        return Err(CapabilityError::NonPositiveValidityWindow);
    }
    if key_epoch == 0 {
        return Err(CapabilityError::ZeroKeyEpoch);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttenuationError {
    DelegationExhausted,
    ChildGranteeMustDifferFromDelegatingIssuer,
    IssuedOutsideParentWindow,
    StartsBeforeParent,
    OutlivesParent,
    DelegationDepthNotReduced,
    AddsAction,
    AddsClaim,
    WeakensRedisclosureRestriction,
    InvalidChild(CapabilityError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityPublicStatus {
    Active,
    Revoked,
}

/// Minimal public revocation/validation commitment shape.
///
/// The digest/revocation handle are produced by a future cryptographic adapter.
/// Fine care purpose, grantee, claim selector and semantic record labels are
/// intentionally absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityCommitmentV1 {
    pub version: u16,
    pub digest: CommitmentDigest,
    pub revocation_handle: RevocationHandle,
    pub status: CapabilityPublicStatus,
    pub state_epoch: u64,
}

impl CapabilityCommitmentV1 {
    pub fn new(
        digest: CommitmentDigest,
        revocation_handle: RevocationHandle,
        status: CapabilityPublicStatus,
        state_epoch: u64,
    ) -> Result<Self, CommitmentError> {
        if state_epoch == 0 {
            return Err(CommitmentError::ZeroStateEpoch);
        }
        Ok(Self {
            version: CAPABILITY_COMMITMENT_V1,
            digest,
            revocation_handle,
            status,
            state_epoch,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitmentError {
    ZeroStateEpoch,
}

/// Local/authorized capability state. Detailed capability material stays private;
/// revocation state gates future use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityState {
    pub capability: CareCapabilityV1,
    pub revoked_at_millis: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevokeError {
    AlreadyRevoked,
    RevocationBeforeIssue,
}

impl CapabilityState {
    pub fn new(capability: CareCapabilityV1) -> Self {
        Self {
            capability,
            revoked_at_millis: None,
        }
    }

    pub fn revoke(&mut self, at_millis: u64) -> Result<(), RevokeError> {
        if self.revoked_at_millis.is_some() {
            return Err(RevokeError::AlreadyRevoked);
        }
        if at_millis < self.capability.issued_at_millis {
            return Err(RevokeError::RevocationBeforeIssue);
        }
        self.revoked_at_millis = Some(at_millis);
        Ok(())
    }

    pub fn is_revoked_at(&self, now_millis: u64) -> bool {
        self.revoked_at_millis
            .is_some_and(|revoked_at| now_millis >= revoked_at)
    }

    /// Authorize the metadata binding for a future cryptographic access capsule.
    ///
    /// This method does not wrap or reveal a key. It proves only that, at this
    /// instant, the private capability semantics allow a View-based capsule bound
    /// to the exact grantee, policy and key epoch.
    pub fn authorize_access_capsule(
        &self,
        envelope_id: EnvelopeId,
        requester: PrincipalHandle,
        policy_handle: PolicyHandle,
        key_epoch: u64,
        now_millis: u64,
    ) -> Result<AccessCapsuleBindingV1, CapsuleAuthorizationError> {
        if self.is_revoked_at(now_millis) {
            return Err(CapsuleAuthorizationError::Revoked);
        }
        if !self.capability.active_at(now_millis) {
            return Err(CapsuleAuthorizationError::Inactive);
        }
        if requester != self.capability.grantee {
            return Err(CapsuleAuthorizationError::GranteeMismatch);
        }
        if policy_handle != self.capability.policy_handle {
            return Err(CapsuleAuthorizationError::PolicyMismatch);
        }
        if key_epoch != self.capability.key_epoch {
            return Err(CapsuleAuthorizationError::KeyEpochMismatch);
        }
        if !self.capability.allows_action(CareAction::View) {
            return Err(CapsuleAuthorizationError::ViewNotAuthorized);
        }

        Ok(AccessCapsuleBindingV1 {
            version: ACCESS_CAPSULE_BINDING_V1,
            envelope_id,
            capability_id: self.capability.capability_id,
            grantee: requester,
            policy_handle,
            key_epoch,
            expires_at_millis: self.capability.expires_at_millis,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapsuleAuthorizationError {
    Revoked,
    Inactive,
    GranteeMismatch,
    PolicyMismatch,
    KeyEpochMismatch,
    ViewNotAuthorized,
}

/// Structural binding a crypto adapter must use when wrapping a content key.
/// Contains no key bytes and makes no cryptographic security claim itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessCapsuleBindingV1 {
    pub version: u16,
    pub envelope_id: EnvelopeId,
    pub capability_id: CapabilityId,
    pub grantee: PrincipalHandle,
    pub policy_handle: PolicyHandle,
    pub key_epoch: u64,
    pub expires_at_millis: u64,
}

/// Separate break-glass capability family. It is deliberately read-only,
/// non-delegable, finite, and cannot express research/model-training rights.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmergencyCapabilityV1 {
    pub capability_id: CapabilityId,
    pub issuer: PrincipalHandle,
    pub grantee: PrincipalHandle,
    pub subject_context: SubjectContextHandle,
    pub policy_handle: PolicyHandle,
    pub issuer_proof: IssuerProofRef,
    pub selector: ClaimSelector,
    pub reason_commitment: EmergencyReasonCommitment,
    pub issued_at_millis: u64,
    pub not_before_millis: u64,
    pub expires_at_millis: u64,
    pub key_epoch: u64,
    pub revocation_handle: RevocationHandle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmergencyCapabilityError {
    IssuerEqualsGrantee,
    NotBeforePrecedesIssue,
    NonPositiveValidityWindow,
    ExceedsMaximumTtl,
    ZeroKeyEpoch,
}

impl EmergencyCapabilityV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        capability_id: CapabilityId,
        issuer: PrincipalHandle,
        grantee: PrincipalHandle,
        subject_context: SubjectContextHandle,
        policy_handle: PolicyHandle,
        issuer_proof: IssuerProofRef,
        selector: ClaimSelector,
        reason_commitment: EmergencyReasonCommitment,
        issued_at_millis: u64,
        not_before_millis: u64,
        expires_at_millis: u64,
        key_epoch: u64,
        revocation_handle: RevocationHandle,
    ) -> Result<Self, EmergencyCapabilityError> {
        if issuer == grantee {
            return Err(EmergencyCapabilityError::IssuerEqualsGrantee);
        }
        if not_before_millis < issued_at_millis {
            return Err(EmergencyCapabilityError::NotBeforePrecedesIssue);
        }
        if expires_at_millis <= not_before_millis {
            return Err(EmergencyCapabilityError::NonPositiveValidityWindow);
        }
        if expires_at_millis - not_before_millis > MAX_EMERGENCY_TTL_MILLIS {
            return Err(EmergencyCapabilityError::ExceedsMaximumTtl);
        }
        if key_epoch == 0 {
            return Err(EmergencyCapabilityError::ZeroKeyEpoch);
        }
        Ok(Self {
            capability_id,
            issuer,
            grantee,
            subject_context,
            policy_handle,
            issuer_proof,
            selector,
            reason_commitment,
            issued_at_millis,
            not_before_millis,
            expires_at_millis,
            key_epoch,
            revocation_handle,
        })
    }

    pub const fn allowed_action(&self) -> CareAction {
        CareAction::View
    }

    pub const fn delegation_allowed(&self) -> bool {
        false
    }

    pub const fn model_training_allowed(&self) -> bool {
        false
    }

    pub const fn research_allowed(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn p(byte: u8) -> PrincipalHandle {
        PrincipalHandle::new(id(byte)).unwrap()
    }

    fn c(byte: u8) -> ClaimRef {
        ClaimRef::new(id(byte)).unwrap()
    }

    fn base_capability() -> CareCapabilityV1 {
        CareCapabilityV1::new(
            CapabilityId::new(id(1)).unwrap(),
            p(2),
            p(3),
            SubjectContextHandle::new(id(4)).unwrap(),
            PolicyHandle::new(id(5)).unwrap(),
            IssuerProofRef::new(id(6)).unwrap(),
            CarePurpose::MentalHealthCare,
            vec![CareAction::View, CareAction::UseForCare, CareAction::Annotate],
            ClaimSelector::new(vec![c(10), c(11), c(12)]).unwrap(),
            100,
            100,
            1_000,
            7,
            2,
            false,
            RevocationHandle::new(id(7)).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn selector_has_no_implicit_empty_or_duplicate_scope() {
        assert_eq!(ClaimSelector::new(vec![]), Err(SelectorError::Empty));
        assert_eq!(
            ClaimSelector::new(vec![c(1), c(1)]),
            Err(SelectorError::DuplicateClaim)
        );
    }

    #[test]
    fn care_purpose_cannot_authorize_model_training() {
        let result = CareCapabilityV1::new(
            CapabilityId::new(id(1)).unwrap(),
            p(2),
            p(3),
            SubjectContextHandle::new(id(4)).unwrap(),
            PolicyHandle::new(id(5)).unwrap(),
            IssuerProofRef::new(id(6)).unwrap(),
            CarePurpose::DirectCare,
            vec![CareAction::View, CareAction::UseForModelTraining],
            ClaimSelector::new(vec![c(10)]).unwrap(),
            100,
            100,
            1_000,
            1,
            0,
            false,
            RevocationHandle::new(id(7)).unwrap(),
        );
        assert_eq!(result, Err(CapabilityError::ActionIncompatibleWithPurpose));
    }

    #[test]
    fn delegation_cannot_add_action() {
        let parent = base_capability();
        let result = parent.attenuate_to(
            CapabilityId::new(id(20)).unwrap(),
            p(30),
            IssuerProofRef::new(id(31)).unwrap(),
            vec![CareAction::View, CareAction::Share],
            ClaimSelector::new(vec![c(10)]).unwrap(),
            200,
            200,
            900,
            1,
            false,
            RevocationHandle::new(id(32)).unwrap(),
        );
        assert_eq!(result, Err(AttenuationError::AddsAction));
    }

    #[test]
    fn delegation_cannot_add_claim() {
        let parent = base_capability();
        let result = parent.attenuate_to(
            CapabilityId::new(id(20)).unwrap(),
            p(30),
            IssuerProofRef::new(id(31)).unwrap(),
            vec![CareAction::View],
            ClaimSelector::new(vec![c(10), c(99)]).unwrap(),
            200,
            200,
            900,
            1,
            false,
            RevocationHandle::new(id(32)).unwrap(),
        );
        assert_eq!(result, Err(AttenuationError::AddsClaim));
    }

    #[test]
    fn delegation_must_reduce_remaining_depth() {
        let parent = base_capability();
        let result = parent.attenuate_to(
            CapabilityId::new(id(20)).unwrap(),
            p(30),
            IssuerProofRef::new(id(31)).unwrap(),
            vec![CareAction::View],
            ClaimSelector::new(vec![c(10)]).unwrap(),
            200,
            200,
            900,
            2,
            false,
            RevocationHandle::new(id(32)).unwrap(),
        );
        assert_eq!(result, Err(AttenuationError::DelegationDepthNotReduced));
    }

    #[test]
    fn delegation_cannot_outlive_parent() {
        let parent = base_capability();
        let result = parent.attenuate_to(
            CapabilityId::new(id(20)).unwrap(),
            p(30),
            IssuerProofRef::new(id(31)).unwrap(),
            vec![CareAction::View],
            ClaimSelector::new(vec![c(10)]).unwrap(),
            200,
            200,
            1_001,
            1,
            false,
            RevocationHandle::new(id(32)).unwrap(),
        );
        assert_eq!(result, Err(AttenuationError::OutlivesParent));
    }

    #[test]
    fn delegation_cannot_weaken_no_further_disclosure() {
        let mut parent = base_capability();
        parent.no_further_disclosure = true;
        let result = parent.attenuate_to(
            CapabilityId::new(id(20)).unwrap(),
            p(30),
            IssuerProofRef::new(id(31)).unwrap(),
            vec![CareAction::View],
            ClaimSelector::new(vec![c(10)]).unwrap(),
            200,
            200,
            900,
            1,
            false,
            RevocationHandle::new(id(32)).unwrap(),
        );
        assert_eq!(
            result,
            Err(AttenuationError::WeakensRedisclosureRestriction)
        );
    }

    #[test]
    fn valid_delegation_uses_parent_grantee_as_child_issuer() {
        let parent = base_capability();
        let child = parent
            .attenuate_to(
                CapabilityId::new(id(20)).unwrap(),
                p(30),
                IssuerProofRef::new(id(31)).unwrap(),
                vec![CareAction::View, CareAction::UseForCare],
                ClaimSelector::new(vec![c(10), c(11)]).unwrap(),
                200,
                200,
                900,
                1,
                true,
                RevocationHandle::new(id(32)).unwrap(),
            )
            .unwrap();
        assert_eq!(child.issuer, parent.grantee);
        assert_eq!(child.parent, Some(parent.capability_id));
        assert_eq!(child.purpose, parent.purpose);
        assert_eq!(child.key_epoch, parent.key_epoch);
    }

    #[test]
    fn revoked_capability_cannot_authorize_new_capsule() {
        let cap = base_capability();
        let mut state = CapabilityState::new(cap.clone());
        state.revoke(300).unwrap();
        let result = state.authorize_access_capsule(
            EnvelopeId::new(id(40)).unwrap(),
            cap.grantee,
            cap.policy_handle,
            cap.key_epoch,
            400,
        );
        assert_eq!(result, Err(CapsuleAuthorizationError::Revoked));
    }

    #[test]
    fn grantee_substitution_is_rejected() {
        let cap = base_capability();
        let state = CapabilityState::new(cap.clone());
        let result = state.authorize_access_capsule(
            EnvelopeId::new(id(40)).unwrap(),
            p(99),
            cap.policy_handle,
            cap.key_epoch,
            400,
        );
        assert_eq!(result, Err(CapsuleAuthorizationError::GranteeMismatch));
    }

    #[test]
    fn policy_substitution_is_rejected() {
        let cap = base_capability();
        let state = CapabilityState::new(cap.clone());
        let result = state.authorize_access_capsule(
            EnvelopeId::new(id(40)).unwrap(),
            cap.grantee,
            PolicyHandle::new(id(99)).unwrap(),
            cap.key_epoch,
            400,
        );
        assert_eq!(result, Err(CapsuleAuthorizationError::PolicyMismatch));
    }

    #[test]
    fn stale_or_future_key_epoch_is_rejected() {
        let cap = base_capability();
        let state = CapabilityState::new(cap.clone());
        for epoch in [cap.key_epoch - 1, cap.key_epoch + 1] {
            let result = state.authorize_access_capsule(
                EnvelopeId::new(id(40)).unwrap(),
                cap.grantee,
                cap.policy_handle,
                epoch,
                400,
            );
            assert_eq!(result, Err(CapsuleAuthorizationError::KeyEpochMismatch));
        }
    }

    #[test]
    fn expired_capability_cannot_authorize_capsule() {
        let cap = base_capability();
        let state = CapabilityState::new(cap.clone());
        let result = state.authorize_access_capsule(
            EnvelopeId::new(id(40)).unwrap(),
            cap.grantee,
            cap.policy_handle,
            cap.key_epoch,
            cap.expires_at_millis,
        );
        assert_eq!(result, Err(CapsuleAuthorizationError::Inactive));
    }

    #[test]
    fn active_capability_authorizes_only_structural_capsule_binding() {
        let cap = base_capability();
        let state = CapabilityState::new(cap.clone());
        let binding = state
            .authorize_access_capsule(
                EnvelopeId::new(id(40)).unwrap(),
                cap.grantee,
                cap.policy_handle,
                cap.key_epoch,
                400,
            )
            .unwrap();
        assert_eq!(binding.capability_id, cap.capability_id);
        assert_eq!(binding.grantee, cap.grantee);
        assert_eq!(binding.key_epoch, cap.key_epoch);
        assert_eq!(binding.expires_at_millis, cap.expires_at_millis);
    }

    #[test]
    fn emergency_capability_is_short_read_only_non_delegable() {
        let emergency = EmergencyCapabilityV1::new(
            CapabilityId::new(id(1)).unwrap(),
            p(2),
            p(3),
            SubjectContextHandle::new(id(4)).unwrap(),
            PolicyHandle::new(id(5)).unwrap(),
            IssuerProofRef::new(id(6)).unwrap(),
            ClaimSelector::new(vec![c(10)]).unwrap(),
            EmergencyReasonCommitment::new(id(7)).unwrap(),
            100,
            100,
            100 + MAX_EMERGENCY_TTL_MILLIS,
            1,
            RevocationHandle::new(id(8)).unwrap(),
        )
        .unwrap();

        assert_eq!(emergency.allowed_action(), CareAction::View);
        assert!(!emergency.delegation_allowed());
        assert!(!emergency.model_training_allowed());
        assert!(!emergency.research_allowed());
    }

    #[test]
    fn emergency_capability_cannot_exceed_max_ttl() {
        let result = EmergencyCapabilityV1::new(
            CapabilityId::new(id(1)).unwrap(),
            p(2),
            p(3),
            SubjectContextHandle::new(id(4)).unwrap(),
            PolicyHandle::new(id(5)).unwrap(),
            IssuerProofRef::new(id(6)).unwrap(),
            ClaimSelector::new(vec![c(10)]).unwrap(),
            EmergencyReasonCommitment::new(id(7)).unwrap(),
            100,
            100,
            101 + MAX_EMERGENCY_TTL_MILLIS,
            1,
            RevocationHandle::new(id(8)).unwrap(),
        );
        assert_eq!(
            result,
            Err(EmergencyCapabilityError::ExceedsMaximumTtl)
        );
    }

    #[test]
    fn public_commitment_contains_no_semantic_scope_fields() {
        let commitment = CapabilityCommitmentV1::new(
            CommitmentDigest::new(id(1)).unwrap(),
            RevocationHandle::new(id(2)).unwrap(),
            CapabilityPublicStatus::Active,
            1,
        )
        .unwrap();
        assert_eq!(commitment.version, CAPABILITY_COMMITMENT_V1);
        assert_eq!(commitment.status, CapabilityPublicStatus::Active);
    }

    #[test]
    fn opaque_identifiers_redact_debug_output() {
        let capability_id = CapabilityId::new(id(42)).unwrap();
        let debug = format!("{capability_id:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("42"));
    }
}
