// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Strict conversion of FHIR clinical instants into Holochain timestamps.
//!
//! The current mapping profile supports Observation.effectiveDateTime and the
//! independent Observation.issued instant. Both must be timezone-qualified
//! RFC3339 values and exactly representable at microsecond precision. Unsupported
//! effective[x] choices and lossy sub-microsecond values are rejected.

use chrono::DateTime;
use hdk::prelude::Timestamp;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationTemporalEvidence {
    /// Clinical event time from Observation.effectiveDateTime.
    pub effective_datetime: Timestamp,
    /// Independently supplied issue time; never inferred from event or import time.
    pub issued: Option<Timestamp>,
}

/// Parse an RFC3339 instant without changing the represented instant or
/// silently truncating precision that the Holochain timestamp cannot retain.
pub fn parse_fhir_instant(value: &str, field: &str) -> Result<Timestamp, String> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|error| {
        format!("{field}: expected a timezone-qualified RFC3339 instant, got {value:?}: {error}")
    })?;

    if parsed.timestamp_subsec_nanos() % 1_000 != 0 {
        return Err(format!(
            "{field}: sub-microsecond precision cannot be represented without loss"
        ));
    }

    Ok(Timestamp::from_micros(parsed.timestamp_micros()))
}

/// Parse Observation event and issue times as separate facts.
///
/// This profile intentionally supports only effectiveDateTime. FHIR's other
/// effective[x] choices (Period, Timing and instant) require a versioned
/// interval/precision envelope and are not collapsed into a point timestamp.
pub fn parse_observation_temporal_evidence(
    resource: &Value,
) -> Result<ObservationTemporalEvidence, String> {
    let effective_choices = [
        "effectiveDateTime",
        "effectivePeriod",
        "effectiveTiming",
        "effectiveInstant",
    ];
    let present_choices: Vec<&str> = effective_choices
        .iter()
        .copied()
        .filter(|field| resource.get(*field).is_some())
        .collect();

    if present_choices.is_empty() {
        return Err(
            "Observation has no effective[x] value; a supported effectiveDateTime is required"
                .to_string(),
        );
    }
    if present_choices.len() != 1 {
        return Err(format!(
            "Observation has multiple effective[x] choices: {}",
            present_choices.join(", ")
        ));
    }
    if present_choices[0] != "effectiveDateTime" {
        return Err(format!(
            "Observation.{} is not supported by this mapping version",
            present_choices[0]
        ));
    }

    let effective_raw = resource
        .get("effectiveDateTime")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "Observation.effectiveDateTime must be a string containing a timezone-qualified instant"
                .to_string()
        })?;
    let effective_datetime = parse_fhir_instant(effective_raw, "Observation.effectiveDateTime")?;

    let issued = match resource.get("issued") {
        None => None,
        Some(Value::String(value)) => {
            Some(parse_fhir_instant(value, "Observation.issued")?)
        }
        Some(_) => {
            return Err("Observation.issued must be a string when present".to_string());
        }
    };

    Ok(ObservationTemporalEvidence {
        effective_datetime,
        issued,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_utc_rfc3339_instant() {
        let parsed = parse_fhir_instant("2025-01-10T08:20:00Z", "effectiveDateTime")
            .expect("valid UTC instant");
        assert_eq!(parsed.as_micros(), 1_736_497_200_000_000);
    }

    #[test]
    fn normalizes_offset_without_changing_the_instant() {
        let utc = parse_fhir_instant("2025-01-10T08:20:00Z", "effectiveDateTime")
            .expect("valid UTC instant");
        let offset = parse_fhir_instant("2025-01-10T10:20:00+02:00", "effectiveDateTime")
            .expect("valid offset instant");
        assert_eq!(utc, offset);
    }

    #[test]
    fn rejects_date_only_precision_in_the_instant_only_profile() {
        let result = parse_fhir_instant("2025-01-10", "effectiveDateTime");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("timezone-qualified RFC3339 instant"));
    }

    #[test]
    fn rejects_wall_clock_time_without_timezone() {
        let result = parse_fhir_instant("2025-01-10T08:20:00", "effectiveDateTime");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unrepresentable_sub_microsecond_precision() {
        let result = parse_fhir_instant(
            "2025-01-10T08:20:00.123456789Z",
            "Observation.effectiveDateTime",
        );
        assert!(result
            .unwrap_err()
            .contains("sub-microsecond precision"));
    }

    #[test]
    fn parses_effective_and_issued_times_independently() {
        let resource = json!({
            "effectiveDateTime": "2025-01-10T08:20:00Z",
            "issued": "2025-01-10T10:30:00+02:00"
        });
        let parsed =
            parse_observation_temporal_evidence(&resource).expect("valid temporal evidence");
        assert_eq!(parsed.effective_datetime.as_micros(), 1_736_497_200_000_000);
        assert_eq!(parsed.issued.unwrap().as_micros(), 1_736_497_800_000_000);
    }

    #[test]
    fn observation_temporal_fidelity_v1_corpus() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../benchmarks/clinical-evidence-integrity/observation-temporal-fidelity-v1.json"
        ))
        .expect("checked-in synthetic temporal corpus is valid JSON");
        assert_eq!(corpus["schema_version"].as_u64(), Some(1));
        assert_eq!(
            corpus["benchmark_id"].as_str(),
            Some("mycelix-clinical-evidence-integrity.observation-temporal-fidelity.v1")
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
            let resource = &case["resource"];
            match case["expected"].as_str().expect("expected result") {
                "accept" => {
                    let parsed = parse_observation_temporal_evidence(resource)
                        .unwrap_or_else(|error| panic!("case {id} should accept: {error}"));
                    assert_eq!(
                        parsed.effective_datetime.as_micros(),
                        case["expected_effective_micros"]
                            .as_i64()
                            .expect("expected event micros"),
                        "effective time mismatch for {id}"
                    );
                    let expected_issued = case["expected_issued_micros"].as_i64();
                    assert_eq!(
                        parsed.issued.map(|timestamp| timestamp.as_micros()),
                        expected_issued,
                        "issued time mismatch for {id}"
                    );
                    accepted += 1;
                }
                "reject" => {
                    let error = parse_observation_temporal_evidence(resource)
                        .expect_err(&format!("case {id} must reject"));
                    if let Some(expected_fragment) = case["expected_error_contains"].as_str() {
                        assert!(
                            error.contains(expected_fragment),
                            "case {id}: expected error containing {expected_fragment:?}, got {error:?}"
                        );
                    }
                    rejected += 1;
                }
                other => panic!("case {id} has unknown expected result {other:?}"),
            }
        }

        assert!(accepted >= 5, "positive temporal controls must be exercised");
        assert!(rejected >= 8, "negative temporal controls must be exercised");
        assert_eq!(accepted + rejected, cases.len());
    }
}
