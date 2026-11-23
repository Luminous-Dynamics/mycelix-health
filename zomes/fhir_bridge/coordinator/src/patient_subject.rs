// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Patient-subject binding for single-patient FHIR Bundle ingestion.
//!
//! A patient-scoped resource is accepted only when its reference resolves to
//! the one patient selected for this ingest operation. Missing, ambiguous or
//! unresolved references are not inferred from Bundle proximity.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatientSubjectBinding {
    MatchesBundlePatient,
    DifferentPatient { referenced_patient_id: String },
    Unresolved { reason: String },
}

/// Extract a supported relative Patient reference ID.
///
/// Versioned references are accepted only when they preserve the same Patient
/// logical ID: Patient/{id}/_history/{version}. Absolute references are not
/// parsed by URL suffix because that could confuse two servers with the same
/// logical ID; they must resolve through an exact Bundle fullUrl instead.
pub fn relative_patient_id(reference: &str) -> Option<&str> {
    let parts: Vec<&str> = reference.split('/').collect();
    if parts.len() == 2 && parts[0] == "Patient" && !parts[1].is_empty() {
        return Some(parts[1]);
    }
    if parts.len() == 4
        && parts[0] == "Patient"
        && !parts[1].is_empty()
        && parts[2] == "_history"
        && !parts[3].is_empty()
    {
        return Some(parts[1]);
    }
    None
}

/// Resolve a patient reference against the one patient selected for this Bundle.
///
/// Supported forms:
/// - Relative Patient/{id}, or Patient/{id}/_history/{version}, matched by id.
/// - Absolute or urn:uuid reference that exactly matches one Bundle entry fullUrl,
///   whose resource is the selected Patient.
///
/// All other forms fail closed.
pub fn resolve_patient_subject(
    reference: Option<&str>,
    bundle_patient_id: &str,
    entries: &[Value],
) -> PatientSubjectBinding {
    let Some(reference) = reference.map(str::trim).filter(|value| !value.is_empty()) else {
        return PatientSubjectBinding::Unresolved {
            reason: "missing patient/subject reference".to_string(),
        };
    };

    if let Some(referenced_id) = relative_patient_id(reference) {
        return if referenced_id == bundle_patient_id {
            PatientSubjectBinding::MatchesBundlePatient
        } else {
            PatientSubjectBinding::DifferentPatient {
                referenced_patient_id: referenced_id.to_string(),
            }
        };
    }

    let matching_entries: Vec<&Value> = entries
        .iter()
        .filter(|entry| {
            entry.get("fullUrl").and_then(Value::as_str) == Some(reference)
        })
        .collect();

    match matching_entries.as_slice() {
        [entry] => {
            let Some(resource) = entry.get("resource") else {
                return PatientSubjectBinding::Unresolved {
                    reason: "matching Bundle fullUrl has no resource".to_string(),
                };
            };
            let resource_type = resource
                .get("resourceType")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if resource_type != "Patient" {
                return PatientSubjectBinding::Unresolved {
                    reason: format!("matching Bundle fullUrl resolves to {resource_type}, not Patient"),
                };
            }
            let Some(referenced_id) = resource.get("id").and_then(Value::as_str) else {
                return PatientSubjectBinding::Unresolved {
                    reason: "matching Patient fullUrl has no logical id".to_string(),
                };
            };
            if referenced_id == bundle_patient_id {
                PatientSubjectBinding::MatchesBundlePatient
            } else {
                PatientSubjectBinding::DifferentPatient {
                    referenced_patient_id: referenced_id.to_string(),
                }
            }
        }
        [] => PatientSubjectBinding::Unresolved {
            reason: "reference is neither a supported relative Patient reference nor an exact Bundle fullUrl"
                .to_string(),
        },
        _ => PatientSubjectBinding::Unresolved {
            reason: "reference matches multiple Bundle fullUrl entries".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn patient_entry(full_url: &str, id: &str) -> Value {
        json!({
            "fullUrl": full_url,
            "resource": {
                "resourceType": "Patient",
                "id": id
            }
        })
    }

    #[test]
    fn accepts_matching_relative_patient_reference() {
        assert_eq!(
            resolve_patient_subject(
                Some("Patient/p-1"),
                "p-1",
                &[]
            ),
            PatientSubjectBinding::MatchesBundlePatient
        );
    }

    #[test]
    fn accepts_matching_versioned_relative_patient_reference() {
        assert_eq!(
            resolve_patient_subject(
                Some("Patient/p-1/_history/3"),
                "p-1",
                &[]
            ),
            PatientSubjectBinding::MatchesBundlePatient
        );
    }

    #[test]
    fn rejects_different_relative_patient_reference() {
        assert_eq!(
            resolve_patient_subject(
                Some("Patient/p-2"),
                "p-1",
                &[]
            ),
            PatientSubjectBinding::DifferentPatient {
                referenced_patient_id: "p-2".to_string()
            }
        );
    }

    #[test]
    fn accepts_urn_reference_only_when_it_resolves_to_the_selected_bundle_patient() {
        let entries = vec![patient_entry("urn:uuid:00000000-0000-4000-8000-000000000001", "p-1")];
        assert_eq!(
            resolve_patient_subject(
                Some("urn:uuid:00000000-0000-4000-8000-000000000001"),
                "p-1",
                &entries
            ),
            PatientSubjectBinding::MatchesBundlePatient
        );
    }

    #[test]
    fn accepts_absolute_reference_only_when_it_exactly_matches_bundle_full_url() {
        let full_url = "https://ehr.example/fhir/Patient/p-1";
        let entries = vec![patient_entry(full_url, "p-1")];
        assert_eq!(
            resolve_patient_subject(Some(full_url), "p-1", &entries),
            PatientSubjectBinding::MatchesBundlePatient
        );
    }

    #[test]
    fn rejects_absolute_reference_not_present_in_bundle_even_if_it_ends_with_selected_id() {
        assert!(matches!(
            resolve_patient_subject(
                Some("https://other-ehr.example/fhir/Patient/p-1"),
                "p-1",
                &[]
            ),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn rejects_missing_patient_reference() {
        assert!(matches!(
            resolve_patient_subject(None, "p-1", &[]),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn rejects_reference_to_non_patient_bundle_entry() {
        let entries = vec![json!({
            "fullUrl": "urn:uuid:00000000-0000-4000-8000-000000000002",
            "resource": { "resourceType": "Organization", "id": "org-1" }
        })];
        assert!(matches!(
            resolve_patient_subject(
                Some("urn:uuid:00000000-0000-4000-8000-000000000002"),
                "p-1",
                &entries
            ),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_bundle_full_url_matches() {
        let full_url = "urn:uuid:00000000-0000-4000-8000-000000000003";
        let entries = vec![
            patient_entry(full_url, "p-1"),
            patient_entry(full_url, "p-1")
        ];
        assert!(matches!(
            resolve_patient_subject(Some(full_url), "p-1", &entries),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }
}
