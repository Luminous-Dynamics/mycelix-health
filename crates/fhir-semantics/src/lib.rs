#![deny(unsafe_code)]
//! Strict FHIR R4 projection into the Mycelix clinical semantics kernel.
//!
//! This is intentionally a projection layer, not a general FHIR validator. It
//! accepts the subset needed to convert common Observation resources into typed
//! clinical facts and rejects ambiguity rather than guessing.

use chrono::DateTime;
use mycelix_clinical_semantics::{
    bind_fhir_patient_reference, ClinicalFact, ClinicalSemanticsError, ClinicalValue, CodeableConcept,
    Coding, FactProvenance, Quantity, SubjectBinding, SubjectRef, TransformationProvenance,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const ADAPTER_NAME: &str = "mycelix-fhir-semantics";
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectionIssueKind {
    MissingPatient,
    SubjectMismatch,
    SubjectUnresolved,
    InvalidObservation,
    SemanticValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionIssue {
    pub resource_id: Option<String>,
    pub kind: ProjectionIssueKind,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BundleProjectionReport {
    pub patient_id: Option<String>,
    pub facts: Vec<ClinicalFact>,
    pub issues: Vec<ProjectionIssue>,
}

/// Project all Observation resources in a FHIR Bundle for the Patient established
/// by that Bundle. Unsafe ambiguity becomes an issue, never a silently attached
/// clinical fact.
pub fn project_bundle_observations(
    bundle: &Value,
    source_system: &str,
    recorded_at_micros: i64,
) -> BundleProjectionReport {
    let entries = bundle
        .get("entry")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let patient_id = entries.iter().find_map(|entry| {
        let resource = entry.get("resource")?;
        if resource.get("resourceType").and_then(Value::as_str) == Some("Patient") {
            resource.get("id").and_then(Value::as_str).map(str::to_string)
        } else {
            None
        }
    });

    let Some(expected_patient_id) = patient_id.as_deref() else {
        return BundleProjectionReport {
            patient_id: None,
            facts: Vec::new(),
            issues: vec![ProjectionIssue {
                resource_id: None,
                kind: ProjectionIssueKind::MissingPatient,
                message: "FHIR Bundle does not establish a Patient id".to_string(),
            }],
        };
    };

    let mut facts = Vec::new();
    let mut issues = Vec::new();

    for entry in &entries {
        let Some(resource) = entry.get("resource") else {
            continue;
        };
        if resource.get("resourceType").and_then(Value::as_str) != Some("Observation") {
            continue;
        }

        match project_observation_strict(
            resource,
            expected_patient_id,
            source_system,
            recorded_at_micros,
        ) {
            Ok(fact) => facts.push(fact),
            Err(error) => issues.push(ProjectionIssue {
                resource_id: resource
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                kind: error.issue_kind(),
                message: error.to_string(),
            }),
        }
    }

    BundleProjectionReport {
        patient_id,
        facts,
        issues,
    }
}

/// Project one FHIR R4 Observation and require it to satisfy the clinical
/// semantics kernel before returning it.
pub fn project_observation_strict(
    resource: &Value,
    expected_patient_id: &str,
    source_system: &str,
    recorded_at_micros: i64,
) -> Result<ClinicalFact, FhirSemanticError> {
    if resource.get("resourceType").and_then(Value::as_str) != Some("Observation") {
        return Err(FhirSemanticError::WrongResourceType);
    }

    let resource_id = required_string(resource, "id")?;
    let subject_reference = resource
        .get("subject")
        .and_then(|subject| subject.get("reference"))
        .and_then(Value::as_str);

    match bind_fhir_patient_reference(subject_reference, expected_patient_id) {
        SubjectBinding::Match => {}
        SubjectBinding::Mismatch { expected, found } => {
            return Err(FhirSemanticError::SubjectMismatch { expected, found });
        }
        SubjectBinding::Unresolved { reference, reason } => {
            return Err(FhirSemanticError::SubjectUnresolved {
                reference,
                reason: format!("{reason:?}"),
            });
        }
    }

    let concept = parse_codeable_concept(
        resource
            .get("code")
            .ok_or(FhirSemanticError::MissingField("code"))?,
    )?;
    let value = parse_observation_value(resource)?;
    let effective_at_micros = parse_effective_time(resource)?;

    let source_version = resource
        .get("meta")
        .and_then(|meta| meta.get("versionId"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let asserted_by = resource
        .get("performer")
        .and_then(Value::as_array)
        .and_then(|performers| performers.first())
        .and_then(|performer| performer.get("reference"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let fact = ClinicalFact {
        fact_id: format!("fhir:{source_system}:Observation:{resource_id}"),
        subject: SubjectRef {
            resource_type: "Patient".to_string(),
            id: expected_patient_id.to_string(),
        },
        concept,
        value,
        effective_at_micros,
        provenance: FactProvenance {
            source_system: source_system.to_string(),
            source_resource_type: "Observation".to_string(),
            source_resource_id: resource_id,
            source_version,
            recorded_at_micros,
            asserted_by,
            transformation: Some(TransformationProvenance {
                software: ADAPTER_NAME.to_string(),
                version: ADAPTER_VERSION.to_string(),
                operation: "FHIR R4 Observation -> ClinicalFact".to_string(),
                input_fact_ids: Vec::new(),
            }),
        },
        uncertainty: None,
    };

    fact.validate_machine_actionable()?;
    Ok(fact)
}

fn parse_observation_value(resource: &Value) -> Result<ClinicalValue, FhirSemanticError> {
    if let Some(quantity) = resource.get("valueQuantity") {
        return Ok(ClinicalValue::Quantity(parse_quantity(quantity)?));
    }
    if let Some(concept) = resource.get("valueCodeableConcept") {
        return Ok(ClinicalValue::CodeableConcept(parse_codeable_concept(concept)?));
    }
    if let Some(value) = resource.get("valueBoolean").and_then(Value::as_bool) {
        return Ok(ClinicalValue::Boolean(value));
    }
    if let Some(value) = resource.get("valueInteger").and_then(Value::as_i64) {
        return Ok(ClinicalValue::Integer(value));
    }
    if let Some(text) = resource.get("valueString").and_then(Value::as_str) {
        return Ok(ClinicalValue::Narrative {
            text: text.to_string(),
            reason_not_typed: "FHIR Observation.valueString has no stronger typed value".to_string(),
        });
    }

    Err(FhirSemanticError::UnsupportedObservationValue)
}

fn parse_quantity(value: &Value) -> Result<Quantity, FhirSemanticError> {
    let number = value
        .get("value")
        .and_then(Value::as_f64)
        .ok_or(FhirSemanticError::InvalidQuantity)?;

    Ok(Quantity {
        value: number,
        display_unit: value.get("unit").and_then(Value::as_str).map(str::to_string),
        system: value
            .get("system")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        code: value
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn parse_codeable_concept(value: &Value) -> Result<CodeableConcept, FhirSemanticError> {
    let coding = value
        .get("coding")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| Coding {
                    system: item
                        .get("system")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    code: item
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    display: item.get("display").and_then(Value::as_str).map(str::to_string),
                    version: item.get("version").and_then(Value::as_str).map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(CodeableConcept {
        coding,
        text: value.get("text").and_then(Value::as_str).map(str::to_string),
    })
}

fn parse_effective_time(resource: &Value) -> Result<i64, FhirSemanticError> {
    let value = resource
        .get("effectiveDateTime")
        .and_then(Value::as_str)
        .or_else(|| resource.get("issued").and_then(Value::as_str))
        .ok_or(FhirSemanticError::MissingEffectiveTime)?;

    DateTime::parse_from_rfc3339(value)
        .map(|date| date.timestamp_micros())
        .map_err(|_| FhirSemanticError::InvalidDateTime(value.to_string()))
}

fn required_string(resource: &Value, field: &'static str) -> Result<String, FhirSemanticError> {
    resource
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(FhirSemanticError::MissingField(field))
}

#[derive(Debug, Error)]
pub enum FhirSemanticError {
    #[error("expected FHIR Observation resource")]
    WrongResourceType,
    #[error("required FHIR field is missing: {0}")]
    MissingField(&'static str),
    #[error("Observation subject mismatch: expected Patient/{expected}, found Patient/{found}")]
    SubjectMismatch { expected: String, found: String },
    #[error("Observation subject could not be resolved safely: {reference:?} ({reason})")]
    SubjectUnresolved {
        reference: Option<String>,
        reason: String,
    },
    #[error("Observation has no supported typed value[x]")]
    UnsupportedObservationValue,
    #[error("Observation valueQuantity requires a numeric value")]
    InvalidQuantity,
    #[error("Observation requires effectiveDateTime or issued for strict projection")]
    MissingEffectiveTime,
    #[error("invalid FHIR dateTime: {0}")]
    InvalidDateTime(String),
    #[error(transparent)]
    ClinicalSemantics(#[from] ClinicalSemanticsError),
}

impl FhirSemanticError {
    fn issue_kind(&self) -> ProjectionIssueKind {
        match self {
            Self::SubjectMismatch { .. } => ProjectionIssueKind::SubjectMismatch,
            Self::SubjectUnresolved { .. } => ProjectionIssueKind::SubjectUnresolved,
            Self::ClinicalSemantics(_) => ProjectionIssueKind::SemanticValidation,
            _ => ProjectionIssueKind::InvalidObservation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{ClinicalValue, SubjectBinding, UCUM_SYSTEM};
    use serde_json::json;

    fn valid_observation() -> Value {
        json!({
            "resourceType": "Observation",
            "id": "obs-1",
            "status": "final",
            "subject": { "reference": "Patient/patient-a" },
            "code": {
                "coding": [{
                    "system": "http://loinc.org",
                    "code": "14771-0",
                    "display": "Glucose [Moles/volume] in Serum or Plasma"
                }]
            },
            "effectiveDateTime": "2026-09-09T10:00:00+02:00",
            "valueQuantity": {
                "value": 5.5,
                "unit": "mmol/L",
                "system": "http://unitsofmeasure.org",
                "code": "mmol/L"
            }
        })
    }

    #[test]
    fn projects_valid_quantity_without_losing_unit_semantics() {
        let fact = project_observation_strict(
            &valid_observation(),
            "patient-a",
            "https://ehr.example/fhir",
            1_700_000_000_000_000,
        )
        .expect("valid Observation should project");

        let ClinicalValue::Quantity(quantity) = fact.value else {
            panic!("expected Quantity");
        };
        assert_eq!(quantity.value, 5.5);
        assert_eq!(quantity.display_unit.as_deref(), Some("mmol/L"));
        assert_eq!(quantity.system, UCUM_SYSTEM);
        assert_eq!(quantity.code, "mmol/L");
    }

    #[test]
    fn missing_ucum_code_is_not_machine_actionable() {
        let mut observation = valid_observation();
        observation["valueQuantity"].as_object_mut().unwrap().remove("code");
        let error = project_observation_strict(
            &observation,
            "patient-a",
            "https://ehr.example/fhir",
            1_700_000_000_000_000,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            FhirSemanticError::ClinicalSemantics(ClinicalSemanticsError::MissingUnitCode)
        ));
    }

    #[test]
    fn cross_patient_observation_is_rejected() {
        let mut observation = valid_observation();
        observation["subject"]["reference"] = Value::String("Patient/patient-b".into());
        let error = project_observation_strict(
            &observation,
            "patient-a",
            "https://ehr.example/fhir",
            1_700_000_000_000_000,
        )
        .unwrap_err();
        assert!(matches!(error, FhirSemanticError::SubjectMismatch { .. }));
    }

    #[test]
    fn urn_subject_is_quarantined_until_bundle_resolution_exists() {
        let mut observation = valid_observation();
        observation["subject"]["reference"] = Value::String("urn:uuid:patient-a".into());
        let error = project_observation_strict(
            &observation,
            "patient-a",
            "https://ehr.example/fhir",
            1_700_000_000_000_000,
        )
        .unwrap_err();
        assert!(matches!(error, FhirSemanticError::SubjectUnresolved { .. }));
    }

    #[test]
    fn bundle_fixture_cannot_attach_cross_patient_observation() {
        let bundle: Value = serde_json::from_str(include_str!(
            "../../../tests/clinical-semantics/fhir-observation-cross-patient.json"
        ))
        .unwrap();
        let report = project_bundle_observations(
            &bundle,
            "https://ehr.example/fhir",
            1_700_000_000_000_000,
        );

        assert_eq!(report.patient_id.as_deref(), Some("patient-a"));
        assert!(report.facts.is_empty());
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].kind, ProjectionIssueKind::SubjectMismatch);
    }

    #[test]
    fn kernel_binding_helper_remains_consistent_with_adapter_expectation() {
        assert_eq!(
            bind_fhir_patient_reference(Some("Patient/patient-a"), "patient-a"),
            SubjectBinding::Match
        );
    }
}
