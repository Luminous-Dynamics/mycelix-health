#![deny(unsafe_code)]
//! Typed clinical semantics and safety invariants for Mycelix-Health.
//!
//! This crate deliberately has no Holochain or transport dependency. Adapters may
//! be permissive while parsing external data, but a clinical fact must satisfy
//! [`ClinicalFact::validate_machine_actionable`] before downstream code may treat
//! it as machine-actionable clinical data.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical UCUM system URI used by FHIR quantities.
pub const UCUM_SYSTEM: &str = "http://unitsofmeasure.org";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coding {
    pub system: String,
    pub code: String,
    pub display: Option<String>,
    pub version: Option<String>,
}

impl Coding {
    fn validate(&self) -> Result<(), ClinicalSemanticsError> {
        if self.system.trim().is_empty() {
            return Err(ClinicalSemanticsError::MissingCodingSystem);
        }
        if self.code.trim().is_empty() {
            return Err(ClinicalSemanticsError::MissingCodingCode);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeableConcept {
    pub coding: Vec<Coding>,
    pub text: Option<String>,
}

impl CodeableConcept {
    pub fn validate_machine_actionable(&self) -> Result<(), ClinicalSemanticsError> {
        if self.coding.is_empty() {
            return Err(ClinicalSemanticsError::UncodedConcept);
        }
        for coding in &self.coding {
            coding.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubjectRef {
    pub resource_type: String,
    pub id: String,
}

impl SubjectRef {
    fn validate(&self) -> Result<(), ClinicalSemanticsError> {
        if self.resource_type.trim().is_empty() || self.id.trim().is_empty() {
            return Err(ClinicalSemanticsError::InvalidSubject);
        }
        Ok(())
    }
}

/// Quantitative clinical data with both display text and a computable unit code.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Quantity {
    pub value: f64,
    pub display_unit: Option<String>,
    pub system: String,
    pub code: String,
}

impl Quantity {
    /// Construct an explicitly UCUM-bound quantity.
    pub fn ucum(value: f64, code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            value,
            display_unit: Some(code.clone()),
            system: UCUM_SYSTEM.to_string(),
            code,
        }
    }

    /// Enforce the strict quantity contract used for computation.
    pub fn validate_machine_actionable(&self) -> Result<(), ClinicalSemanticsError> {
        if !self.value.is_finite() {
            return Err(ClinicalSemanticsError::NonFiniteNumber);
        }
        if self.system != UCUM_SYSTEM {
            return Err(ClinicalSemanticsError::NonUcumQuantity {
                system: self.system.clone(),
            });
        }
        if self.code.trim().is_empty() {
            return Err(ClinicalSemanticsError::MissingUnitCode);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Range {
    pub low: Option<Quantity>,
    pub high: Option<Quantity>,
}

impl Range {
    fn validate_machine_actionable(&self) -> Result<(), ClinicalSemanticsError> {
        if self.low.is_none() && self.high.is_none() {
            return Err(ClinicalSemanticsError::EmptyRange);
        }
        if let Some(low) = &self.low {
            low.validate_machine_actionable()?;
        }
        if let Some(high) = &self.high {
            high.validate_machine_actionable()?;
        }
        if let (Some(low), Some(high)) = (&self.low, &self.high) {
            if low.system != high.system || low.code != high.code {
                return Err(ClinicalSemanticsError::IncompatibleRangeUnits);
            }
            if low.value > high.value {
                return Err(ClinicalSemanticsError::InvertedRange);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ratio {
    pub numerator: Quantity,
    pub denominator: Quantity,
}

impl Ratio {
    fn validate_machine_actionable(&self) -> Result<(), ClinicalSemanticsError> {
        self.numerator.validate_machine_actionable()?;
        self.denominator.validate_machine_actionable()?;
        if self.denominator.value == 0.0 {
            return Err(ClinicalSemanticsError::ZeroRatioDenominator);
        }
        Ok(())
    }
}

/// Clinical value algebra. Narrative data is preserved explicitly rather than
/// coerced into a typed value.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClinicalValue {
    Quantity(Quantity),
    CodeableConcept(CodeableConcept),
    Range(Range),
    Ratio(Ratio),
    Boolean(bool),
    Integer(i64),
    Decimal(f64),
    DateTimeMicros(i64),
    Reference(SubjectRef),
    Narrative {
        text: String,
        reason_not_typed: String,
    },
}

impl ClinicalValue {
    pub fn validate_machine_actionable(&self) -> Result<(), ClinicalSemanticsError> {
        match self {
            Self::Quantity(value) => value.validate_machine_actionable(),
            Self::CodeableConcept(value) => value.validate_machine_actionable(),
            Self::Range(value) => value.validate_machine_actionable(),
            Self::Ratio(value) => value.validate_machine_actionable(),
            Self::Boolean(_) | Self::Integer(_) | Self::DateTimeMicros(_) => Ok(()),
            Self::Decimal(value) if value.is_finite() => Ok(()),
            Self::Decimal(_) => Err(ClinicalSemanticsError::NonFiniteNumber),
            Self::Reference(value) => value.validate(),
            Self::Narrative { .. } => Err(ClinicalSemanticsError::NarrativeOnly),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransformationProvenance {
    pub software: String,
    pub version: String,
    pub operation: String,
    pub input_fact_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FactProvenance {
    pub source_system: String,
    pub source_resource_type: String,
    pub source_resource_id: String,
    pub source_version: Option<String>,
    pub recorded_at_micros: i64,
    pub asserted_by: Option<String>,
    pub transformation: Option<TransformationProvenance>,
}

impl FactProvenance {
    fn validate(&self) -> Result<(), ClinicalSemanticsError> {
        if self.source_system.trim().is_empty()
            || self.source_resource_type.trim().is_empty()
            || self.source_resource_id.trim().is_empty()
        {
            return Err(ClinicalSemanticsError::IncompleteProvenance);
        }
        if let Some(transformation) = &self.transformation {
            if transformation.software.trim().is_empty()
                || transformation.version.trim().is_empty()
                || transformation.operation.trim().is_empty()
            {
                return Err(ClinicalSemanticsError::IncompleteTransformationProvenance);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Uncertainty {
    /// Optional calibrated probability/confidence. None means no calibrated
    /// numerical confidence is claimed.
    pub confidence: Option<f64>,
    pub interpretation: Option<String>,
}

impl Uncertainty {
    fn validate(&self) -> Result<(), ClinicalSemanticsError> {
        if let Some(confidence) = self.confidence {
            if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
                return Err(ClinicalSemanticsError::InvalidConfidence);
            }
        }
        Ok(())
    }
}

/// Minimum semantic unit permitted at a strict clinical trust boundary.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClinicalFact {
    pub fact_id: String,
    pub subject: SubjectRef,
    pub concept: CodeableConcept,
    pub value: ClinicalValue,
    /// Time the underlying clinical event/measurement occurred, distinct from
    /// provenance.recorded_at_micros.
    pub effective_at_micros: i64,
    pub provenance: FactProvenance,
    pub uncertainty: Option<Uncertainty>,
}

impl ClinicalFact {
    pub fn validate_machine_actionable(&self) -> Result<(), ClinicalSemanticsError> {
        if self.fact_id.trim().is_empty() {
            return Err(ClinicalSemanticsError::MissingFactId);
        }
        self.subject.validate()?;
        self.concept.validate_machine_actionable()?;
        self.value.validate_machine_actionable()?;
        self.provenance.validate()?;
        if let Some(uncertainty) = &self.uncertainty {
            uncertainty.validate()?;
        }
        Ok(())
    }
}

/// Four-state evaluation prevents missing information from becoming false
/// evidence for or against a clinical criterion.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvaluationState {
    Satisfied,
    NotSatisfied,
    Indeterminate,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationResult {
    pub state: EvaluationState,
    pub reasons: Vec<String>,
    pub evidence_fact_ids: Vec<String>,
    pub missing_requirements: Vec<String>,
}

impl EvaluationResult {
    pub fn indeterminate(requirement: impl Into<String>) -> Self {
        Self {
            state: EvaluationState::Indeterminate,
            reasons: Vec::new(),
            evidence_fact_ids: Vec::new(),
            missing_requirements: vec![requirement.into()],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubjectBinding {
    Match,
    Mismatch { expected: String, found: String },
    Unresolved {
        reference: Option<String>,
        reason: UnresolvedReferenceReason,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnresolvedReferenceReason {
    MissingReference,
    UnsupportedReferenceForm,
    WrongResourceType,
}

/// Resolve common FHIR R4 patient reference forms without guessing.
///
/// Relative (`Patient/123`) and absolute (`https://ehr/fhir/Patient/123`) forms
/// are compared directly. URN and contained references require Bundle context,
/// so they remain explicitly unresolved for a caller to resolve or quarantine.
pub fn bind_fhir_patient_reference(
    reference: Option<&str>,
    expected_patient_id: &str,
) -> SubjectBinding {
    let Some(reference) = reference.map(str::trim).filter(|value| !value.is_empty()) else {
        return SubjectBinding::Unresolved {
            reference: None,
            reason: UnresolvedReferenceReason::MissingReference,
        };
    };

    if reference.starts_with("urn:") || reference.starts_with('#') {
        return SubjectBinding::Unresolved {
            reference: Some(reference.to_string()),
            reason: UnresolvedReferenceReason::UnsupportedReferenceForm,
        };
    }

    let without_query = reference.split('?').next().unwrap_or(reference);
    let without_fragment = without_query.split('#').next().unwrap_or(without_query);
    let segments: Vec<&str> = without_fragment
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    if segments.len() == 2 && segments[0] != "Patient" {
        return SubjectBinding::Unresolved {
            reference: Some(reference.to_string()),
            reason: UnresolvedReferenceReason::WrongResourceType,
        };
    }

    let patient_id = segments
        .windows(2)
        .find(|pair| pair[0] == "Patient")
        .map(|pair| pair[1]);

    match patient_id {
        Some(id) if id == expected_patient_id => SubjectBinding::Match,
        Some(id) => SubjectBinding::Mismatch {
            expected: expected_patient_id.to_string(),
            found: id.to_string(),
        },
        None => SubjectBinding::Unresolved {
            reference: Some(reference.to_string()),
            reason: UnresolvedReferenceReason::UnsupportedReferenceForm,
        },
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClinicalSemanticsError {
    #[error("clinical fact id is required")]
    MissingFactId,
    #[error("clinical subject must have resource type and id")]
    InvalidSubject,
    #[error("machine-actionable concept requires at least one coding")]
    UncodedConcept,
    #[error("coding system is required")]
    MissingCodingSystem,
    #[error("coding code is required")]
    MissingCodingCode,
    #[error("numeric clinical value must be finite")]
    NonFiniteNumber,
    #[error("machine-actionable quantity requires UCUM; got {system}")]
    NonUcumQuantity { system: String },
    #[error("machine-actionable quantity requires a UCUM code")]
    MissingUnitCode,
    #[error("range requires at least one bound")]
    EmptyRange,
    #[error("range bounds must use the same unit encoding")]
    IncompatibleRangeUnits,
    #[error("range low bound exceeds high bound")]
    InvertedRange,
    #[error("ratio denominator cannot be zero")]
    ZeroRatioDenominator,
    #[error("narrative value is not machine-actionable")]
    NarrativeOnly,
    #[error("source provenance is incomplete")]
    IncompleteProvenance,
    #[error("transformation provenance is incomplete")]
    IncompleteTransformationProvenance,
    #[error("confidence must be finite and between 0 and 1")]
    InvalidConfidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glucose_concept() -> CodeableConcept {
        CodeableConcept {
            coding: vec![Coding {
                system: "http://loinc.org".into(),
                code: "14771-0".into(),
                display: Some("Glucose [Moles/volume] in Serum or Plasma".into()),
                version: None,
            }],
            text: Some("Glucose".into()),
        }
    }

    fn valid_fact() -> ClinicalFact {
        ClinicalFact {
            fact_id: "fact-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            concept: glucose_concept(),
            value: ClinicalValue::Quantity(Quantity::ucum(5.5, "mmol/L")),
            effective_at_micros: 1_700_000_000_000_000,
            provenance: FactProvenance {
                source_system: "https://ehr.example/fhir".into(),
                source_resource_type: "Observation".into(),
                source_resource_id: "obs-1".into(),
                source_version: Some("7".into()),
                recorded_at_micros: 1_700_000_000_100_000,
                asserted_by: Some("Practitioner/123".into()),
                transformation: Some(TransformationProvenance {
                    software: "mycelix-fhir-bridge".into(),
                    version: "1".into(),
                    operation: "FHIR R4 Observation -> ClinicalFact".into(),
                    input_fact_ids: vec![],
                }),
            },
            uncertainty: None,
        }
    }

    #[test]
    fn valid_ucum_fact_is_machine_actionable() {
        assert_eq!(valid_fact().validate_machine_actionable(), Ok(()));
    }

    #[test]
    fn display_only_unit_is_rejected() {
        let mut fact = valid_fact();
        fact.value = ClinicalValue::Quantity(Quantity {
            value: 5.5,
            display_unit: Some("mmol/L".into()),
            system: String::new(),
            code: String::new(),
        });
        assert!(matches!(
            fact.validate_machine_actionable(),
            Err(ClinicalSemanticsError::NonUcumQuantity { .. })
        ));
    }

    #[test]
    fn non_finite_number_is_rejected() {
        let mut fact = valid_fact();
        fact.value = ClinicalValue::Quantity(Quantity::ucum(f64::NAN, "mmol/L"));
        assert_eq!(
            fact.validate_machine_actionable(),
            Err(ClinicalSemanticsError::NonFiniteNumber)
        );
    }

    #[test]
    fn narrative_is_preserved_but_not_promoted() {
        let mut fact = valid_fact();
        fact.value = ClinicalValue::Narrative {
            text: "five point five".into(),
            reason_not_typed: "source supplied only free text".into(),
        };
        assert_eq!(
            fact.validate_machine_actionable(),
            Err(ClinicalSemanticsError::NarrativeOnly)
        );
    }

    #[test]
    fn uncoded_concept_is_rejected() {
        let mut fact = valid_fact();
        fact.concept = CodeableConcept {
            coding: vec![],
            text: Some("Glucose".into()),
        };
        assert_eq!(
            fact.validate_machine_actionable(),
            Err(ClinicalSemanticsError::UncodedConcept)
        );
    }

    #[test]
    fn range_units_must_match() {
        let range = Range {
            low: Some(Quantity::ucum(3.0, "mmol/L")),
            high: Some(Quantity::ucum(180.0, "mg/dL")),
        };
        assert_eq!(
            range.validate_machine_actionable(),
            Err(ClinicalSemanticsError::IncompatibleRangeUnits)
        );
    }

    #[test]
    fn relative_cross_patient_reference_is_mismatch() {
        assert_eq!(
            bind_fhir_patient_reference(Some("Patient/patient-b"), "patient-a"),
            SubjectBinding::Mismatch {
                expected: "patient-a".into(),
                found: "patient-b".into(),
            }
        );
    }

    #[test]
    fn absolute_patient_reference_is_comparable() {
        assert_eq!(
            bind_fhir_patient_reference(
                Some("https://ehr.example/fhir/Patient/patient-a"),
                "patient-a"
            ),
            SubjectBinding::Match
        );
    }

    #[test]
    fn historical_absolute_cross_patient_reference_is_mismatch() {
        assert_eq!(
            bind_fhir_patient_reference(
                Some("https://ehr.example/fhir/Patient/patient-b/_history/7"),
                "patient-a"
            ),
            SubjectBinding::Mismatch {
                expected: "patient-a".into(),
                found: "patient-b".into(),
            }
        );
    }

    #[test]
    fn urn_reference_is_explicitly_unresolved() {
        assert_eq!(
            bind_fhir_patient_reference(Some("urn:uuid:abc"), "patient-a"),
            SubjectBinding::Unresolved {
                reference: Some("urn:uuid:abc".into()),
                reason: UnresolvedReferenceReason::UnsupportedReferenceForm,
            }
        );
    }

    #[test]
    fn missing_reference_is_explicitly_unresolved() {
        assert_eq!(
            bind_fhir_patient_reference(None, "patient-a"),
            SubjectBinding::Unresolved {
                reference: None,
                reason: UnresolvedReferenceReason::MissingReference,
            }
        );
    }

    #[test]
    fn wrong_resource_type_is_not_treated_as_patient() {
        assert_eq!(
            bind_fhir_patient_reference(Some("Group/cohort-a"), "patient-a"),
            SubjectBinding::Unresolved {
                reference: Some("Group/cohort-a".into()),
                reason: UnresolvedReferenceReason::WrongResourceType,
            }
        );
    }

    #[test]
    fn invalid_confidence_is_rejected() {
        let mut fact = valid_fact();
        fact.uncertainty = Some(Uncertainty {
            confidence: Some(1.2),
            interpretation: None,
        });
        assert_eq!(
            fact.validate_machine_actionable(),
            Err(ClinicalSemanticsError::InvalidConfidence)
        );
    }

    #[test]
    fn indeterminate_preserves_missing_requirement() {
        let result = EvaluationResult::indeterminate("HbA1c within 90 days");
        assert_eq!(result.state, EvaluationState::Indeterminate);
        assert_eq!(result.missing_requirements, vec!["HbA1c within 90 days"]);
    }
}
