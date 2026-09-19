#![forbid(unsafe_code)]
//! Minimum-necessary care-disclosure reference semantics.
//!
//! This crate freezes the semantic boundary proposed by CARE-DISC-001 (#171).
//! It deliberately does **not** implement encryption, Holochain storage,
//! signatures, key distribution, legal consent, clinical decision-making, or AI
//! inference. Those are separate proof lines.
//!
//! Core boundary:
//! - the local [`CareVaultRecord`] describes a protected record in a person-owned
//!   semantic index;
//! - [`DisclosureProjection`] selects explicit claims for one purpose, audience,
//!   action set and finite validity window;
//! - [`SymthaeaCareTaskEnvelope`] derives a short-lived inference-only task from
//!   one projection;
//! - [`TaskOutputReceipt`] preserves task/projection/source provenance and can
//!   express only AI/non-clinical output authority.
//!
//! There is intentionally no wildcard selector, no vault/master key field, no
//! ambient Holochain capability field, no root care capability field, and no
//! switch that can turn model training or delegation on for a care task.

use core::fmt;
use std::collections::BTreeSet;

pub const CARE_DISCLOSURE_V1: u16 = 1;
pub const SYMTHAEA_CARE_TASK_V1: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueIdError {
    AllZero,
}

macro_rules! opaque_id32 {
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

opaque_id32!(VaultRecordId);
opaque_id32!(ClaimId);
opaque_id32!(ProjectionId);
opaque_id32!(TaskId);
opaque_id32!(SubjectContextHandle);
opaque_id32!(RecipientHandle);
opaque_id32!(ProvenanceRef);
opaque_id32!(CapabilityRef);
opaque_id32!(OutputDigest);

/// The epistemic/role authority of a source assertion.
///
/// Selection and summarization must preserve this class. A high-authority source
/// can inform an AI output, but the AI output never inherits the human source's
/// authority class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorityClass {
    PersonalNarrative,
    PersonalBelief,
    ObservedFact,
    ValidatedMeasure,
    ProfessionalObservation,
    ClinicalAssessment,
    ClinicalDiagnosis,
    AiHypothesis,
    ResearchEvidence,
    SpiritualInterpretation,
    ResearchHypothesis,
}

/// Local sensitivity class for vault organization and disclosure review.
/// This is not declared safe for DHT-clear publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SensitivityClass {
    ProtectedHealth,
    Psychotherapy,
    SubstanceUse,
    Crisis,
    SexualHealth,
    Genetic,
    PeerSupport,
    PersonalReflection,
    SpiritualOrPastoral,
}

/// A protected record as known to the person's local semantic vault.
///
/// This is an index/reference type, not the protected payload itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareVaultRecord {
    pub record_id: VaultRecordId,
    pub provenance: ProvenanceRef,
    pub authority: AuthorityClass,
    pub sensitivity: SensitivityClass,
}

/// Purpose is explicit and independent from action.
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

/// Fine-grained operations that can be authorized over selected material.
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

/// Conservative purpose/action compatibility.
///
/// A capability layer may be *more restrictive* than this table. It must never be
/// more permissive. In particular, care purposes never imply research, evaluation
/// or training rights.
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedClaim {
    pub claim_id: ClaimId,
    pub source_record: VaultRecordId,
    pub field_path: String,
    pub provenance: ProvenanceRef,
    pub authority: AuthorityClass,
    pub sensitivity: SensitivityClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedClaimError {
    EmptyFieldPath,
    FieldPathTooLong,
    ControlCharacter,
}

impl SelectedClaim {
    pub fn new(
        claim_id: ClaimId,
        source_record: VaultRecordId,
        field_path: impl Into<String>,
        provenance: ProvenanceRef,
        authority: AuthorityClass,
        sensitivity: SensitivityClass,
    ) -> Result<Self, SelectedClaimError> {
        let field_path = field_path.into();
        if field_path.trim().is_empty() {
            return Err(SelectedClaimError::EmptyFieldPath);
        }
        if field_path.len() > 256 {
            return Err(SelectedClaimError::FieldPathTooLong);
        }
        if field_path.chars().any(char::is_control) {
            return Err(SelectedClaimError::ControlCharacter);
        }
        Ok(Self {
            claim_id,
            source_record,
            field_path,
            provenance,
            authority,
            sensitivity,
        })
    }
}

/// Explicit, finite, purpose-bound disclosure.
///
/// There is intentionally no wildcard selector. Every disclosed claim is present
/// in `items`, and every source retains provenance + authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosureProjection {
    pub version: u16,
    pub projection_id: ProjectionId,
    pub subject_context: SubjectContextHandle,
    pub recipient: RecipientHandle,
    pub purpose: CarePurpose,
    pub actions: Vec<CareAction>,
    pub items: Vec<SelectedClaim>,
    pub capability: CapabilityRef,
    pub parent_projection: Option<ProjectionId>,
    pub issued_at_micros: i64,
    pub not_before_micros: i64,
    pub expires_at_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    EmptyActions,
    DuplicateAction,
    ActionIncompatibleWithPurpose,
    EmptyItems,
    DuplicateClaim,
    NotBeforePrecedesIssue,
    NonPositiveValidityWindow,
}

impl DisclosureProjection {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        projection_id: ProjectionId,
        subject_context: SubjectContextHandle,
        recipient: RecipientHandle,
        purpose: CarePurpose,
        actions: Vec<CareAction>,
        items: Vec<SelectedClaim>,
        capability: CapabilityRef,
        issued_at_micros: i64,
        not_before_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, ProjectionError> {
        validate_projection_parts(
            purpose,
            &actions,
            &items,
            issued_at_micros,
            not_before_micros,
            expires_at_micros,
        )?;

        Ok(Self {
            version: CARE_DISCLOSURE_V1,
            projection_id,
            subject_context,
            recipient,
            purpose,
            actions,
            items,
            capability,
            parent_projection: None,
            issued_at_micros,
            not_before_micros,
            expires_at_micros,
        })
    }

    pub fn contains_claim(&self, claim_id: ClaimId) -> bool {
        self.items.iter().any(|item| item.claim_id == claim_id)
    }

    pub fn allows_action(&self, action: CareAction) -> bool {
        self.actions.contains(&action)
    }

    /// Conservative attenuation: v1 keeps the same purpose, recipient, subject
    /// context and capability proof and permits only a subset of claims/actions
    /// with a validity interval contained within the parent interval.
    ///
    /// Delegating to a different recipient is a CARE-CAP operation and is not
    /// smuggled into projection attenuation.
    pub fn attenuate(
        &self,
        child_id: ProjectionId,
        actions: Vec<CareAction>,
        items: Vec<SelectedClaim>,
        not_before_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, AttenuationError> {
        validate_projection_parts(
            self.purpose,
            &actions,
            &items,
            self.issued_at_micros,
            not_before_micros,
            expires_at_micros,
        )
        .map_err(AttenuationError::InvalidProjection)?;

        if not_before_micros < self.not_before_micros {
            return Err(AttenuationError::StartsBeforeParent);
        }
        if expires_at_micros > self.expires_at_micros {
            return Err(AttenuationError::OutlivesParent);
        }
        if actions.iter().any(|action| !self.actions.contains(action)) {
            return Err(AttenuationError::AddsAction);
        }

        for child_item in &items {
            let Some(parent_item) = self
                .items
                .iter()
                .find(|parent| parent.claim_id == child_item.claim_id)
            else {
                return Err(AttenuationError::AddsClaim);
            };
            if child_item != parent_item {
                return Err(AttenuationError::MutatesClaimProvenanceOrAuthority);
            }
        }

        Ok(Self {
            version: CARE_DISCLOSURE_V1,
            projection_id: child_id,
            subject_context: self.subject_context,
            recipient: self.recipient,
            purpose: self.purpose,
            actions,
            items,
            capability: self.capability,
            parent_projection: Some(self.projection_id),
            issued_at_micros: self.issued_at_micros,
            not_before_micros,
            expires_at_micros,
        })
    }
}

fn validate_projection_parts(
    purpose: CarePurpose,
    actions: &[CareAction],
    items: &[SelectedClaim],
    issued_at_micros: i64,
    not_before_micros: i64,
    expires_at_micros: i64,
) -> Result<(), ProjectionError> {
    if actions.is_empty() {
        return Err(ProjectionError::EmptyActions);
    }
    let action_set: BTreeSet<_> = actions.iter().copied().collect();
    if action_set.len() != actions.len() {
        return Err(ProjectionError::DuplicateAction);
    }
    if actions
        .iter()
        .copied()
        .any(|action| !purpose_allows_action(purpose, action))
    {
        return Err(ProjectionError::ActionIncompatibleWithPurpose);
    }

    if items.is_empty() {
        return Err(ProjectionError::EmptyItems);
    }
    let claim_set: BTreeSet<_> = items.iter().map(|item| item.claim_id).collect();
    if claim_set.len() != items.len() {
        return Err(ProjectionError::DuplicateClaim);
    }

    if not_before_micros < issued_at_micros {
        return Err(ProjectionError::NotBeforePrecedesIssue);
    }
    if expires_at_micros <= not_before_micros {
        return Err(ProjectionError::NonPositiveValidityWindow);
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttenuationError {
    InvalidProjection(ProjectionError),
    StartsBeforeParent,
    OutlivesParent,
    AddsAction,
    AddsClaim,
    MutatesClaimProvenanceOrAuthority,
}

/// Bounded care-only task kinds. Autonomous diagnosis, prescribing, unrestricted
/// psychotherapy, coercive crisis action and capability delegation are absent by
/// construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymthaeaCareTaskKind {
    AppointmentPreparation,
    LongitudinalOrganization,
    EvidenceSynthesis,
    ReflectionOrganization,
    SymptomSleepTimeline,
    CarePlanQuestionPreparation,
    ComparativeSpiritualPerspectives,
}

/// AI-origin output authority. Deliberately excludes professional/clinical
/// authority classes such as diagnosis, order, prescription or professional
/// observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiOutputAuthority {
    AiDraft,
    AiHypothesis,
    EvidenceSummary,
    ReflectionPrompt,
    OrganizationSummary,
}

fn purpose_is_care_inference(purpose: CarePurpose) -> bool {
    matches!(
        purpose,
        CarePurpose::DirectCare
            | CarePurpose::CareCoordination
            | CarePurpose::MentalHealthCare
            | CarePurpose::MedicationManagement
            | CarePurpose::SpiritualCare
            | CarePurpose::PeerSupport
            | CarePurpose::CrisisPlanning
            | CarePurpose::CrisisResponse
    )
}

fn task_kind_compatible_with_purpose(kind: SymthaeaCareTaskKind, purpose: CarePurpose) -> bool {
    match kind {
        SymthaeaCareTaskKind::ComparativeSpiritualPerspectives => {
            purpose == CarePurpose::SpiritualCare
        }
        SymthaeaCareTaskKind::ReflectionOrganization => matches!(
            purpose,
            CarePurpose::MentalHealthCare
                | CarePurpose::SpiritualCare
                | CarePurpose::PeerSupport
                | CarePurpose::DirectCare
        ),
        SymthaeaCareTaskKind::SymptomSleepTimeline => matches!(
            purpose,
            CarePurpose::DirectCare
                | CarePurpose::MentalHealthCare
                | CarePurpose::MedicationManagement
                | CarePurpose::CrisisPlanning
        ),
        SymthaeaCareTaskKind::AppointmentPreparation
        | SymthaeaCareTaskKind::LongitudinalOrganization
        | SymthaeaCareTaskKind::EvidenceSynthesis
        | SymthaeaCareTaskKind::CarePlanQuestionPreparation => purpose_is_care_inference(purpose),
    }
}

fn output_compatible_with_task(
    kind: SymthaeaCareTaskKind,
    output: AiOutputAuthority,
) -> bool {
    match kind {
        SymthaeaCareTaskKind::EvidenceSynthesis => {
            matches!(output, AiOutputAuthority::EvidenceSummary | AiOutputAuthority::AiDraft)
        }
        SymthaeaCareTaskKind::ReflectionOrganization => matches!(
            output,
            AiOutputAuthority::ReflectionPrompt
                | AiOutputAuthority::OrganizationSummary
                | AiOutputAuthority::AiDraft
        ),
        SymthaeaCareTaskKind::LongitudinalOrganization
        | SymthaeaCareTaskKind::SymptomSleepTimeline => matches!(
            output,
            AiOutputAuthority::OrganizationSummary
                | AiOutputAuthority::AiHypothesis
                | AiOutputAuthority::AiDraft
        ),
        SymthaeaCareTaskKind::ComparativeSpiritualPerspectives => matches!(
            output,
            AiOutputAuthority::EvidenceSummary
                | AiOutputAuthority::ReflectionPrompt
                | AiOutputAuthority::AiDraft
        ),
        SymthaeaCareTaskKind::AppointmentPreparation
        | SymthaeaCareTaskKind::CarePlanQuestionPreparation => matches!(
            output,
            AiOutputAuthority::AiDraft
                | AiOutputAuthority::OrganizationSummary
                | AiOutputAuthority::ReflectionPrompt
        ),
    }
}

/// Short-lived, non-delegable, inference-only care task for Symthaea.
///
/// There is no model-training flag and no delegation flag to accidentally set.
/// The only exposed methods return `false` for those capabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaCareTaskEnvelope {
    pub version: u16,
    pub task_id: TaskId,
    pub projection_id: ProjectionId,
    pub purpose: CarePurpose,
    pub kind: SymthaeaCareTaskKind,
    pub input_claims: Vec<ClaimId>,
    pub expected_output: AiOutputAuthority,
    pub issued_at_micros: i64,
    pub expires_at_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskError {
    NonCarePurpose,
    ProjectionLacksCareUse,
    TaskKindIncompatibleWithPurpose,
    OutputIncompatibleWithTask,
    EmptyInputs,
    DuplicateInput,
    InputOutsideProjection,
    IssuedOutsideProjectionWindow,
    NonPositiveValidityWindow,
    OutlivesProjection,
}

impl SymthaeaCareTaskEnvelope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        projection: &DisclosureProjection,
        task_id: TaskId,
        kind: SymthaeaCareTaskKind,
        input_claims: Vec<ClaimId>,
        expected_output: AiOutputAuthority,
        issued_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, TaskError> {
        if !purpose_is_care_inference(projection.purpose) {
            return Err(TaskError::NonCarePurpose);
        }
        if !projection.allows_action(CareAction::View)
            || !projection.allows_action(CareAction::UseForCare)
        {
            return Err(TaskError::ProjectionLacksCareUse);
        }
        if !task_kind_compatible_with_purpose(kind, projection.purpose) {
            return Err(TaskError::TaskKindIncompatibleWithPurpose);
        }
        if !output_compatible_with_task(kind, expected_output) {
            return Err(TaskError::OutputIncompatibleWithTask);
        }
        if input_claims.is_empty() {
            return Err(TaskError::EmptyInputs);
        }
        let unique: BTreeSet<_> = input_claims.iter().copied().collect();
        if unique.len() != input_claims.len() {
            return Err(TaskError::DuplicateInput);
        }
        if input_claims
            .iter()
            .copied()
            .any(|claim| !projection.contains_claim(claim))
        {
            return Err(TaskError::InputOutsideProjection);
        }
        if issued_at_micros < projection.not_before_micros
            || issued_at_micros >= projection.expires_at_micros
        {
            return Err(TaskError::IssuedOutsideProjectionWindow);
        }
        if expires_at_micros <= issued_at_micros {
            return Err(TaskError::NonPositiveValidityWindow);
        }
        if expires_at_micros > projection.expires_at_micros {
            return Err(TaskError::OutlivesProjection);
        }

        Ok(Self {
            version: SYMTHAEA_CARE_TASK_V1,
            task_id,
            projection_id: projection.projection_id,
            purpose: projection.purpose,
            kind,
            input_claims,
            expected_output,
            issued_at_micros,
            expires_at_micros,
        })
    }

    pub const fn delegation_allowed(&self) -> bool {
        false
    }

    pub const fn model_training_allowed(&self) -> bool {
        false
    }

    pub const fn model_evaluation_allowed(&self) -> bool {
        false
    }

    pub const fn population_analytics_allowed(&self) -> bool {
        false
    }
}

/// Provenance receipt for an AI-produced artifact. The actual content remains in
/// a separately protected output object; this receipt freezes origin/authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskOutputReceipt {
    pub task_id: TaskId,
    pub projection_id: ProjectionId,
    pub source_claims: Vec<ClaimId>,
    pub authority: AiOutputAuthority,
    pub produced_at_micros: i64,
    pub output_digest: OutputDigest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputReceiptError {
    ProducedBeforeTask,
    ProducedAfterTaskExpiry,
    AuthorityMismatch,
}

impl TaskOutputReceipt {
    pub fn new(
        task: &SymthaeaCareTaskEnvelope,
        produced_at_micros: i64,
        authority: AiOutputAuthority,
        output_digest: OutputDigest,
    ) -> Result<Self, OutputReceiptError> {
        if produced_at_micros < task.issued_at_micros {
            return Err(OutputReceiptError::ProducedBeforeTask);
        }
        if produced_at_micros > task.expires_at_micros {
            return Err(OutputReceiptError::ProducedAfterTaskExpiry);
        }
        if authority != task.expected_output {
            return Err(OutputReceiptError::AuthorityMismatch);
        }
        Ok(Self {
            task_id: task.task_id,
            projection_id: task.projection_id,
            source_claims: task.input_claims.clone(),
            authority,
            produced_at_micros,
            output_digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn claim(byte: u8, authority: AuthorityClass) -> SelectedClaim {
        SelectedClaim::new(
            ClaimId::new(id(byte)).unwrap(),
            VaultRecordId::new(id(byte.wrapping_add(10))).unwrap(),
            format!("record.claim.{byte}"),
            ProvenanceRef::new(id(byte.wrapping_add(20))).unwrap(),
            authority,
            SensitivityClass::ProtectedHealth,
        )
        .unwrap()
    }

    fn care_projection() -> DisclosureProjection {
        DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::MentalHealthCare,
            vec![CareAction::View, CareAction::UseForCare],
            vec![
                claim(4, AuthorityClass::PersonalNarrative),
                claim(5, AuthorityClass::ValidatedMeasure),
            ],
            CapabilityRef::new(id(6)).unwrap(),
            100,
            100,
            1_000,
        )
        .unwrap()
    }

    #[test]
    fn projection_rejects_empty_items() {
        let result = DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::DirectCare,
            vec![CareAction::View],
            vec![],
            CapabilityRef::new(id(4)).unwrap(),
            10,
            10,
            20,
        );
        assert_eq!(result, Err(ProjectionError::EmptyItems));
    }

    #[test]
    fn projection_rejects_duplicate_claims() {
        let one = claim(4, AuthorityClass::ObservedFact);
        let result = DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::DirectCare,
            vec![CareAction::View],
            vec![one.clone(), one],
            CapabilityRef::new(id(6)).unwrap(),
            10,
            10,
            20,
        );
        assert_eq!(result, Err(ProjectionError::DuplicateClaim));
    }

    #[test]
    fn direct_care_cannot_gain_model_training_action() {
        let result = DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::DirectCare,
            vec![CareAction::View, CareAction::UseForModelTraining],
            vec![claim(4, AuthorityClass::ObservedFact)],
            CapabilityRef::new(id(6)).unwrap(),
            10,
            10,
            20,
        );
        assert_eq!(
            result,
            Err(ProjectionError::ActionIncompatibleWithPurpose)
        );
    }

    #[test]
    fn projection_requires_finite_positive_window() {
        let result = DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::DirectCare,
            vec![CareAction::View],
            vec![claim(4, AuthorityClass::ObservedFact)],
            CapabilityRef::new(id(6)).unwrap(),
            10,
            10,
            10,
        );
        assert_eq!(result, Err(ProjectionError::NonPositiveValidityWindow));
    }

    #[test]
    fn attenuation_cannot_add_claim() {
        let parent = care_projection();
        let result = parent.attenuate(
            ProjectionId::new(id(30)).unwrap(),
            vec![CareAction::View],
            vec![claim(99, AuthorityClass::ObservedFact)],
            200,
            900,
        );
        assert_eq!(result, Err(AttenuationError::AddsClaim));
    }

    #[test]
    fn attenuation_cannot_add_action() {
        let parent = care_projection();
        let result = parent.attenuate(
            ProjectionId::new(id(30)).unwrap(),
            vec![CareAction::View, CareAction::Annotate],
            vec![parent.items[0].clone()],
            200,
            900,
        );
        assert_eq!(result, Err(AttenuationError::AddsAction));
    }

    #[test]
    fn attenuation_cannot_extend_expiry() {
        let parent = care_projection();
        let result = parent.attenuate(
            ProjectionId::new(id(30)).unwrap(),
            vec![CareAction::View],
            vec![parent.items[0].clone()],
            200,
            1_001,
        );
        assert_eq!(result, Err(AttenuationError::OutlivesParent));
    }

    #[test]
    fn attenuation_preserves_claim_authority_and_provenance() {
        let parent = care_projection();
        let mut mutated = parent.items[0].clone();
        mutated.authority = AuthorityClass::ClinicalDiagnosis;
        let result = parent.attenuate(
            ProjectionId::new(id(30)).unwrap(),
            vec![CareAction::View],
            vec![mutated],
            200,
            900,
        );
        assert_eq!(
            result,
            Err(AttenuationError::MutatesClaimProvenanceOrAuthority)
        );
    }

    #[test]
    fn care_task_requires_projection_view_and_use_for_care() {
        let projection = DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::MentalHealthCare,
            vec![CareAction::View],
            vec![claim(4, AuthorityClass::ObservedFact)],
            CapabilityRef::new(id(6)).unwrap(),
            100,
            100,
            1_000,
        )
        .unwrap();

        let result = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::AppointmentPreparation,
            vec![projection.items[0].claim_id],
            AiOutputAuthority::AiDraft,
            200,
            300,
        );
        assert_eq!(result, Err(TaskError::ProjectionLacksCareUse));
    }

    #[test]
    fn model_training_projection_cannot_become_care_task() {
        let projection = DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::ModelTraining,
            vec![CareAction::View, CareAction::UseForModelTraining],
            vec![claim(4, AuthorityClass::ObservedFact)],
            CapabilityRef::new(id(6)).unwrap(),
            100,
            100,
            1_000,
        )
        .unwrap();

        let result = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::EvidenceSynthesis,
            vec![projection.items[0].claim_id],
            AiOutputAuthority::EvidenceSummary,
            200,
            300,
        );
        assert_eq!(result, Err(TaskError::NonCarePurpose));
    }

    #[test]
    fn care_task_inputs_must_be_projection_subset() {
        let projection = care_projection();
        let result = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::EvidenceSynthesis,
            vec![ClaimId::new(id(99)).unwrap()],
            AiOutputAuthority::EvidenceSummary,
            200,
            300,
        );
        assert_eq!(result, Err(TaskError::InputOutsideProjection));
    }

    #[test]
    fn care_task_cannot_outlive_projection() {
        let projection = care_projection();
        let result = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::EvidenceSynthesis,
            vec![projection.items[0].claim_id],
            AiOutputAuthority::EvidenceSummary,
            200,
            1_001,
        );
        assert_eq!(result, Err(TaskError::OutlivesProjection));
    }

    #[test]
    fn spiritual_comparison_requires_spiritual_care_purpose() {
        let projection = care_projection();
        let result = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::ComparativeSpiritualPerspectives,
            vec![projection.items[0].claim_id],
            AiOutputAuthority::EvidenceSummary,
            200,
            300,
        );
        assert_eq!(result, Err(TaskError::TaskKindIncompatibleWithPurpose));
    }

    #[test]
    fn care_task_is_structurally_non_delegable_and_inference_only() {
        let projection = care_projection();
        let task = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::EvidenceSynthesis,
            vec![projection.items[0].claim_id],
            AiOutputAuthority::EvidenceSummary,
            200,
            300,
        )
        .unwrap();

        assert!(!task.delegation_allowed());
        assert!(!task.model_training_allowed());
        assert!(!task.model_evaluation_allowed());
        assert!(!task.population_analytics_allowed());
    }

    #[test]
    fn output_receipt_preserves_task_projection_and_sources() {
        let projection = care_projection();
        let task = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::EvidenceSynthesis,
            vec![projection.items[0].claim_id, projection.items[1].claim_id],
            AiOutputAuthority::EvidenceSummary,
            200,
            300,
        )
        .unwrap();

        let receipt = TaskOutputReceipt::new(
            &task,
            250,
            AiOutputAuthority::EvidenceSummary,
            OutputDigest::new(id(10)).unwrap(),
        )
        .unwrap();

        assert_eq!(receipt.task_id, task.task_id);
        assert_eq!(receipt.projection_id, projection.projection_id);
        assert_eq!(receipt.source_claims, task.input_claims);
        assert_eq!(receipt.authority, AiOutputAuthority::EvidenceSummary);
    }

    #[test]
    fn output_receipt_cannot_claim_different_ai_authority() {
        let projection = care_projection();
        let task = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::EvidenceSynthesis,
            vec![projection.items[0].claim_id],
            AiOutputAuthority::EvidenceSummary,
            200,
            300,
        )
        .unwrap();

        let result = TaskOutputReceipt::new(
            &task,
            250,
            AiOutputAuthority::AiHypothesis,
            OutputDigest::new(id(10)).unwrap(),
        );
        assert_eq!(result, Err(OutputReceiptError::AuthorityMismatch));
    }

    #[test]
    fn opaque_identifiers_are_redacted_in_debug_output() {
        let projection_id = ProjectionId::new(id(42)).unwrap();
        let debug = format!("{projection_id:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("42"));
    }

    #[test]
    fn source_authority_does_not_promote_ai_output_authority() {
        let high_authority_claim = claim(7, AuthorityClass::ClinicalDiagnosis);
        let projection = DisclosureProjection::new(
            ProjectionId::new(id(1)).unwrap(),
            SubjectContextHandle::new(id(2)).unwrap(),
            RecipientHandle::new(id(3)).unwrap(),
            CarePurpose::DirectCare,
            vec![CareAction::View, CareAction::UseForCare],
            vec![high_authority_claim],
            CapabilityRef::new(id(6)).unwrap(),
            100,
            100,
            1_000,
        )
        .unwrap();

        let task = SymthaeaCareTaskEnvelope::new(
            &projection,
            TaskId::new(id(9)).unwrap(),
            SymthaeaCareTaskKind::CarePlanQuestionPreparation,
            vec![projection.items[0].claim_id],
            AiOutputAuthority::AiDraft,
            200,
            300,
        )
        .unwrap();

        assert_eq!(projection.items[0].authority, AuthorityClass::ClinicalDiagnosis);
        assert_eq!(task.expected_output, AiOutputAuthority::AiDraft);
    }
}
