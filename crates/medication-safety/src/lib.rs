#![deny(unsafe_code)]
//! Coverage-aware medication safety evidence for Mycelix-Health.
//!
//! The central invariant is epistemic rather than pharmacological:
//!
//! **"No finding" is not the same as "safe".**
//!
//! A clearance is produced only when an explicit policy's required patient-context
//! components and safety checks are present, fresh, bound to the exact medication
//! artifact, evaluated against identified knowledge artifacts, and report complete
//! coverage with a clear outcome. Missing/partial/stale evidence is indeterminate.
//!
//! This crate does not contain a drug database or calculate clinical interactions.
//! Trusted adapters feed it verified evaluation evidence from qualified knowledge
//! sources. The output clearance is deliberately non-serializable/non-cloneable.

use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_semantics::SubjectRef;
use mycelix_fhir_medication_semantics::MedicationRequestArtifact;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use thiserror::Error;

const MAX_FUTURE_SKEW_MICROS: i64 = 300_000_000; // five minutes
const CONTEXT_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-safety-context-v1";
const POLICY_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-safety-policy-v1";
const CHECK_SCHEMA_TAG: &[u8] = b"mycelix-health/medication-safety-check-v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SafetyContextKind {
    ActiveMedications,
    AllergiesIntolerances,
    ActiveConditions,
    RenalFunction,
    HepaticFunction,
    PregnancyLactation,
    Age,
    Weight,
    Pharmacogenomics,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContextEvidenceState {
    /// Relevant clinical evidence is present in the bound evidence artifact.
    Available,
    /// An evidence-bearing process established absence, e.g. medication list reviewed
    /// and no other active medication was reported. This is not the same as an empty list.
    VerifiedAbsent,
    /// An evidence-bearing applicability decision established that this context is not
    /// applicable under the policy. Policies must opt in to accepting this state.
    NotApplicable,
}

/// One patient-context component bound to a separately verified clinical artifact.
///
/// It is serializable as part of an exact context snapshot but not deserializable into
/// trusted state. Construction requires an in-process `VerifiedDigest`.
#[derive(Serialize)]
pub struct ContextComponent {
    kind: SafetyContextKind,
    state: ContextEvidenceState,
    evidence_digest: StoredDigest,
    observed_at_micros: i64,
    valid_until_micros: Option<i64>,
}

impl ContextComponent {
    pub fn from_verified_evidence(
        kind: SafetyContextKind,
        state: ContextEvidenceState,
        evidence_digest: VerifiedDigest,
        observed_at_micros: i64,
        valid_until_micros: Option<i64>,
    ) -> Result<Self, MedicationSafetyError> {
        evidence_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        if let Some(valid_until) = valid_until_micros {
            if valid_until <= observed_at_micros {
                return Err(MedicationSafetyError::InvalidContextValidityWindow(kind));
            }
        }
        Ok(Self {
            kind,
            state,
            evidence_digest: evidence_digest.stored(),
            observed_at_micros,
            valid_until_micros,
        })
    }

    pub fn kind(&self) -> SafetyContextKind {
        self.kind
    }

    pub fn state(&self) -> ContextEvidenceState {
        self.state
    }

    pub fn evidence_digest(&self) -> StoredDigest {
        self.evidence_digest
    }

    pub fn observed_at_micros(&self) -> i64 {
        self.observed_at_micros
    }

    pub fn valid_until_micros(&self) -> Option<i64> {
        self.valid_until_micros
    }
}

/// Exact patient context against which medication safety was evaluated.
///
/// No `Deserialize` implementation is provided: callers cannot turn arbitrary wire
/// JSON into a trusted context snapshot without re-establishing verified components.
#[derive(Serialize)]
pub struct MedicationSafetyContextSnapshot {
    subject: SubjectRef,
    medication_artifact_digest: StoredDigest,
    components: Vec<ContextComponent>,
    assembled_at_micros: i64,
}

impl MedicationSafetyContextSnapshot {
    pub fn from_verified_components(
        medication: &MedicationRequestArtifact,
        components: Vec<ContextComponent>,
        assembled_at_micros: i64,
    ) -> Result<Self, MedicationSafetyError> {
        if components.is_empty() {
            return Err(MedicationSafetyError::EmptyContextSnapshot);
        }
        let mut kinds = HashSet::new();
        for component in &components {
            if !kinds.insert(component.kind) {
                return Err(MedicationSafetyError::DuplicateContextComponent(
                    component.kind,
                ));
            }
        }

        let medication_artifact_digest = medication.verified_digest()?;
        medication_artifact_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;

        Ok(Self {
            subject: medication.order().subject.clone(),
            medication_artifact_digest: medication_artifact_digest.stored(),
            components,
            assembled_at_micros,
        })
    }

    pub fn subject(&self) -> &SubjectRef {
        &self.subject
    }

    pub fn medication_artifact_digest(&self) -> StoredDigest {
        self.medication_artifact_digest
    }

    pub fn components(&self) -> &[ContextComponent] {
        &self.components
    }

    pub fn assembled_at_micros(&self) -> i64 {
        self.assembled_at_micros
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, MedicationSafetyError> {
        hash_json(DigestDomain::MedicationSafetyContext, CONTEXT_SCHEMA_TAG, self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SafetyCheckKind {
    DrugDrugInteraction,
    DrugAllergy,
    DuplicateTherapy,
    DrugDiseaseContraindication,
    DoseRange,
    RenalAdjustment,
    HepaticAdjustment,
    PregnancyLactation,
    AgeWeight,
    Pharmacogenomics,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContextRequirement {
    pub kind: SafetyContextKind,
    /// Maximum evidence age at the point of clearance. `None` means the policy relies
    /// only on the component's explicit `valid_until_micros` for that domain.
    pub max_age_micros: Option<i64>,
    pub allow_verified_absent: bool,
    pub allow_not_applicable: bool,
}

impl ContextRequirement {
    fn validate(&self) -> Result<(), MedicationSafetyError> {
        if self.max_age_micros.is_some_and(|age| age <= 0) {
            return Err(MedicationSafetyError::InvalidContextMaximumAge(self.kind));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckRequirement {
    pub kind: SafetyCheckKind,
    pub max_evaluation_age_micros: i64,
    pub max_knowledge_age_micros: i64,
}

impl CheckRequirement {
    fn validate(&self) -> Result<(), MedicationSafetyError> {
        if self.max_evaluation_age_micros <= 0 {
            return Err(MedicationSafetyError::InvalidCheckMaximumAge(self.kind));
        }
        if self.max_knowledge_age_micros <= 0 {
            return Err(MedicationSafetyError::InvalidKnowledgeMaximumAge(self.kind));
        }
        Ok(())
    }
}

/// Versioned safety-coverage policy. This says which evidence is required; it does
/// not establish that a specific medical rule or knowledge provider is clinically valid.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MedicationSafetyPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub required_context: Vec<ContextRequirement>,
    pub required_checks: Vec<CheckRequirement>,
    pub max_future_skew_micros: i64,
}

impl MedicationSafetyPolicyV1 {
    pub fn validate(&self) -> Result<(), MedicationSafetyError> {
        if self.schema_version != 1 {
            return Err(MedicationSafetyError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        if self.policy_id.trim().is_empty() {
            return Err(MedicationSafetyError::MissingPolicyId);
        }
        if self.required_context.is_empty() {
            return Err(MedicationSafetyError::PolicyHasNoContextRequirements);
        }
        if self.required_checks.is_empty() {
            return Err(MedicationSafetyError::PolicyHasNoCheckRequirements);
        }
        if self.max_future_skew_micros < 0
            || self.max_future_skew_micros > MAX_FUTURE_SKEW_MICROS
        {
            return Err(MedicationSafetyError::InvalidFutureSkew);
        }

        let mut context_kinds = HashSet::new();
        for requirement in &self.required_context {
            requirement.validate()?;
            if !context_kinds.insert(requirement.kind) {
                return Err(MedicationSafetyError::DuplicateContextRequirement(
                    requirement.kind,
                ));
            }
        }

        let mut check_kinds = HashSet::new();
        for requirement in &self.required_checks {
            requirement.validate()?;
            if !check_kinds.insert(requirement.kind) {
                return Err(MedicationSafetyError::DuplicateCheckRequirement(
                    requirement.kind,
                ));
            }
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<VerifiedDigest, MedicationSafetyError> {
        self.validate()?;
        hash_json(DigestDomain::MedicationSafetyPolicy, POLICY_SCHEMA_TAG, self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnowledgeCoverage {
    /// The evaluator attests that its named/versioned knowledge artifact covers the
    /// exact scope required by this safety policy/check for the evaluated artifact.
    CompleteForPolicy,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SafetyCheckOutcome {
    Clear,
    Warning,
    Blocked,
    Indeterminate,
}

#[derive(Serialize)]
struct SafetyCheckDigestMaterial {
    kind: SafetyCheckKind,
    medication_artifact_digest: StoredDigest,
    context_digest: StoredDigest,
    policy_digest: StoredDigest,
    knowledge_digest: StoredDigest,
    evaluator_digest: StoredDigest,
    coverage: KnowledgeCoverage,
    outcome: SafetyCheckOutcome,
    finding_count: u32,
    evaluated_at_micros: i64,
    knowledge_as_of_micros: i64,
}

/// Safety check evidence that crossed a trusted evaluator adapter boundary.
///
/// It intentionally has no serde implementation. A caller must supply already
/// verified artifact/context/policy/knowledge/evaluator digests, and the exact check
/// envelope receives its own domain-separated evaluation digest.
pub struct VerifiedSafetyCheck {
    kind: SafetyCheckKind,
    medication_artifact_digest: StoredDigest,
    context_digest: StoredDigest,
    policy_digest: StoredDigest,
    knowledge_digest: StoredDigest,
    evaluator_digest: StoredDigest,
    coverage: KnowledgeCoverage,
    outcome: SafetyCheckOutcome,
    finding_count: u32,
    evaluated_at_micros: i64,
    knowledge_as_of_micros: i64,
    evaluation_digest: VerifiedDigest,
}

impl VerifiedSafetyCheck {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_evaluation(
        kind: SafetyCheckKind,
        medication_artifact_digest: VerifiedDigest,
        context_digest: VerifiedDigest,
        policy_digest: VerifiedDigest,
        knowledge_digest: VerifiedDigest,
        evaluator_digest: VerifiedDigest,
        coverage: KnowledgeCoverage,
        outcome: SafetyCheckOutcome,
        finding_count: u32,
        evaluated_at_micros: i64,
        knowledge_as_of_micros: i64,
    ) -> Result<Self, MedicationSafetyError> {
        medication_artifact_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;
        context_digest.require_domain(DigestDomain::MedicationSafetyContext)?;
        policy_digest.require_domain(DigestDomain::MedicationSafetyPolicy)?;
        knowledge_digest.require_domain(DigestDomain::MedicationSafetyKnowledge)?;
        evaluator_digest.require_domain(DigestDomain::ClinicalArtifact)?;

        let material = SafetyCheckDigestMaterial {
            kind,
            medication_artifact_digest: medication_artifact_digest.stored(),
            context_digest: context_digest.stored(),
            policy_digest: policy_digest.stored(),
            knowledge_digest: knowledge_digest.stored(),
            evaluator_digest: evaluator_digest.stored(),
            coverage,
            outcome,
            finding_count,
            evaluated_at_micros,
            knowledge_as_of_micros,
        };
        let evaluation_digest = hash_json(
            DigestDomain::MedicationSafetyEvaluation,
            CHECK_SCHEMA_TAG,
            &material,
        )?;

        Ok(Self {
            kind,
            medication_artifact_digest: material.medication_artifact_digest,
            context_digest: material.context_digest,
            policy_digest: material.policy_digest,
            knowledge_digest: material.knowledge_digest,
            evaluator_digest: material.evaluator_digest,
            coverage,
            outcome,
            finding_count,
            evaluated_at_micros,
            knowledge_as_of_micros,
            evaluation_digest,
        })
    }

    pub fn kind(&self) -> SafetyCheckKind {
        self.kind
    }

    pub fn coverage(&self) -> KnowledgeCoverage {
        self.coverage
    }

    pub fn outcome(&self) -> SafetyCheckOutcome {
        self.outcome
    }

    pub fn finding_count(&self) -> u32 {
        self.finding_count
    }

    pub fn knowledge_digest(&self) -> StoredDigest {
        self.knowledge_digest
    }

    pub fn evaluator_digest(&self) -> StoredDigest {
        self.evaluator_digest
    }

    pub fn evaluation_digest(&self) -> StoredDigest {
        self.evaluation_digest.stored()
    }
}

/// Produce a domain-separated identity for the exact knowledge snapshot used by a
/// medication safety adapter. Version/license/source metadata should be included in
/// the caller's canonical bytes rather than conveyed only in display text.
pub fn hash_safety_knowledge_artifact(
    canonical_bytes: &[u8],
) -> Result<VerifiedDigest, MedicationSafetyError> {
    Ok(hash_canonical_bytes(
        DigestDomain::MedicationSafetyKnowledge,
        canonical_bytes,
    )?)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MedicationSafetyDecision {
    Cleared,
    RequiresReview,
    Blocked,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SafetyDecisionReason {
    MissingContext(SafetyContextKind),
    ContextFromFuture(SafetyContextKind),
    ContextStale(SafetyContextKind),
    ContextExpired(SafetyContextKind),
    ContextStateNotAllowed(SafetyContextKind),
    MissingCheck(SafetyCheckKind),
    CheckFromFuture(SafetyCheckKind),
    CheckStale(SafetyCheckKind),
    KnowledgeFromFuture(SafetyCheckKind),
    KnowledgeStale(SafetyCheckKind),
    IncompleteKnowledgeCoverage(SafetyCheckKind),
    CheckWarning(SafetyCheckKind),
    CheckBlocked(SafetyCheckKind),
    CheckIndeterminate(SafetyCheckKind),
}

pub struct MedicationSafetyEvaluation {
    pub decision: MedicationSafetyDecision,
    pub reasons: Vec<SafetyDecisionReason>,
    clearance: Option<MedicationSafetyClearance>,
}

impl MedicationSafetyEvaluation {
    pub fn clearance(&self) -> Option<&MedicationSafetyClearance> {
        self.clearance.as_ref()
    }

    pub fn into_clearance(self) -> Option<MedicationSafetyClearance> {
        self.clearance
    }
}

/// Single-owner proof that one exact medication artifact satisfied one exact safety
/// policy against one exact patient-context snapshot and complete/fresh required
/// check evidence. It intentionally implements no Clone/Copy/Serialize/Deserialize/Debug.
pub struct MedicationSafetyClearance {
    medication_artifact_digest: StoredDigest,
    context_digest: StoredDigest,
    policy_digest: StoredDigest,
    check_evaluation_digests: Vec<StoredDigest>,
    cleared_at_micros: i64,
}

impl MedicationSafetyClearance {
    pub fn medication_artifact_digest(&self) -> StoredDigest {
        self.medication_artifact_digest
    }

    pub fn context_digest(&self) -> StoredDigest {
        self.context_digest
    }

    pub fn policy_digest(&self) -> StoredDigest {
        self.policy_digest
    }

    pub fn check_evaluation_digests(&self) -> &[StoredDigest] {
        &self.check_evaluation_digests
    }

    pub fn cleared_at_micros(&self) -> i64 {
        self.cleared_at_micros
    }
}

pub fn evaluate_medication_safety(
    medication: &MedicationRequestArtifact,
    context: &MedicationSafetyContextSnapshot,
    policy: &MedicationSafetyPolicyV1,
    checks: &[VerifiedSafetyCheck],
    at_micros: i64,
) -> Result<MedicationSafetyEvaluation, MedicationSafetyError> {
    policy.validate()?;
    medication
        .validate_for_activation(at_micros)
        .map_err(|error| MedicationSafetyError::MedicationArtifact(error.to_string()))?;

    let medication_digest = medication.verified_digest()?;
    medication_digest.require_domain(DigestDomain::MedicationRequestArtifact)?;
    if context.medication_artifact_digest != medication_digest.stored() {
        return Err(MedicationSafetyError::ContextMedicationMismatch);
    }
    if context.subject != medication.order().subject {
        return Err(MedicationSafetyError::ContextSubjectMismatch);
    }
    let context_digest = context.verified_digest()?;
    let policy_digest = policy.verified_digest()?;

    let mut reasons = Vec::new();
    let mut indeterminate = false;

    for requirement in &policy.required_context {
        let Some(component) = context
            .components
            .iter()
            .find(|component| component.kind == requirement.kind)
        else {
            indeterminate = true;
            reasons.push(SafetyDecisionReason::MissingContext(requirement.kind));
            continue;
        };

        match evidence_age(
            component.observed_at_micros,
            at_micros,
            policy.max_future_skew_micros,
        ) {
            EvidenceAge::Future => {
                indeterminate = true;
                reasons.push(SafetyDecisionReason::ContextFromFuture(requirement.kind));
            }
            EvidenceAge::Age(age) => {
                if requirement.max_age_micros.is_some_and(|max| age > max as i128) {
                    indeterminate = true;
                    reasons.push(SafetyDecisionReason::ContextStale(requirement.kind));
                }
            }
        }

        if component
            .valid_until_micros
            .is_some_and(|valid_until| at_micros >= valid_until)
        {
            indeterminate = true;
            reasons.push(SafetyDecisionReason::ContextExpired(requirement.kind));
        }

        let state_allowed = match component.state {
            ContextEvidenceState::Available => true,
            ContextEvidenceState::VerifiedAbsent => requirement.allow_verified_absent,
            ContextEvidenceState::NotApplicable => requirement.allow_not_applicable,
        };
        if !state_allowed {
            indeterminate = true;
            reasons.push(SafetyDecisionReason::ContextStateNotAllowed(
                requirement.kind,
            ));
        }
    }

    let required: BTreeSet<SafetyCheckKind> =
        policy.required_checks.iter().map(|check| check.kind).collect();
    let mut supplied = HashSet::new();
    for check in checks {
        if !supplied.insert(check.kind) {
            return Err(MedicationSafetyError::DuplicateCheckEvidence(check.kind));
        }
        if !required.contains(&check.kind) {
            return Err(MedicationSafetyError::UnexpectedCheckEvidence(check.kind));
        }
        if check.medication_artifact_digest != medication_digest.stored() {
            return Err(MedicationSafetyError::CheckMedicationMismatch(check.kind));
        }
        if check.context_digest != context_digest.stored() {
            return Err(MedicationSafetyError::CheckContextMismatch(check.kind));
        }
        if check.policy_digest != policy_digest.stored() {
            return Err(MedicationSafetyError::CheckPolicyMismatch(check.kind));
        }
    }

    let mut blocked = false;
    let mut warning = false;
    let mut clearance_digests = Vec::with_capacity(policy.required_checks.len());

    for requirement in &policy.required_checks {
        let Some(check) = checks.iter().find(|check| check.kind == requirement.kind) else {
            indeterminate = true;
            reasons.push(SafetyDecisionReason::MissingCheck(requirement.kind));
            continue;
        };

        match evidence_age(
            check.evaluated_at_micros,
            at_micros,
            policy.max_future_skew_micros,
        ) {
            EvidenceAge::Future => {
                indeterminate = true;
                reasons.push(SafetyDecisionReason::CheckFromFuture(requirement.kind));
            }
            EvidenceAge::Age(age) if age > requirement.max_evaluation_age_micros as i128 => {
                indeterminate = true;
                reasons.push(SafetyDecisionReason::CheckStale(requirement.kind));
            }
            EvidenceAge::Age(_) => {}
        }

        match evidence_age(
            check.knowledge_as_of_micros,
            at_micros,
            policy.max_future_skew_micros,
        ) {
            EvidenceAge::Future => {
                indeterminate = true;
                reasons.push(SafetyDecisionReason::KnowledgeFromFuture(requirement.kind));
            }
            EvidenceAge::Age(age) if age > requirement.max_knowledge_age_micros as i128 => {
                indeterminate = true;
                reasons.push(SafetyDecisionReason::KnowledgeStale(requirement.kind));
            }
            EvidenceAge::Age(_) => {}
        }

        if check.coverage != KnowledgeCoverage::CompleteForPolicy {
            indeterminate = true;
            reasons.push(SafetyDecisionReason::IncompleteKnowledgeCoverage(
                requirement.kind,
            ));
        }

        match check.outcome {
            SafetyCheckOutcome::Clear => {}
            SafetyCheckOutcome::Warning => {
                warning = true;
                reasons.push(SafetyDecisionReason::CheckWarning(requirement.kind));
            }
            SafetyCheckOutcome::Blocked => {
                blocked = true;
                reasons.push(SafetyDecisionReason::CheckBlocked(requirement.kind));
            }
            SafetyCheckOutcome::Indeterminate => {
                indeterminate = true;
                reasons.push(SafetyDecisionReason::CheckIndeterminate(requirement.kind));
            }
        }
        clearance_digests.push(check.evaluation_digest.stored());
    }

    let decision = if blocked {
        MedicationSafetyDecision::Blocked
    } else if indeterminate {
        MedicationSafetyDecision::Indeterminate
    } else if warning {
        MedicationSafetyDecision::RequiresReview
    } else {
        MedicationSafetyDecision::Cleared
    };

    let clearance = if decision == MedicationSafetyDecision::Cleared {
        Some(MedicationSafetyClearance {
            medication_artifact_digest: medication_digest.stored(),
            context_digest: context_digest.stored(),
            policy_digest: policy_digest.stored(),
            check_evaluation_digests: clearance_digests,
            cleared_at_micros: at_micros,
        })
    } else {
        None
    };

    Ok(MedicationSafetyEvaluation {
        decision,
        reasons,
        clearance,
    })
}

enum EvidenceAge {
    Future,
    Age(i128),
}

fn evidence_age(evidence_at: i64, now: i64, allowed_future_skew: i64) -> EvidenceAge {
    let delta = now as i128 - evidence_at as i128;
    if delta < -(allowed_future_skew as i128) {
        EvidenceAge::Future
    } else {
        EvidenceAge::Age(delta.max(0))
    }
}

fn hash_json<T: Serialize>(
    domain: DigestDomain,
    schema_tag: &[u8],
    value: &T,
) -> Result<VerifiedDigest, MedicationSafetyError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| MedicationSafetyError::Serialization(error.to_string()))?;
    let mut framed = Vec::with_capacity(schema_tag.len() + 1 + encoded.len());
    framed.extend_from_slice(schema_tag);
    framed.push(0);
    framed.extend_from_slice(&encoded);
    Ok(hash_canonical_bytes(domain, &framed)?)
}

#[derive(Debug, Error)]
pub enum MedicationSafetyError {
    #[error("unsupported medication safety policy version {0}")]
    UnsupportedPolicyVersion(u16),
    #[error("medication safety policy id is required")]
    MissingPolicyId,
    #[error("medication safety policy requires patient-context requirements")]
    PolicyHasNoContextRequirements,
    #[error("medication safety policy requires safety checks")]
    PolicyHasNoCheckRequirements,
    #[error("medication safety policy future-skew allowance must be between 0 and five minutes")]
    InvalidFutureSkew,
    #[error("duplicate patient-context requirement: {0:?}")]
    DuplicateContextRequirement(SafetyContextKind),
    #[error("invalid maximum age for patient-context requirement: {0:?}")]
    InvalidContextMaximumAge(SafetyContextKind),
    #[error("duplicate safety-check requirement: {0:?}")]
    DuplicateCheckRequirement(SafetyCheckKind),
    #[error("invalid safety-check maximum evaluation age: {0:?}")]
    InvalidCheckMaximumAge(SafetyCheckKind),
    #[error("invalid safety-check maximum knowledge age: {0:?}")]
    InvalidKnowledgeMaximumAge(SafetyCheckKind),
    #[error("medication safety context snapshot cannot be empty")]
    EmptyContextSnapshot,
    #[error("duplicate patient-context component: {0:?}")]
    DuplicateContextComponent(SafetyContextKind),
    #[error("invalid validity window for patient-context component: {0:?}")]
    InvalidContextValidityWindow(SafetyContextKind),
    #[error("patient-context snapshot targets a different medication artifact")]
    ContextMedicationMismatch,
    #[error("patient-context snapshot subject differs from medication subject")]
    ContextSubjectMismatch,
    #[error("duplicate safety-check evidence supplied: {0:?}")]
    DuplicateCheckEvidence(SafetyCheckKind),
    #[error("unexpected safety-check evidence not required by policy: {0:?}")]
    UnexpectedCheckEvidence(SafetyCheckKind),
    #[error("safety-check evidence targets a different medication artifact: {0:?}")]
    CheckMedicationMismatch(SafetyCheckKind),
    #[error("safety-check evidence targets a different patient-context snapshot: {0:?}")]
    CheckContextMismatch(SafetyCheckKind),
    #[error("safety-check evidence was evaluated under a different policy: {0:?}")]
    CheckPolicyMismatch(SafetyCheckKind),
    #[error("resolved medication artifact failed activation preconditions: {0}")]
    MedicationArtifact(String),
    #[error("failed to serialize exact medication safety artifact: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("FHIR medication artifact error: {0}")]
    FhirMedication(String),
}

impl From<mycelix_fhir_medication_semantics::FhirMedicationError> for MedicationSafetyError {
    fn from(error: mycelix_fhir_medication_semantics::FhirMedicationError) -> Self {
        Self::FhirMedication(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_authority::PrincipalBinding;
    use mycelix_provider_principal::{
        ExternalIdentifier, ExternalReferenceBinding, ExternalResourceKey,
        VerifiedProviderPrincipalBinding,
    };
    use serde_json::json;

    fn clinical_digest(seed: u8) -> VerifiedDigest {
        hash_canonical_bytes(DigestDomain::ClinicalArtifact, &[seed]).unwrap()
    }

    fn provider_binding() -> VerifiedProviderPrincipalBinding {
        VerifiedProviderPrincipalBinding::from_verified_provider_record(
            PrincipalBinding([7; 32]),
            vec![ExternalReferenceBinding {
                source_system: "https://ehr.example/fhir".into(),
                resource: ExternalResourceKey {
                    resource_type: "Practitioner".into(),
                    id: "prac-1".into(),
                },
            }],
            vec![ExternalIdentifier {
                system: "http://hl7.org/fhir/sid/us-npi".into(),
                value: "1234567893".into(),
            }],
            0,
            Some(10_000_000),
            None,
            clinical_digest(1),
            clinical_digest(2),
            clinical_digest(3),
        )
        .unwrap()
    }

    fn medication(id: &str, resolved_at: i64) -> MedicationRequestArtifact {
        let resource = json!({
            "resourceType": "MedicationRequest",
            "id": id,
            "status": "active",
            "intent": "order",
            "medicationCodeableConcept": {
                "coding": [{
                    "system": "http://www.nlm.nih.gov/research/umls/rxnorm",
                    "code": "860975"
                }]
            },
            "subject": { "reference": "Patient/patient-a" },
            "requester": {
                "reference": "Practitioner/prac-1",
                "type": "Practitioner",
                "identifier": {
                    "system": "http://hl7.org/fhir/sid/us-npi",
                    "value": "1234567893"
                }
            },
            "dosageInstruction": [{
                "timing": { "repeat": { "frequency": 2, "period": 1, "periodUnit": "d" } },
                "route": {
                    "coding": [{"system": "http://snomed.info/sct", "code": "26643006"}]
                },
                "doseAndRate": [{
                    "doseQuantity": {
                        "value": 1,
                        "system": "http://unitsofmeasure.org",
                        "code": "{tablet}"
                    }
                }]
            }]
        });
        mycelix_fhir_medication_semantics::project_active_medication_request_strict(
            &resource,
            "patient-a",
            "https://ehr.example/fhir",
            resolved_at,
            &[provider_binding()],
        )
        .unwrap()
    }

    fn policy() -> MedicationSafetyPolicyV1 {
        MedicationSafetyPolicyV1 {
            schema_version: 1,
            policy_id: "outpatient-medication-safety-test-v1".into(),
            required_context: vec![
                ContextRequirement {
                    kind: SafetyContextKind::ActiveMedications,
                    max_age_micros: Some(100),
                    allow_verified_absent: true,
                    allow_not_applicable: false,
                },
                ContextRequirement {
                    kind: SafetyContextKind::AllergiesIntolerances,
                    max_age_micros: Some(100),
                    allow_verified_absent: true,
                    allow_not_applicable: false,
                },
            ],
            required_checks: vec![
                CheckRequirement {
                    kind: SafetyCheckKind::DrugDrugInteraction,
                    max_evaluation_age_micros: 100,
                    max_knowledge_age_micros: 1_000,
                },
                CheckRequirement {
                    kind: SafetyCheckKind::DrugAllergy,
                    max_evaluation_age_micros: 100,
                    max_knowledge_age_micros: 1_000,
                },
            ],
            max_future_skew_micros: 0,
        }
    }

    fn context(
        medication: &MedicationRequestArtifact,
        observed_at: i64,
    ) -> MedicationSafetyContextSnapshot {
        MedicationSafetyContextSnapshot::from_verified_components(
            medication,
            vec![
                ContextComponent::from_verified_evidence(
                    SafetyContextKind::ActiveMedications,
                    ContextEvidenceState::Available,
                    clinical_digest(11),
                    observed_at,
                    None,
                )
                .unwrap(),
                ContextComponent::from_verified_evidence(
                    SafetyContextKind::AllergiesIntolerances,
                    ContextEvidenceState::VerifiedAbsent,
                    clinical_digest(12),
                    observed_at,
                    None,
                )
                .unwrap(),
            ],
            observed_at,
        )
        .unwrap()
    }

    fn check(
        medication: &MedicationRequestArtifact,
        context: &MedicationSafetyContextSnapshot,
        policy: &MedicationSafetyPolicyV1,
        kind: SafetyCheckKind,
        coverage: KnowledgeCoverage,
        outcome: SafetyCheckOutcome,
        at: i64,
        seed: u8,
    ) -> VerifiedSafetyCheck {
        VerifiedSafetyCheck::from_verified_evaluation(
            kind,
            medication.verified_digest().unwrap(),
            context.verified_digest().unwrap(),
            policy.verified_digest().unwrap(),
            hash_safety_knowledge_artifact(&[seed, 1]).unwrap(),
            clinical_digest(seed),
            coverage,
            outcome,
            0,
            at,
            at,
        )
        .unwrap()
    }

    fn complete_clear_checks(
        medication: &MedicationRequestArtifact,
        context: &MedicationSafetyContextSnapshot,
        policy: &MedicationSafetyPolicyV1,
        at: i64,
    ) -> Vec<VerifiedSafetyCheck> {
        vec![
            check(
                medication,
                context,
                policy,
                SafetyCheckKind::DrugDrugInteraction,
                KnowledgeCoverage::CompleteForPolicy,
                SafetyCheckOutcome::Clear,
                at,
                21,
            ),
            check(
                medication,
                context,
                policy,
                SafetyCheckKind::DrugAllergy,
                KnowledgeCoverage::CompleteForPolicy,
                SafetyCheckOutcome::Clear,
                at,
                22,
            ),
        ]
    }

    #[test]
    fn complete_fresh_evidence_can_produce_clearance() {
        let medication = medication("rx-a", 100);
        let context = context(&medication, 100);
        let policy = policy();
        let checks = complete_clear_checks(&medication, &context, &policy, 100);

        let result = evaluate_medication_safety(&medication, &context, &policy, &checks, 150)
            .unwrap();
        assert_eq!(result.decision, MedicationSafetyDecision::Cleared);
        let clearance = result.into_clearance().expect("clearance expected");
        assert_eq!(clearance.check_evaluation_digests().len(), 2);
        assert_eq!(
            clearance.medication_artifact_digest(),
            medication.verified_digest().unwrap().stored()
        );
    }

    #[test]
    fn zero_findings_with_unknown_coverage_is_indeterminate_not_safe() {
        let medication = medication("rx-a", 100);
        let context = context(&medication, 100);
        let policy = policy();
        let mut checks = complete_clear_checks(&medication, &context, &policy, 100);
        checks[0] = check(
            &medication,
            &context,
            &policy,
            SafetyCheckKind::DrugDrugInteraction,
            KnowledgeCoverage::Unknown,
            SafetyCheckOutcome::Clear,
            100,
            31,
        );

        let result = evaluate_medication_safety(&medication, &context, &policy, &checks, 150)
            .unwrap();
        assert_eq!(result.decision, MedicationSafetyDecision::Indeterminate);
        assert!(result.clearance().is_none());
        assert!(result.reasons.contains(
            &SafetyDecisionReason::IncompleteKnowledgeCoverage(
                SafetyCheckKind::DrugDrugInteraction
            )
        ));
    }

    #[test]
    fn skipped_required_check_is_indeterminate() {
        let medication = medication("rx-a", 100);
        let context = context(&medication, 100);
        let policy = policy();
        let checks = vec![check(
            &medication,
            &context,
            &policy,
            SafetyCheckKind::DrugDrugInteraction,
            KnowledgeCoverage::CompleteForPolicy,
            SafetyCheckOutcome::Clear,
            100,
            21,
        )];

        let result = evaluate_medication_safety(&medication, &context, &policy, &checks, 150)
            .unwrap();
        assert_eq!(result.decision, MedicationSafetyDecision::Indeterminate);
        assert!(result
            .reasons
            .contains(&SafetyDecisionReason::MissingCheck(SafetyCheckKind::DrugAllergy)));
    }

    #[test]
    fn stale_patient_context_is_indeterminate() {
        let medication = medication("rx-a", 10);
        let context = context(&medication, 10);
        let policy = policy();
        let checks = complete_clear_checks(&medication, &context, &policy, 150);

        let result = evaluate_medication_safety(&medication, &context, &policy, &checks, 150)
            .unwrap();
        assert_eq!(result.decision, MedicationSafetyDecision::Indeterminate);
        assert!(result.reasons.iter().any(|reason| matches!(
            reason,
            SafetyDecisionReason::ContextStale(SafetyContextKind::ActiveMedications)
        )));
    }

    #[test]
    fn warning_requires_review_and_does_not_issue_clearance() {
        let medication = medication("rx-a", 100);
        let context = context(&medication, 100);
        let policy = policy();
        let mut checks = complete_clear_checks(&medication, &context, &policy, 100);
        checks[0] = check(
            &medication,
            &context,
            &policy,
            SafetyCheckKind::DrugDrugInteraction,
            KnowledgeCoverage::CompleteForPolicy,
            SafetyCheckOutcome::Warning,
            100,
            31,
        );

        let result = evaluate_medication_safety(&medication, &context, &policy, &checks, 150)
            .unwrap();
        assert_eq!(result.decision, MedicationSafetyDecision::RequiresReview);
        assert!(result.clearance().is_none());
    }

    #[test]
    fn known_blocker_dominates_missing_evidence() {
        let medication = medication("rx-a", 100);
        let context = context(&medication, 100);
        let policy = policy();
        let checks = vec![check(
            &medication,
            &context,
            &policy,
            SafetyCheckKind::DrugDrugInteraction,
            KnowledgeCoverage::CompleteForPolicy,
            SafetyCheckOutcome::Blocked,
            100,
            21,
        )];

        let result = evaluate_medication_safety(&medication, &context, &policy, &checks, 150)
            .unwrap();
        assert_eq!(result.decision, MedicationSafetyDecision::Blocked);
        assert!(result.clearance().is_none());
    }

    #[test]
    fn stale_knowledge_is_indeterminate_even_when_check_says_clear() {
        let medication = medication("rx-a", 100);
        let context = context(&medication, 100);
        let mut policy = policy();
        policy.required_checks[0].max_knowledge_age_micros = 10;
        let mut checks = complete_clear_checks(&medication, &context, &policy, 100);
        checks[0] = VerifiedSafetyCheck::from_verified_evaluation(
            SafetyCheckKind::DrugDrugInteraction,
            medication.verified_digest().unwrap(),
            context.verified_digest().unwrap(),
            policy.verified_digest().unwrap(),
            hash_safety_knowledge_artifact(b"old-kb").unwrap(),
            clinical_digest(41),
            KnowledgeCoverage::CompleteForPolicy,
            SafetyCheckOutcome::Clear,
            0,
            150,
            100,
        )
        .unwrap();

        let result = evaluate_medication_safety(&medication, &context, &policy, &checks, 150)
            .unwrap();
        assert_eq!(result.decision, MedicationSafetyDecision::Indeterminate);
        assert!(result.reasons.contains(
            &SafetyDecisionReason::KnowledgeStale(SafetyCheckKind::DrugDrugInteraction)
        ));
    }

    #[test]
    fn check_for_another_medication_is_rejected_not_reused() {
        let medication_a = medication("rx-a", 100);
        let medication_b = medication("rx-b", 100);
        let context = context(&medication_a, 100);
        let policy = policy();
        let wrong = VerifiedSafetyCheck::from_verified_evaluation(
            SafetyCheckKind::DrugDrugInteraction,
            medication_b.verified_digest().unwrap(),
            context.verified_digest().unwrap(),
            policy.verified_digest().unwrap(),
            hash_safety_knowledge_artifact(b"kb").unwrap(),
            clinical_digest(51),
            KnowledgeCoverage::CompleteForPolicy,
            SafetyCheckOutcome::Clear,
            0,
            100,
            100,
        )
        .unwrap();
        let allergy = check(
            &medication_a,
            &context,
            &policy,
            SafetyCheckKind::DrugAllergy,
            KnowledgeCoverage::CompleteForPolicy,
            SafetyCheckOutcome::Clear,
            100,
            52,
        );

        let error = match evaluate_medication_safety(
            &medication_a,
            &context,
            &policy,
            &[wrong, allergy],
            150,
        ) {
            Ok(_) => panic!("expected target mismatch"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            MedicationSafetyError::CheckMedicationMismatch(
                SafetyCheckKind::DrugDrugInteraction
            )
        ));
    }
}
