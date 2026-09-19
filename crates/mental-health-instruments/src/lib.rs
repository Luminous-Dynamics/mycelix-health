#![forbid(unsafe_code)]
//! Fail-closed reference semantics for mental-health screening instruments.
//!
//! This crate deliberately does **not** diagnose, assign disorder severity,
//! recommend treatment, or determine crisis disposition. It only answers a
//! narrower software question: does a response set satisfy the exact structural
//! contract of a registered instrument/scorer revision that Mycelix has chosen
//! to support?
//!
//! The first revision is intentionally conservative. Instruments whose current
//! Mycelix representation is ambiguous, version-dependent, branching, or not
//! yet tied to an exact scorer are refused instead of receiving generic
//! validation or severity.

use std::collections::BTreeSet;

/// Stable identity for instrument families already represented by the health hApp.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum InstrumentId {
    Phq9,
    Phq2,
    Gad7,
    Audit,
    Dast10,
    Cage,
    Cssrs,
    Pcl5,
    Mdq,
    Epds,
    Psc17,
    Custom,
}

/// Whether this exact v1 registry is willing to validate a response set for
/// generic score calculation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportStatus {
    /// Exact response cardinality and item score bounds are frozen here.
    Supported,
    /// The instrument requires its own versioned/branching protocol rather than
    /// the generic summed-score path.
    SeparateProtocolRequired,
    /// Mycelix has not yet frozen an exact v1 scorer/response contract.
    Unsupported,
}

/// Versioned structural contract for one instrument/scorer pairing.
///
/// `version` and `scoring_revision` are identity-bearing semantic labels; this
/// crate intentionally does not infer them from the enum discriminant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstrumentSpec {
    pub id: InstrumentId,
    pub namespace: &'static str,
    pub version: &'static str,
    pub scoring_revision: &'static str,
    pub expected_responses: Option<usize>,
    pub max_item_score: Option<u8>,
    pub support: SupportStatus,
}

const fn supported(
    id: InstrumentId,
    namespace: &'static str,
    version: &'static str,
    scoring_revision: &'static str,
    expected_responses: usize,
    max_item_score: u8,
) -> InstrumentSpec {
    InstrumentSpec {
        id,
        namespace,
        version,
        scoring_revision,
        expected_responses: Some(expected_responses),
        max_item_score: Some(max_item_score),
        support: SupportStatus::Supported,
    }
}

const fn separate_protocol(
    id: InstrumentId,
    namespace: &'static str,
    version: &'static str,
) -> InstrumentSpec {
    InstrumentSpec {
        id,
        namespace,
        version,
        scoring_revision: "separate-protocol-required",
        expected_responses: None,
        max_item_score: None,
        support: SupportStatus::SeparateProtocolRequired,
    }
}

const fn unsupported(id: InstrumentId, namespace: &'static str) -> InstrumentSpec {
    InstrumentSpec {
        id,
        namespace,
        version: "unfrozen",
        scoring_revision: "unfrozen",
        expected_responses: None,
        max_item_score: None,
        support: SupportStatus::Unsupported,
    }
}

/// Return the first conservative Mycelix registry entry for an instrument.
///
/// This is a structural software contract, not a claim that an instrument is
/// clinically appropriate for a particular population, purpose, or patient.
pub const fn spec_for(id: InstrumentId) -> InstrumentSpec {
    match id {
        InstrumentId::Phq9 => supported(
            id,
            "mycelix.health.instrument/phq-9",
            "phq-9",
            "response-shape-v1",
            9,
            3,
        ),
        InstrumentId::Phq2 => supported(
            id,
            "mycelix.health.instrument/phq-2",
            "phq-2",
            "response-shape-v1",
            2,
            3,
        ),
        InstrumentId::Gad7 => supported(
            id,
            "mycelix.health.instrument/gad-7",
            "gad-7",
            "response-shape-v1",
            7,
            3,
        ),
        InstrumentId::Audit => supported(
            id,
            "mycelix.health.instrument/audit",
            "audit-10",
            "response-shape-v1",
            10,
            4,
        ),
        // Current Mycelix coordinator already contains explicit DAST-10/CAGE
        // score branches, but the shared validator currently treats them as
        // unknown. This reference core freezes only their structural response
        // shape; score interpretation remains outside this crate.
        InstrumentId::Dast10 => supported(
            id,
            "mycelix.health.instrument/dast-10",
            "dast-10",
            "binary-response-shape-v1",
            10,
            1,
        ),
        InstrumentId::Cage => supported(
            id,
            "mycelix.health.instrument/cage",
            "cage-4",
            "binary-response-shape-v1",
            4,
            1,
        ),
        // C-SSRS has multiple versions/settings and branching semantics. It must
        // not be represented as one generic fixed-count summed severity scale.
        InstrumentId::Cssrs => separate_protocol(
            id,
            "mycelix.health.instrument/c-ssrs",
            "version-required",
        ),
        InstrumentId::Pcl5 => unsupported(id, "mycelix.health.instrument/pcl-5"),
        InstrumentId::Mdq => unsupported(id, "mycelix.health.instrument/mdq"),
        InstrumentId::Epds => unsupported(id, "mycelix.health.instrument/epds"),
        InstrumentId::Psc17 => unsupported(id, "mycelix.health.instrument/psc-17"),
        InstrumentId::Custom => unsupported(id, "mycelix.health.instrument/custom"),
    }
}

/// Structural response supplied to the validator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemResponse<'a> {
    pub item_id: &'a str,
    pub score: u8,
}

/// Fail-closed structural validation failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    UnsupportedInstrument {
        id: InstrumentId,
    },
    SeparateProtocolRequired {
        id: InstrumentId,
    },
    WrongResponseCount {
        expected: usize,
        actual: usize,
    },
    EmptyItemId {
        index: usize,
    },
    DuplicateItemId {
        item_id: String,
    },
    ItemScoreOutOfRange {
        item_id: String,
        score: u8,
        max: u8,
    },
}

/// Validated structural response set.
///
/// Fields are private so downstream code cannot construct a proof-shaped value
/// without passing this crate's checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedResponseSet {
    spec: InstrumentSpec,
    response_count: usize,
    raw_sum: u32,
}

impl ValidatedResponseSet {
    pub const fn spec(&self) -> InstrumentSpec {
        self.spec
    }

    pub const fn response_count(&self) -> usize {
        self.response_count
    }

    /// Arithmetic sum only. This is not a diagnosis, severity, screening
    /// disposition, clinical interpretation, or permission to act.
    pub const fn raw_sum(&self) -> u32 {
        self.raw_sum
    }
}

/// Validate only the exact structural contract frozen by [`InstrumentSpec`].
///
/// Unknown, unsupported, and separate-protocol instruments fail closed. No
/// generic fallback is available.
pub fn validate_responses(
    id: InstrumentId,
    responses: &[ItemResponse<'_>],
) -> Result<ValidatedResponseSet, ValidationError> {
    let spec = spec_for(id);

    match spec.support {
        SupportStatus::Unsupported => {
            return Err(ValidationError::UnsupportedInstrument { id });
        }
        SupportStatus::SeparateProtocolRequired => {
            return Err(ValidationError::SeparateProtocolRequired { id });
        }
        SupportStatus::Supported => {}
    }

    let expected = spec
        .expected_responses
        .expect("supported instrument must freeze response count");
    let max = spec
        .max_item_score
        .expect("supported instrument must freeze item score range");

    if responses.len() != expected {
        return Err(ValidationError::WrongResponseCount {
            expected,
            actual: responses.len(),
        });
    }

    let mut ids = BTreeSet::new();
    let mut raw_sum = 0u32;

    for (index, response) in responses.iter().enumerate() {
        let item_id = response.item_id.trim();
        if item_id.is_empty() {
            return Err(ValidationError::EmptyItemId { index });
        }
        if !ids.insert(item_id) {
            return Err(ValidationError::DuplicateItemId {
                item_id: item_id.to_owned(),
            });
        }
        if response.score > max {
            return Err(ValidationError::ItemScoreOutOfRange {
                item_id: item_id.to_owned(),
                score: response.score,
                max,
            });
        }
        raw_sum = raw_sum
            .checked_add(response.score as u32)
            .expect("bounded response count and u8 scores cannot overflow u32");
    }

    Ok(ValidatedResponseSet {
        spec,
        response_count: responses.len(),
        raw_sum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbered(count: usize, score: u8) -> Vec<(String, u8)> {
        (1..=count).map(|n| (format!("q{n}"), score)).collect()
    }

    fn borrow(values: &[(String, u8)]) -> Vec<ItemResponse<'_>> {
        values
            .iter()
            .map(|(id, score)| ItemResponse {
                item_id: id,
                score: *score,
            })
            .collect()
    }

    #[test]
    fn phq9_requires_exact_response_count() {
        let short = numbered(8, 0);
        let error = validate_responses(InstrumentId::Phq9, &borrow(&short)).unwrap_err();
        assert_eq!(
            error,
            ValidationError::WrongResponseCount {
                expected: 9,
                actual: 8,
            }
        );
    }

    #[test]
    fn phq9_rejects_out_of_range_item() {
        let mut values = numbered(9, 0);
        values[4].1 = 4;
        let error = validate_responses(InstrumentId::Phq9, &borrow(&values)).unwrap_err();
        assert_eq!(
            error,
            ValidationError::ItemScoreOutOfRange {
                item_id: "q5".to_string(),
                score: 4,
                max: 3,
            }
        );
    }

    #[test]
    fn duplicate_item_ids_fail_closed() {
        let mut values = numbered(9, 0);
        values[8].0 = "q1".to_string();
        let error = validate_responses(InstrumentId::Phq9, &borrow(&values)).unwrap_err();
        assert_eq!(
            error,
            ValidationError::DuplicateItemId {
                item_id: "q1".to_string(),
            }
        );
    }

    #[test]
    fn dast10_and_cage_are_binary_structural_contracts() {
        let dast = numbered(10, 1);
        assert!(validate_responses(InstrumentId::Dast10, &borrow(&dast)).is_ok());

        let cage = numbered(4, 1);
        assert!(validate_responses(InstrumentId::Cage, &borrow(&cage)).is_ok());

        let mut invalid = numbered(4, 0);
        invalid[0].1 = 2;
        assert!(matches!(
            validate_responses(InstrumentId::Cage, &borrow(&invalid)),
            Err(ValidationError::ItemScoreOutOfRange { max: 1, .. })
        ));
    }

    #[test]
    fn cssrs_requires_a_separate_protocol() {
        let error = validate_responses(InstrumentId::Cssrs, &[]).unwrap_err();
        assert_eq!(
            error,
            ValidationError::SeparateProtocolRequired {
                id: InstrumentId::Cssrs,
            }
        );
    }

    #[test]
    fn currently_unfrozen_instruments_never_receive_generic_validation() {
        for id in [
            InstrumentId::Pcl5,
            InstrumentId::Mdq,
            InstrumentId::Epds,
            InstrumentId::Psc17,
            InstrumentId::Custom,
        ] {
            assert_eq!(
                validate_responses(id, &[]),
                Err(ValidationError::UnsupportedInstrument { id })
            );
        }
    }

    #[test]
    fn validated_raw_sum_is_explicitly_arithmetic_only() {
        let values = numbered(7, 2);
        let validated = validate_responses(InstrumentId::Gad7, &borrow(&values)).unwrap();
        assert_eq!(validated.response_count(), 7);
        assert_eq!(validated.raw_sum(), 14);
        assert_eq!(validated.spec().support, SupportStatus::Supported);
    }
}
