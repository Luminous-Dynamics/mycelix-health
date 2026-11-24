// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! FHIR R4 Bundle reference resolution for single-patient ingestion.
//!
//! Relative references are expanded from the referencing entry's RESTful
//! fullUrl. Version-specific references are matched against Resource.meta.versionId.
//! FHIR RESTful fullUrl type/id must agree with the embedded resource.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatientSubjectBinding {
    MatchesBundlePatient,
    DifferentPatient { referenced_patient_id: String },
    Unresolved { reason: String },
}

fn valid_fhir_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
}

/// Parse an absolute RESTful resource fullUrl into its service root, resource
/// type, and logical id. Query/fragment, encoded or invalid ids are rejected.
fn restful_resource_identity(full_url: &str) -> Option<(&str, &str, &str)> {
    if full_url.contains('?') || full_url.contains('#') {
        return None;
    }

    let scheme_len = if full_url.starts_with("https://") {
        "https://".len()
    } else if full_url.starts_with("http://") {
        "http://".len()
    } else {
        return None;
    };

    let authority_and_path = &full_url[scheme_len..];
    let path_offset = authority_and_path.find('/')?;
    let path_start = scheme_len + path_offset;
    let authority = &full_url[scheme_len..path_start];
    if authority.is_empty() {
        return None;
    }

    let id_separator = full_url.rfind('/')?;
    if id_separator < path_start || id_separator + 1 >= full_url.len() {
        return None;
    }
    let type_separator = full_url[..id_separator].rfind('/')?;
    if type_separator < path_start {
        return None;
    }

    let resource_type = &full_url[type_separator + 1..id_separator];
    let id = &full_url[id_separator + 1..];
    if resource_type.is_empty()
        || !resource_type.bytes().all(|byte| byte.is_ascii_alphabetic())
        || !resource_type.as_bytes()[0].is_ascii_uppercase()
        || !valid_fhir_id(id)
    {
        return None;
    }

    Some((&full_url[..type_separator + 1], resource_type, id))
}

/// A Bundle entry with an HTTP(S) fullUrl must agree with the resource's
/// resourceType and id. URN fullUrls are opaque identifiers and are resolved
/// only by exact equality.
pub fn bundle_entry_full_url_matches_resource(entry: &Value) -> bool {
    let Some(full_url) = entry.get("fullUrl").and_then(Value::as_str) else {
        return true;
    };

    if full_url.starts_with("urn:") {
        return true;
    }

    let Some((_, full_url_type, full_url_id)) = restful_resource_identity(full_url) else {
        return false;
    };
    let Some(resource) = entry.get("resource") else {
        return false;
    };
    let resource_type = resource
        .get("resourceType")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let resource_id = resource
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default();

    full_url_type == resource_type && full_url_id == resource_id
}

fn source_restful_root(source_entry: Option<&Value>) -> Option<&str> {
    let entry = source_entry?;
    let full_url = entry.get("fullUrl").and_then(Value::as_str)?;
    let (root, full_url_type, full_url_id) = restful_resource_identity(full_url)?;
    let resource = entry.get("resource")?;
    let resource_type = resource.get("resourceType").and_then(Value::as_str)?;
    let resource_id = resource.get("id").and_then(Value::as_str)?;

    if full_url_type == resource_type && full_url_id == resource_id {
        Some(root)
    } else {
        None
    }
}

fn parse_relative_patient_reference(reference: &str) -> Option<(&str, Option<&str>)> {
    let parts: Vec<&str> = reference.split('/').collect();
    match parts.as_slice() {
        ["Patient", id] if valid_fhir_id(id) => Some((*id, None)),
        ["Patient", id, "_history", version] if valid_fhir_id(id) && valid_fhir_id(version) => {
            Some((*id, Some(*version)))
        }
        _ => None,
    }
}

fn split_absolute_history(reference: &str) -> Option<(&str, Option<&str>)> {
    if let Some((base, version)) = reference.rsplit_once("/_history/") {
        if base.is_empty() || !valid_fhir_id(version) || version.contains('/') {
            return None;
        }
        return Some((base, Some(version)));
    }
    if reference.contains("/_history") {
        return None;
    }
    Some((reference, None))
}

/// Resolve a patient-scoped reference using Bundle reference identity semantics.
///
/// - Relative Patient references are expanded from the referencing entry's
///   RESTful fullUrl, which must agree with that entry's resource identity.
/// - Absolute references match Bundle entry fullUrl exactly.
/// - URNs match Bundle entry fullUrl exactly.
/// - Version-specific references additionally require exact meta.versionId.
/// - For RESTful targets, fullUrl resource type/id must agree with the target.
/// - Exactly one Patient target must resolve to the selected patient.
pub fn resolve_patient_subject(
    reference: Option<&str>,
    source_entry: Option<&Value>,
    bundle_patient_id: &str,
    entries: &[Value],
) -> PatientSubjectBinding {
    let Some(reference) = reference.map(str::trim).filter(|value| !value.is_empty()) else {
        return PatientSubjectBinding::Unresolved {
            reason: "missing patient/subject reference".to_string(),
        };
    };

    let (target_full_url, requested_version) = if let Some((patient_id, version)) =
        parse_relative_patient_reference(reference)
    {
        let Some(root) = source_restful_root(source_entry) else {
            return PatientSubjectBinding::Unresolved {
                reason: "relative reference has no consistent RESTful source fullUrl context"
                    .to_string(),
            };
        };
        (format!("{root}Patient/{patient_id}"), version)
    } else if reference.starts_with("urn:") {
        if reference.contains("/_history") {
            return PatientSubjectBinding::Unresolved {
                reason: "URN patient references cannot carry a FHIR history version".to_string(),
            };
        }
        (reference.to_string(), None)
    } else if reference.starts_with("https://") || reference.starts_with("http://") {
        let Some((base, version)) = split_absolute_history(reference) else {
            return PatientSubjectBinding::Unresolved {
                reason: "malformed version-specific absolute reference".to_string(),
            };
        };
        (base.to_string(), version)
    } else {
        return PatientSubjectBinding::Unresolved {
            reason: "unsupported or malformed patient reference form".to_string(),
        };
    };

    resolve_bundle_target(
        &target_full_url,
        requested_version,
        bundle_patient_id,
        entries,
    )
}

fn resolve_bundle_target(
    target_full_url: &str,
    requested_version: Option<&str>,
    bundle_patient_id: &str,
    entries: &[Value],
) -> PatientSubjectBinding {
    let exact_matches: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry.get("fullUrl").and_then(Value::as_str) == Some(target_full_url))
        .collect();

    if exact_matches.is_empty() {
        return PatientSubjectBinding::Unresolved {
            reason: "reference does not resolve to a Bundle entry fullUrl".to_string(),
        };
    }

    let version_matches: Vec<&Value> = match requested_version {
        Some(version) => exact_matches
            .into_iter()
            .filter(|entry| {
                entry
                    .get("resource")
                    .and_then(|resource| resource.get("meta"))
                    .and_then(|meta| meta.get("versionId"))
                    .and_then(Value::as_str)
                    == Some(version)
            })
            .collect(),
        None => exact_matches,
    };

    match version_matches.as_slice() {
        [] => PatientSubjectBinding::Unresolved {
            reason: "Bundle fullUrl exists but the requested meta.versionId does not".to_string(),
        },
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
                    reason: format!(
                        "matching Bundle fullUrl resolves to {resource_type}, not Patient"
                    ),
                };
            }
            let Some(referenced_id) = resource.get("id").and_then(Value::as_str) else {
                return PatientSubjectBinding::Unresolved {
                    reason: "matching Patient fullUrl has no logical id".to_string(),
                };
            };
            if target_full_url.starts_with("https://") || target_full_url.starts_with("http://") {
                let Some((_, full_url_type, full_url_id)) =
                    restful_resource_identity(target_full_url)
                else {
                    return PatientSubjectBinding::Unresolved {
                        reason: "target RESTful fullUrl is malformed".to_string(),
                    };
                };
                if full_url_type != "Patient" || full_url_id != referenced_id {
                    return PatientSubjectBinding::Unresolved {
                        reason: "Patient fullUrl type/id disagrees with embedded resource"
                            .to_string(),
                    };
                }
            }
            if referenced_id == bundle_patient_id {
                PatientSubjectBinding::MatchesBundlePatient
            } else {
                PatientSubjectBinding::DifferentPatient {
                    referenced_patient_id: referenced_id.to_string(),
                }
            }
        }
        _ => PatientSubjectBinding::Unresolved {
            reason: "reference resolves to multiple Bundle entries".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SOURCE_FULL_URL: &str = "https://ehr.example/fhir/Observation/o-1";
    const PATIENT_FULL_URL: &str = "https://ehr.example/fhir/Patient/p-1";

    fn patient_entry(full_url: &str, id: &str, version_id: Option<&str>) -> Value {
        let mut resource = json!({
            "resourceType": "Patient",
            "id": id
        });
        if let Some(version_id) = version_id {
            resource["meta"] = json!({ "versionId": version_id });
        }
        json!({ "fullUrl": full_url, "resource": resource })
    }

    fn source_entry(full_url: &str, id: &str) -> Value {
        json!({
            "fullUrl": full_url,
            "resource": {
                "resourceType": "Observation",
                "id": id
            }
        })
    }

    #[test]
    fn accepts_relative_reference_expanded_from_consistent_restful_source_url() {
        let entries = vec![patient_entry(PATIENT_FULL_URL, "p-1", None)];
        let source = source_entry(SOURCE_FULL_URL, "o-1");
        assert_eq!(
            resolve_patient_subject(Some("Patient/p-1"), Some(&source), "p-1", &entries),
            PatientSubjectBinding::MatchesBundlePatient
        );
    }

    #[test]
    fn rejects_relative_reference_without_a_consistent_restful_source_url() {
        for source in [
            Some(source_entry(
                "urn:uuid:00000000-0000-4000-8000-000000000002",
                "o-1",
            )),
            Some(source_entry(
                "https://ehr.example/fhir/Observation/other",
                "o-1",
            )),
        ] {
            assert!(matches!(
                resolve_patient_subject(Some("Patient/p-1"), source.as_ref(), "p-1", &[]),
                PatientSubjectBinding::Unresolved { .. }
            ));
        }
        assert!(matches!(
            resolve_patient_subject(Some("Patient/p-1"), None, "p-1", &[]),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn rejects_relative_reference_when_expanded_url_does_not_match_patient_full_url() {
        let entries = vec![patient_entry(
            "https://other-ehr.example/fhir/Patient/p-1",
            "p-1",
            None,
        )];
        let source = source_entry(SOURCE_FULL_URL, "o-1");
        assert!(matches!(
            resolve_patient_subject(Some("Patient/p-1"), Some(&source), "p-1", &entries),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn matches_versioned_relative_reference_against_meta_version_id() {
        let entries = vec![patient_entry(PATIENT_FULL_URL, "p-1", Some("3"))];
        let source = source_entry(SOURCE_FULL_URL, "o-1");
        assert_eq!(
            resolve_patient_subject(
                Some("Patient/p-1/_history/3"),
                Some(&source),
                "p-1",
                &entries
            ),
            PatientSubjectBinding::MatchesBundlePatient
        );
        assert!(matches!(
            resolve_patient_subject(
                Some("Patient/p-1/_history/4"),
                Some(&source),
                "p-1",
                &entries
            ),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn resolves_absolute_versioned_reference_against_exact_meta_version_id() {
        let versioned = format!("{PATIENT_FULL_URL}/_history/9");
        let entries = vec![patient_entry(PATIENT_FULL_URL, "p-1", Some("9"))];
        assert_eq!(
            resolve_patient_subject(Some(&versioned), None, "p-1", &entries),
            PatientSubjectBinding::MatchesBundlePatient
        );
    }

    #[test]
    fn rejects_absolute_reference_with_wrong_version_id() {
        let versioned = format!("{PATIENT_FULL_URL}/_history/8");
        let entries = vec![patient_entry(PATIENT_FULL_URL, "p-1", Some("9"))];
        assert!(matches!(
            resolve_patient_subject(Some(&versioned), None, "p-1", &entries),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn accepts_absolute_url_and_urn_only_on_exact_full_url_match() {
        let urn = "urn:uuid:00000000-0000-4000-8000-000000000001";
        for (reference, full_url) in [(PATIENT_FULL_URL, PATIENT_FULL_URL), (urn, urn)] {
            let entries = vec![patient_entry(full_url, "p-1", None)];
            assert_eq!(
                resolve_patient_subject(Some(reference), None, "p-1", &entries),
                PatientSubjectBinding::MatchesBundlePatient
            );
        }
    }

    #[test]
    fn rejects_missing_or_external_reference() {
        assert!(matches!(
            resolve_patient_subject(None, None, "p-1", &[]),
            PatientSubjectBinding::Unresolved { .. }
        ));
        assert!(matches!(
            resolve_patient_subject(
                Some("https://other-ehr.example/fhir/Patient/p-1"),
                None,
                "p-1",
                &[]
            ),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn rejects_non_patient_target_and_duplicate_unversioned_target() {
        let non_patient = vec![json!({
            "fullUrl": "urn:uuid:00000000-0000-4000-8000-000000000002",
            "resource": { "resourceType": "Organization", "id": "org-1" }
        })];
        assert!(matches!(
            resolve_patient_subject(
                Some("urn:uuid:00000000-0000-4000-8000-000000000002"),
                None,
                "p-1",
                &non_patient
            ),
            PatientSubjectBinding::Unresolved { .. }
        ));

        let duplicates = vec![
            patient_entry(PATIENT_FULL_URL, "p-1", Some("8")),
            patient_entry(PATIENT_FULL_URL, "p-1", Some("9")),
        ];
        assert!(matches!(
            resolve_patient_subject(Some(PATIENT_FULL_URL), None, "p-1", &duplicates),
            PatientSubjectBinding::Unresolved { .. }
        ));
        let versioned = format!("{PATIENT_FULL_URL}/_history/9");
        assert_eq!(
            resolve_patient_subject(Some(&versioned), None, "p-1", &duplicates),
            PatientSubjectBinding::MatchesBundlePatient
        );
    }

    #[test]
    fn rejects_target_full_url_that_disagrees_with_patient_resource_id() {
        let entries = vec![patient_entry(
            "https://ehr.example/fhir/Patient/p-wrong",
            "p-1",
            None,
        )];
        assert!(matches!(
            resolve_patient_subject(
                Some("https://ehr.example/fhir/Patient/p-wrong"),
                None,
                "p-1",
                &entries
            ),
            PatientSubjectBinding::Unresolved { .. }
        ));
    }

    #[test]
    fn rejects_inconsistent_source_or_patient_bundle_entry_identity() {
        let source = source_entry("https://ehr.example/fhir/Observation/o-spoofed", "o-1");
        assert!(!bundle_entry_full_url_matches_resource(&source));

        let patient = patient_entry("https://ehr.example/fhir/Patient/p-spoofed", "p-1", None);
        assert!(!bundle_entry_full_url_matches_resource(&patient));
    }

    #[test]
    fn identifies_a_different_patient_even_when_reference_is_well_formed() {
        let entries = vec![patient_entry(
            "https://ehr.example/fhir/Patient/p-2",
            "p-2",
            None,
        )];
        let source = source_entry(SOURCE_FULL_URL, "o-1");
        assert_eq!(
            resolve_patient_subject(Some("Patient/p-2"), Some(&source), "p-1", &entries),
            PatientSubjectBinding::DifferentPatient {
                referenced_patient_id: "p-2".to_string()
            }
        );
    }

    #[test]
    fn fhir_subject_binding_v1_corpus() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../benchmarks/clinical-evidence-integrity/fhir-subject-binding-v1.json"
        ))
        .expect("checked-in synthetic benchmark corpus is valid JSON");

        assert_eq!(corpus["schema_version"].as_u64(), Some(1));
        assert_eq!(
            corpus["benchmark_id"].as_str(),
            Some("mycelix-clinical-evidence-integrity.fhir-subject-binding.v1")
        );
        assert_eq!(
            corpus["data_classification"].as_str(),
            Some("fully_synthetic_no_phi")
        );

        let cases = corpus["cases"]
            .as_array()
            .expect("corpus contains a cases array");
        assert!(
            cases.len() >= 15,
            "benchmark corpus is intentionally adversarial"
        );

        let mut accepted = 0usize;
        let mut rejected = 0usize;
        for case in cases {
            let case_id = case["id"].as_str().expect("case id");
            let reference = case["reference"].as_str();
            let source_entry = &case["source_entry"];
            let selected_patient_id = case["selected_patient_id"]
                .as_str()
                .expect("selected patient id");
            let entries = case["entries"].as_array().expect("bundle entries");

            let patient_entries: Vec<&Value> = entries
                .iter()
                .filter(|entry| {
                    entry
                        .get("resource")
                        .and_then(|resource| resource.get("resourceType"))
                        .and_then(Value::as_str)
                        == Some("Patient")
                })
                .collect();

            let patient_entries_valid = patient_entries
                .iter()
                .all(|entry| bundle_entry_full_url_matches_resource(entry));
            let source_entry_valid = bundle_entry_full_url_matches_resource(source_entry);
            let binding = resolve_patient_subject(
                reference,
                Some(source_entry),
                selected_patient_id,
                entries,
            );
            let binding_label = match &binding {
                PatientSubjectBinding::MatchesBundlePatient => "matches",
                PatientSubjectBinding::DifferentPatient { .. } => "different_patient",
                PatientSubjectBinding::Unresolved { .. } => "unresolved",
            };

            assert_eq!(
                binding_label,
                case["expected_binding"].as_str().expect("expected binding"),
                "binding mismatch for case {case_id}"
            );

            // Mirror the coordinator's pre-ingest single-patient gates:
            // exactly one Patient, valid selected/source Bundle identities,
            // and an unambiguous subject match.
            let accepted_by_policy = patient_entries.len() == 1
                && patient_entries_valid
                && source_entry_valid
                && binding == PatientSubjectBinding::MatchesBundlePatient;

            let actual_ingest = if accepted_by_policy {
                "accept"
            } else {
                "reject"
            };
            assert_eq!(
                actual_ingest,
                case["expected_ingest"]
                    .as_str()
                    .expect("expected ingest outcome"),
                "ingest policy mismatch for case {case_id}"
            );

            if accepted_by_policy {
                accepted += 1;
            } else {
                rejected += 1;
            }
        }

        assert!(accepted >= 4, "positive controls must exercise valid paths");
        assert!(
            rejected >= 10,
            "negative controls must exercise fail-closed paths"
        );
        assert_eq!(accepted + rejected, cases.len());
    }
}
