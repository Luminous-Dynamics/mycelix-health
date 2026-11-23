// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Strict conversion of FHIR instant values into Holochain timestamps.
//!
//! This helper accepts timezone-qualified RFC3339 instants only. It does not
//! coerce date-only values or local wall-clock times into invented instants.

use chrono::DateTime;
use hdk::prelude::Timestamp;

/// Parse a timezone-qualified RFC3339 instant without changing the represented instant.
pub fn parse_fhir_instant(value: &str, field: &str) -> Result<Timestamp, String> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|error| {
        format!(
            "{field}: expected a timezone-qualified RFC3339 instant, got {value:?}: {error}"
        )
    })?;

    Ok(Timestamp::from_micros(parsed.timestamp_micros()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(result.unwrap_err().contains("timezone-qualified RFC3339 instant"));
    }

    #[test]
    fn rejects_wall_clock_time_without_timezone() {
        let result = parse_fhir_instant("2025-01-10T08:20:00", "effectiveDateTime");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_malformed_instants_and_names_the_source_field() {
        let result = parse_fhir_instant("not-a-date", "Observation.issued");
        assert!(result.unwrap_err().contains("Observation.issued"));
    }
}
