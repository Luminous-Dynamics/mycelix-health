#![deny(unsafe_code)]
//! Strict FHIR R4 bundle preflight for machine-actionable clinical promotion.
//!
//! This crate is deliberately stricter than general FHIR validity. It validates
//! the subset of bundle/Observation semantics that Mycelix-Health currently knows
//! how to promote safely, then delegates typed Observation projection to
//! `mycelix-fhir-semantics`.

use mycelix_clinical_semantics::ClinicalFact;
use mycelix_fhir_semantics::{project_bundle_observations, ProjectionIssueKind};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Ambiguity affects bundle identity or attribution. No facts may promote.
    Fatal,
    /// One resource is unsafe to promote; other independent resources may proceed.
    ResourceRejected,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConformanceIssueKind {
    EmptySourceSystem,
    WrongResourceType,
    MissingBundleType,
    UnsupportedBundleType,
    MissingEntries,
    InvalidPatient,
    MissingPatient,
    MultiplePatients,
    DuplicateResourceIdentity,
    DuplicateFullUrlVersion,
    VersionSpecificFullUrl,
    MissingObservationId,
    MissingObservationStatus,
    NonPromotableObservationStatus,
    MissingEffectiveTime,
    UnsupportedEffectiveForm,
    Projection(ProjectionIssueKind),
    DuplicateFactIdentity,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConformanceIssue {
    pub severity: IssueSeverity,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub kind: ConformanceIssueKind,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BundleConformanceReport {
    pub patient_id: Option<String>,
    pub facts: Vec<ClinicalFact>,
    pub issues: Vec<ConformanceIssue>,
}

impl BundleConformanceReport {
    pub fn has_fatal_issues(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Fatal)
    }
}

/// Strictly preflight a FHIR R4 Bundle and project only promotable Observations.
///
/// v1 intentionally supports only `collection` and `searchset` bundles. Other
/// bundle types have transaction/history/document/message semantics that need
/// their own validated handling rather than accidental reuse of this path.
pub fn conform_and_project_observations(
    bundle: &Value,
    source_system: &str,
    recorded_at_micros: i64,
) -> BundleConformanceReport {
    let mut report = BundleConformanceReport {
        patient_id: None,
        facts: Vec::new(),
        issues: Vec::new(),
    };

    if source_system.trim().is_empty() {
        push_fatal(
            &mut report,
            ConformanceIssueKind::EmptySourceSystem,
            None,
            None,
            "source_system is required for provenance",
        );
        return report;
    }

    if bundle.get("resourceType").and_then(Value::as_str) != Some("Bundle") {
        push_fatal(
            &mut report,
            ConformanceIssueKind::WrongResourceType,
            None,
            None,
            "strict FHIR promotion requires resourceType=Bundle",
        );
        return report;
    }

    let Some(bundle_type) = bundle.get("type").and_then(Value::as_str) else {
        push_fatal(
            &mut report,
            ConformanceIssueKind::MissingBundleType,
            Some("Bundle"),
            bundle_id(bundle),
            "FHIR Bundle.type is required",
        );
        return report;
    };

    if !matches!(bundle_type, "collection" | "searchset") {
        push_fatal(
            &mut report,
            ConformanceIssueKind::UnsupportedBundleType,
            Some("Bundle"),
            bundle_id(bundle),
            &format!(
                "bundle type {bundle_type:?} is not supported by strict promotion v1; \
                 use a purpose-built handler rather than discarding its workflow semantics"
            ),
        );
        return report;
    }

    let Some(entries) = bundle.get("entry").and_then(Value::as_array) else {
        push_fatal(
            &mut report,
            ConformanceIssueKind::MissingEntries,
            Some("Bundle"),
            bundle_id(bundle),
            "FHIR Bundle.entry must be an array for strict promotion",
        );
        return report;
    };

    validate_bundle_identity(entries, &mut report);
    validate_single_patient(entries, &mut report);

    // Bundle-level ambiguity poisons attribution. Do not attempt partial
    // projection when patient/resource identity is not unique.
    if report.has_fatal_issues() {
        return report;
    }

    // Own the ID rather than borrowing it from report: resource-level checks
    // below mutate report while the expected subject remains needed later.
    let expected_patient_id = report
        .patient_id
        .clone()
        .expect("single-patient validation established a patient id");

    let mut safe_entries = Vec::new();

    for entry in entries {
        let Some(resource) = entry.get("resource") else {
            continue;
        };
        let resource_type = resource.get("resourceType").and_then(Value::as_str);

        if resource_type == Some("Patient") {
            safe_entries.push(entry.clone());
            continue;
        }

        if resource_type != Some("Observation") {
            continue;
        }

        if observation_is_promotable(resource, &mut report) {
            safe_entries.push(entry.clone());
        }
    }

    let filtered_bundle = Value::Object(Map::from_iter([
        ("resourceType".to_string(), Value::String("Bundle".to_string())),
        ("type".to_string(), Value::String(bundle_type.to_string())),
        ("entry".to_string(), Value::Array(safe_entries)),
    ]));

    let projection =
        project_bundle_observations(&filtered_bundle, source_system, recorded_at_micros);

    // The preflight patient and the projection patient must agree. The projection
    // crate derives this from the filtered bundle, so disagreement would indicate
    // an internal contract failure rather than external data ambiguity.
    if projection.patient_id.as_deref() != Some(expected_patient_id.as_str()) {
        push_fatal(
            &mut report,
            ConformanceIssueKind::InvalidPatient,
            Some("Patient"),
            projection.patient_id.as_deref(),
            "filtered bundle patient identity changed across the projection boundary",
        );
        return report;
    }

    for issue in projection.issues {
        report.issues.push(ConformanceIssue {
            severity: IssueSeverity::ResourceRejected,
            resource_type: Some("Observation".to_string()),
            resource_id: issue.resource_id,
            kind: ConformanceIssueKind::Projection(issue.kind),
            message: issue.message,
        });
    }

    let mut fact_ids = HashSet::new();
    for fact in projection.facts {
        if !fact_ids.insert(fact.fact_id.clone()) {
            push_fatal(
                &mut report,
                ConformanceIssueKind::DuplicateFactIdentity,
                Some("Observation"),
                Some(fact.provenance.source_resource_id.as_str()),
                "projection produced a duplicate ClinicalFact identity",
            );
        } else {
            report.facts.push(fact);
        }
    }

    if report.has_fatal_issues() {
        report.facts.clear();
    }

    report
}

fn validate_bundle_identity(entries: &[Value], report: &mut BundleConformanceReport) {
    let mut resource_identities: HashSet<(String, String)> = HashSet::new();
    let mut full_url_versions: HashSet<(String, String)> = HashSet::new();

    for entry in entries {
        let resource = entry.get("resource");
        let resource_type = resource
            .and_then(|value| value.get("resourceType"))
            .and_then(Value::as_str);
        let resource_id = resource
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty());

        if let (Some(resource_type), Some(resource_id)) = (resource_type, resource_id) {
            let identity = (resource_type.to_string(), resource_id.to_string());
            if !resource_identities.insert(identity) {
                push_fatal(
                    report,
                    ConformanceIssueKind::DuplicateResourceIdentity,
                    Some(resource_type),
                    Some(resource_id),
                    "the same logical resource identity appears more than once; strict promotion \
                     requires one unambiguous version per logical resource",
                );
            }
        }

        let Some(full_url) = entry
            .get("fullUrl")
            .and_then(Value::as_str)
            .filter(|url| !url.trim().is_empty())
        else {
            continue;
        };

        if full_url.contains("/_history/") {
            push_fatal(
                report,
                ConformanceIssueKind::VersionSpecificFullUrl,
                resource_type,
                resource_id,
                "FHIR R4 bdl-8 forbids version-specific Bundle.entry.fullUrl values",
            );
        }

        let version_id = resource
            .and_then(|value| value.get("meta"))
            .and_then(|meta| meta.get("versionId"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        if !full_url_versions.insert((full_url.to_string(), version_id.to_string())) {
            push_fatal(
                report,
                ConformanceIssueKind::DuplicateFullUrlVersion,
                resource_type,
                resource_id,
                "FHIR R4 bdl-7 requires fullUrl/version identity to be distinct in a non-history bundle",
            );
        }
    }
}

fn validate_single_patient(entries: &[Value], report: &mut BundleConformanceReport) {
    let mut patients = Vec::new();

    for entry in entries {
        let Some(resource) = entry.get("resource") else {
            continue;
        };
        if resource.get("resourceType").and_then(Value::as_str) != Some("Patient") {
            continue;
        }

        match resource
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
        {
            Some(id) => patients.push(id.to_string()),
            None => push_fatal(
                report,
                ConformanceIssueKind::InvalidPatient,
                Some("Patient"),
                None,
                "Patient.id is required by the strict single-patient promotion profile",
            ),
        }
    }

    match patients.as_slice() {
        [] => push_fatal(
            report,
            ConformanceIssueKind::MissingPatient,
            Some("Patient"),
            None,
            "strict promotion requires the bundle to establish exactly one Patient",
        ),
        [patient_id] => report.patient_id = Some(patient_id.clone()),
        _ => push_fatal(
            report,
            ConformanceIssueKind::MultiplePatients,
            Some("Patient"),
            None,
            "strict single-patient promotion refuses bundles containing multiple Patient resources",
        ),
    }
}

fn observation_is_promotable(resource: &Value, report: &mut BundleConformanceReport) -> bool {
    let resource_id = resource
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty());

    let Some(resource_id) = resource_id else {
        push_rejected(
            report,
            ConformanceIssueKind::MissingObservationId,
            Some("Observation"),
            None,
            "Observation.id is required so promoted facts have stable source identity",
        );
        return false;
    };

    let Some(status) = resource.get("status").and_then(Value::as_str) else {
        push_rejected(
            report,
            ConformanceIssueKind::MissingObservationStatus,
            Some("Observation"),
            Some(resource_id),
            "FHIR R4 Observation.status is required",
        );
        return false;
    };

    // This is a Mycelix promotion policy, not a claim that other statuses are
    // invalid FHIR. Preliminary/registered results remain useful in appropriate
    // workflows, but they must not silently enter the v1 machine-actionable fact
    // boundary that currently has no explicit Observation status field.
    if !matches!(status, "final" | "amended" | "corrected") {
        push_rejected(
            report,
            ConformanceIssueKind::NonPromotableObservationStatus,
            Some("Observation"),
            Some(resource_id),
            &format!(
                "Observation.status={status:?} is not promotable by v1; accepted promotion states are final, amended, corrected"
            ),
        );
        return false;
    }

    if resource.get("effectiveDateTime").and_then(Value::as_str).is_none() {
        let has_other_effective_form = ["effectivePeriod", "effectiveTiming", "effectiveInstant"]
            .iter()
            .any(|field| resource.get(*field).is_some());

        if has_other_effective_form {
            push_rejected(
                report,
                ConformanceIssueKind::UnsupportedEffectiveForm,
                Some("Observation"),
                Some(resource_id),
                "v1 projection supports effectiveDateTime only; other FHIR effective[x] forms need typed support before promotion",
            );
        } else {
            push_rejected(
                report,
                ConformanceIssueKind::MissingEffectiveTime,
                Some("Observation"),
                Some(resource_id),
                "strict promotion requires clinical effective time; Observation.issued is provenance/availability time and is not substituted",
            );
        }
        return false;
    }

    true
}

fn bundle_id(bundle: &Value) -> Option<&str> {
    bundle.get("id").and_then(Value::as_str)
}

fn push_fatal(
    report: &mut BundleConformanceReport,
    kind: ConformanceIssueKind,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    message: &str,
) {
    report.issues.push(ConformanceIssue {
        severity: IssueSeverity::Fatal,
        resource_type: resource_type.map(str::to_string),
        resource_id: resource_id.map(str::to_string),
        kind,
        message: message.to_string(),
    });
}

fn push_rejected(
    report: &mut BundleConformanceReport,
    kind: ConformanceIssueKind,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    message: &str,
) {
    report.issues.push(ConformanceIssue {
        severity: IssueSeverity::ResourceRejected,
        resource_type: resource_type.map(str::to_string),
        resource_id: resource_id.map(str::to_string),
        kind,
        message: message.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn patient(id: &str) -> Value {
        json!({
            "fullUrl": format!("https://ehr.example/fhir/Patient/{id}"),
            "resource": {
                "resourceType": "Patient",
                "id": id
            }
        })
    }

    fn observation(id: &str, patient_id: &str) -> Value {
        json!({
            "fullUrl": format!("https://ehr.example/fhir/Observation/{id}"),
            "resource": {
                "resourceType": "Observation",
                "id": id,
                "status": "final",
                "subject": { "reference": format!("Patient/{patient_id}") },
                "code": {
                    "coding": [{
                        "system": "http://loinc.org",
                        "code": "14771-0",
                        "display": "Glucose [Moles/volume] in Serum or Plasma"
                    }]
                },
                "effectiveDateTime": "2026-09-09T10:00:00+02:00",
                "issued": "2026-09-09T10:05:00+02:00",
                "valueQuantity": {
                    "value": 5.5,
                    "unit": "mmol/L",
                    "system": "http://unitsofmeasure.org",
                    "code": "mmol/L"
                }
            }
        })
    }

    fn bundle(entries: Vec<Value>) -> Value {
        json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": entries
        })
    }

    #[test]
    fn final_ucum_observation_promotes() {
        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), observation("obs-1", "patient-a")]),
            "https://ehr.example/fhir",
            1_700_000_000_000_000,
        );
        assert!(!report.has_fatal_issues());
        assert!(report.issues.is_empty());
        assert_eq!(report.facts.len(), 1);
    }

    #[test]
    fn entered_in_error_never_promotes() {
        let mut obs = observation("obs-1", "patient-a");
        obs["resource"]["status"] = Value::String("entered-in-error".into());
        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), obs]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.facts.is_empty());
        assert!(report.issues.iter().any(|issue| {
            issue.kind == ConformanceIssueKind::NonPromotableObservationStatus
        }));
    }

    #[test]
    fn preliminary_observation_is_kept_out_of_v1_promotion() {
        let mut obs = observation("obs-1", "patient-a");
        obs["resource"]["status"] = Value::String("preliminary".into());
        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), obs]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.facts.is_empty());
        assert!(report.issues.iter().any(|issue| {
            issue.kind == ConformanceIssueKind::NonPromotableObservationStatus
        }));
    }

    #[test]
    fn issued_is_not_substituted_for_effective_time() {
        let mut obs = observation("obs-1", "patient-a");
        obs["resource"]
            .as_object_mut()
            .unwrap()
            .remove("effectiveDateTime");
        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), obs]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.facts.is_empty());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ConformanceIssueKind::MissingEffectiveTime));
    }

    #[test]
    fn multiple_patients_fail_closed_for_entire_bundle() {
        let report = conform_and_project_observations(
            &bundle(vec![
                patient("patient-a"),
                patient("patient-b"),
                observation("obs-1", "patient-a"),
            ]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.has_fatal_issues());
        assert!(report.facts.is_empty());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ConformanceIssueKind::MultiplePatients));
    }

    #[test]
    fn duplicate_resource_identity_fails_closed() {
        let report = conform_and_project_observations(
            &bundle(vec![
                patient("patient-a"),
                observation("obs-1", "patient-a"),
                observation("obs-1", "patient-a"),
            ]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.has_fatal_issues());
        assert!(report.facts.is_empty());
        assert!(report.issues.iter().any(|issue| {
            issue.kind == ConformanceIssueKind::DuplicateResourceIdentity
        }));
    }

    #[test]
    fn bdl7_duplicate_fullurl_version_is_fatal() {
        let mut obs_a = observation("obs-1", "patient-a");
        let mut obs_b = observation("obs-2", "patient-a");
        obs_a["resource"]["meta"] = json!({"versionId": "7"});
        obs_b["resource"]["meta"] = json!({"versionId": "7"});
        obs_b["fullUrl"] = obs_a["fullUrl"].clone();

        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), obs_a, obs_b]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.has_fatal_issues());
        assert!(report.issues.iter().any(|issue| {
            issue.kind == ConformanceIssueKind::DuplicateFullUrlVersion
        }));
    }

    #[test]
    fn one_bad_observation_does_not_destroy_independent_good_fact() {
        let good = observation("obs-good", "patient-a");
        let bad = observation("obs-bad", "patient-b");
        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), good, bad]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(!report.has_fatal_issues());
        assert_eq!(report.facts.len(), 1);
        assert!(report.issues.iter().any(|issue| {
            matches!(
                issue.kind,
                ConformanceIssueKind::Projection(ProjectionIssueKind::SubjectMismatch)
            )
        }));
    }

    #[test]
    fn narrative_observation_cannot_become_machine_actionable() {
        let mut obs = observation("obs-1", "patient-a");
        obs["resource"]
            .as_object_mut()
            .unwrap()
            .remove("valueQuantity");
        obs["resource"]["valueString"] = Value::String("approximately normal".into());

        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), obs]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.facts.is_empty());
        assert!(report.issues.iter().any(|issue| {
            matches!(
                issue.kind,
                ConformanceIssueKind::Projection(ProjectionIssueKind::SemanticValidation)
            )
        }));
    }

    #[test]
    fn display_only_quantity_unit_cannot_promote() {
        let mut obs = observation("obs-1", "patient-a");
        obs["resource"]["valueQuantity"]
            .as_object_mut()
            .unwrap()
            .remove("code");

        let report = conform_and_project_observations(
            &bundle(vec![patient("patient-a"), obs]),
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.facts.is_empty());
        assert!(report.issues.iter().any(|issue| {
            matches!(
                issue.kind,
                ConformanceIssueKind::Projection(ProjectionIssueKind::SemanticValidation)
            )
        }));
    }

    #[test]
    fn history_bundle_requires_a_dedicated_handler() {
        let mut value = bundle(vec![patient("patient-a")]);
        value["type"] = Value::String("history".into());
        let report = conform_and_project_observations(
            &value,
            "https://ehr.example/fhir",
            1,
        );
        assert!(report.has_fatal_issues());
        assert!(report.issues.iter().any(|issue| {
            issue.kind == ConformanceIssueKind::UnsupportedBundleType
        }));
    }
}
