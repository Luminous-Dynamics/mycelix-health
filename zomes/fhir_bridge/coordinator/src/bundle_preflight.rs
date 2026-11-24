// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Fail-closed FHIR R4 Bundle envelope preflight for the current ingestion profile.
//!
//! This endpoint deliberately supports only Bundle.type = collection.
//! Other Bundle types have different entry request/response/search constraints
//! and require separately versioned ingestion profiles.

use crate::patient_subject::bundle_entry_full_url_matches_resource;
use serde_json::Value;
use std::collections::BTreeSet;

/// Validate the complete supported Bundle envelope before any patient or
/// clinical resource is written.
///
/// The current profile requires every resource-bearing entry to have a
/// resourceType, persistent resource ID and unique absolute HTTP(S) or opaque
/// URN fullUrl. This is a deliberately narrow collection-profile validator,
/// not a complete FHIR R4 Bundle validator.
pub fn validate_collection_bundle(bundle: &Value) -> Result<Vec<Value>, Vec<String>> {
    let mut errors = Vec::new();

    if bundle.get("resourceType").and_then(Value::as_str) != Some("Bundle") {
        errors.push("Bundle.resourceType must be exactly Bundle".to_string());
    }

    if bundle.get("type").and_then(Value::as_str) != Some("collection") {
        errors.push(
            "Only Bundle.type = collection is supported by this ingestion profile".to_string(),
        );
    }

    let Some(entries) = bundle.get("entry").and_then(Value::as_array) else {
        errors.push("Bundle.entry must be an array".to_string());
        return Err(errors);
    };

    if entries.is_empty() {
        errors.push("Bundle.entry must contain at least one resource entry".to_string());
    }

    let mut seen_full_urls = BTreeSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let prefix = format!("Bundle.entry[{index}]");

        if !entry.is_object() {
            errors.push(format!("{prefix} must be an object"));
            continue;
        }

        for forbidden in ["request", "response", "search"] {
            if entry.get(forbidden).is_some() {
                errors.push(format!(
                    "{prefix}.{forbidden} is not allowed in the collection ingestion profile"
                ));
            }
        }

        let Some(resource) = entry.get("resource").filter(|value| value.is_object()) else {
            errors.push(format!("{prefix}.resource must be an object"));
            continue;
        };

        let Some(resource_type) = resource
            .get("resourceType")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            errors.push(format!("{prefix}.resource.resourceType is required"));
            continue;
        };

        let Some(resource_id) = resource
            .get("id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            errors.push(format!(
                "{prefix}.resource.id is required by this ingestion profile"
            ));
            continue;
        };

        let Some(full_url) = entry
            .get("fullUrl")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            errors.push(format!("{prefix}.fullUrl is required"));
            continue;
        };

        if !seen_full_urls.insert(full_url) {
            errors.push(format!("{prefix}.fullUrl duplicates another Bundle entry"));
        }

        if full_url.starts_with("urn:") {
            if full_url.len() <= 4 || full_url.chars().any(char::is_whitespace) {
                errors.push(format!("{prefix}.fullUrl is not a valid opaque URN"));
            }
        } else if full_url.starts_with("https://") || full_url.starts_with("http://") {
            if !bundle_entry_full_url_matches_resource(entry) {
                errors.push(format!(
                    "{prefix}.fullUrl type/id must agree with resourceType={resource_type}, id={resource_id}; malformed or version-specific fullUrls are unsupported"
                ));
            }
        } else {
            errors.push(format!(
                "{prefix}.fullUrl must be an absolute HTTP(S) URL or opaque URN"
            ));
        }
    }

    if errors.is_empty() {
        Ok(entries.clone())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fhir_bundle_preflight_v1_corpus() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../benchmarks/clinical-evidence-integrity/fhir-bundle-preflight-v1.json"
        ))
        .expect("checked-in Bundle preflight corpus is valid JSON");

        assert_eq!(corpus["schema_version"].as_u64(), Some(1));
        assert_eq!(
            corpus["benchmark_id"].as_str(),
            Some("mycelix-clinical-evidence-integrity.fhir-bundle-preflight.v1")
        );
        assert_eq!(
            corpus["data_classification"].as_str(),
            Some("fully_synthetic_no_phi")
        );

        let cases = corpus["cases"].as_array().expect("corpus has cases");
        assert!(cases.len() >= 15);
        let mut accepted = 0usize;
        let mut rejected = 0usize;

        for case in cases {
            let id = case["id"].as_str().expect("case id");
            match case["expected"].as_str().expect("expected result") {
                "accept" => {
                    let entries = validate_collection_bundle(&case["bundle"])
                        .unwrap_or_else(|errors| panic!("case {id} should accept: {errors:?}"));
                    assert_eq!(
                        entries.len(),
                        case["expected_entry_count"].as_u64().unwrap() as usize,
                        "entry count mismatch for {id}"
                    );
                    accepted += 1;
                }
                "reject" => {
                    let errors = validate_collection_bundle(&case["bundle"])
                        .expect_err(&format!("case {id} must reject"));
                    let combined = errors.join(" | ");
                    if let Some(fragment) = case["expected_error_contains"].as_str() {
                        assert!(
                            combined.contains(fragment),
                            "case {id}: expected error containing {fragment:?}, got {combined:?}"
                        );
                    }
                    rejected += 1;
                }
                other => panic!("case {id} has unknown expected result {other:?}"),
            }
        }

        assert!(accepted >= 2, "positive Bundle controls must be exercised");
        assert!(rejected >= 12, "negative Bundle controls must be exercised");
        assert_eq!(accepted + rejected, cases.len());
    }

    #[test]
    fn rejects_collection_entry_without_full_url() {
        let bundle = json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [{ "resource": { "resourceType": "Patient", "id": "patient-1" } }]
        });
        let errors = validate_collection_bundle(&bundle).expect_err("missing fullUrl rejects");
        assert!(errors
            .iter()
            .any(|error| error.contains("fullUrl is required")));
    }

    #[test]
    fn accepts_opaque_urn_and_consistent_restful_full_url() {
        let bundle = json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [
                {
                    "fullUrl": "urn:uuid:patient-1",
                    "resource": { "resourceType": "Patient", "id": "patient-1" }
                },
                {
                    "fullUrl": "https://ehr.example/fhir/Observation/obs-1",
                    "resource": { "resourceType": "Observation", "id": "obs-1" }
                }
            ]
        });
        assert_eq!(validate_collection_bundle(&bundle).unwrap().len(), 2);
    }
}
