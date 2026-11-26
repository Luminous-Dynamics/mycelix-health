#![forbid(unsafe_code)]

//! Offline reference contract for immutable FHIR source revisions.
//!
//! This crate classifies source revision events. It does not authenticate the
//! supplied source principal, compute payload digests, canonicalize FHIR JSON,
//! write Holochain records, or decide clinical authority. Those remain separate
//! implementation and qualification boundaries.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalResourceKey {
    /// Tenant or trust-domain partition. Same FHIR IDs in different tenants do not collide.
    pub tenant_id: String,
    /// Must be injected by an authenticated source boundary, not caller metadata.
    pub source_principal: String,
    pub resource_type: String,
    pub logical_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOrder {
    /// Identifier for a documented source-specific ordering contract.
    pub profile_id: String,
    /// Monotonic sequence supplied under that exact ordering contract.
    pub sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRevision {
    pub logical_key: LogicalResourceKey,
    /// FHIR meta.versionId is opaque. It is compared for equality, never sorted.
    pub version_id: Option<String>,
    /// Digest of canonical bytes under the exact named canonicalization profile.
    /// The digest is supplied by the caller; this crate does not calculate it.
    pub payload_digest: [u8; 32],
    pub canonicalization_profile: String,
    /// Optional only where an authenticated source adapter has a documented
    /// monotonic order contract. Absence means chronology is unknown.
    pub source_order: Option<SourceOrder>,
    /// Source-provided lastUpdated time; preserved but never used as revision order.
    pub source_last_updated_micros: Option<i64>,
    /// Local receipt time; preserved but never used as source revision order.
    pub received_at_micros: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevisionError {
    EmptyTenant,
    EmptySourcePrincipal,
    EmptyResourceType,
    InvalidResourceType,
    InvalidLogicalId,
    InvalidVersionId,
    EmptyCanonicalizationProfile,
    EmptySourceOrderProfile,
    CurrentIdentityMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevisionDisposition {
    /// No prior revision exists for this logical resource.
    FirstRevision,
    /// Same logical key, version ID, digest and canonicalization profile.
    ExactReplay,
    /// Same version ID but a different payload digest.
    VersionEquivocation,
    /// Same version ID and digest bytes, but a different canonicalization profile.
    CanonicalizationProfileConflict,
    /// Same revision identity but contradictory source-order metadata.
    SourceOrderMetadataConflict,
    /// New version is later under the same documented source-order profile.
    NewerBySourceSequence,
    /// New version is earlier under the same documented source-order profile.
    OlderBySourceSequence,
    /// Distinct version IDs claim the same sequence under one ordering profile.
    SourceSequenceCollision,
    /// A new revision exists, but no trusted comparable order is available.
    OrderUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionEffect {
    Promote,
    PreserveHistoryOnly,
    NoChange,
    Quarantine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionAssessment {
    pub disposition: RevisionDisposition,
    pub projection_effect: ProjectionEffect,
}

fn valid_fhir_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
}

fn validate_revision(revision: &SourceRevision) -> Result<(), RevisionError> {
    if revision.logical_key.tenant_id.trim().is_empty() {
        return Err(RevisionError::EmptyTenant);
    }
    if revision.logical_key.source_principal.trim().is_empty() {
        return Err(RevisionError::EmptySourcePrincipal);
    }
    if revision.logical_key.resource_type.trim().is_empty() {
        return Err(RevisionError::EmptyResourceType);
    }
    if !revision
        .logical_key
        .resource_type
        .bytes()
        .all(|byte| byte.is_ascii_alphabetic())
        || !revision.logical_key.resource_type.as_bytes()[0].is_ascii_uppercase()
    {
        return Err(RevisionError::InvalidResourceType);
    }
    if !valid_fhir_id(&revision.logical_key.logical_id) {
        return Err(RevisionError::InvalidLogicalId);
    }
    if let Some(version_id) = &revision.version_id {
        if !valid_fhir_id(version_id) {
            return Err(RevisionError::InvalidVersionId);
        }
    }
    if revision.canonicalization_profile.trim().is_empty() {
        return Err(RevisionError::EmptyCanonicalizationProfile);
    }
    if let Some(order) = &revision.source_order {
        if order.profile_id.trim().is_empty() {
            return Err(RevisionError::EmptySourceOrderProfile);
        }
    }
    Ok(())
}

fn same_logical_resource(left: &SourceRevision, right: &SourceRevision) -> bool {
    left.logical_key == right.logical_key
}

fn assessment(disposition: RevisionDisposition) -> RevisionAssessment {
    let projection_effect = match disposition {
        RevisionDisposition::FirstRevision | RevisionDisposition::NewerBySourceSequence => {
            ProjectionEffect::Promote
        }
        RevisionDisposition::ExactReplay => ProjectionEffect::NoChange,
        RevisionDisposition::VersionEquivocation
        | RevisionDisposition::CanonicalizationProfileConflict
        | RevisionDisposition::SourceOrderMetadataConflict
        | RevisionDisposition::SourceSequenceCollision => ProjectionEffect::Quarantine,
        RevisionDisposition::OlderBySourceSequence | RevisionDisposition::OrderUnknown => {
            ProjectionEffect::PreserveHistoryOnly
        }
    };
    RevisionAssessment {
        disposition,
        projection_effect,
    }
}

/// Classify an incoming immutable source revision against append-only history
/// and an optional current projection.
///
/// The algorithm never infers chronology from:
/// - numeric-looking or lexical version IDs;
/// - source lastUpdated time;
/// - local receive/ingestion time;
/// - array arrival order.
///
/// Projection promotion beyond the first accepted revision requires a matching,
/// documented source-order profile and a strictly increasing source sequence.
pub fn classify_revision(
    incoming: &SourceRevision,
    history: &[SourceRevision],
    current: Option<&SourceRevision>,
) -> Result<RevisionAssessment, RevisionError> {
    validate_revision(incoming)?;
    if let Some(current_revision) = current {
        validate_revision(current_revision)?;
        if !same_logical_resource(incoming, current_revision) {
            return Err(RevisionError::CurrentIdentityMismatch);
        }
    }

    // Treat the supplied append-only history as security-relevant input too.
    // A malformed matching record must not be silently ignored while deriving
    // the current projection.
    for prior in history
        .iter()
        .filter(|revision| same_logical_resource(revision, incoming))
    {
        validate_revision(prior)?;
    }

    let existing: Vec<&SourceRevision> = history
        .iter()
        .chain(current)
        .filter(|revision| same_logical_resource(revision, incoming))
        .collect();

    if existing.is_empty() {
        return Ok(assessment(RevisionDisposition::FirstRevision));
    }

    if let Some(version_id) = &incoming.version_id {
        // Do not return on the first match: replicated or corrupted history
        // can contain contradictory entries for one version ID. Classification
        // must be independent of history iteration order.
        let mut matching_version_found = false;
        let mut profile_conflict = false;
        let mut digest_conflict = false;
        let mut order_conflict = false;

        for prior in existing
            .iter()
            .filter(|prior| prior.version_id.as_ref() == Some(version_id))
        {
            matching_version_found = true;
            if prior.canonicalization_profile != incoming.canonicalization_profile {
                profile_conflict = true;
                continue;
            }
            if prior.payload_digest != incoming.payload_digest {
                digest_conflict = true;
            }
            if prior.source_order != incoming.source_order {
                order_conflict = true;
            }
        }

        if profile_conflict {
            return Ok(assessment(
                RevisionDisposition::CanonicalizationProfileConflict,
            ));
        }
        if digest_conflict {
            return Ok(assessment(RevisionDisposition::VersionEquivocation));
        }
        if order_conflict {
            return Ok(assessment(RevisionDisposition::SourceOrderMetadataConflict));
        }
        if matching_version_found {
            return Ok(assessment(RevisionDisposition::ExactReplay));
        }
    } else {
        let mut unversioned_exact_replay = false;
        let mut source_order_conflict = false;
        for prior in existing.iter().filter(|prior| prior.version_id.is_none()) {
            if prior.payload_digest == incoming.payload_digest
                && prior.canonicalization_profile == incoming.canonicalization_profile
            {
                unversioned_exact_replay = true;
                if prior.source_order != incoming.source_order {
                    source_order_conflict = true;
                }
            }
        }
        if source_order_conflict {
            return Ok(assessment(RevisionDisposition::SourceOrderMetadataConflict));
        }
        if unversioned_exact_replay {
            return Ok(assessment(RevisionDisposition::ExactReplay));
        }
    }

    let Some(current_revision) = current else {
        return Ok(assessment(RevisionDisposition::OrderUnknown));
    };
    let (Some(incoming_order), Some(current_order)) =
        (&incoming.source_order, &current_revision.source_order)
    else {
        return Ok(assessment(RevisionDisposition::OrderUnknown));
    };
    if incoming_order.profile_id != current_order.profile_id {
        return Ok(assessment(RevisionDisposition::OrderUnknown));
    }

    // Compare against every accepted revision with the same documented source
    // ordering profile, not merely the current pointer. If the projection lags
    // behind history, an intermediate late event must not move it forward to
    // an older revision.
    let highest_observed_sequence = existing
        .iter()
        .filter_map(|revision| revision.source_order.as_ref())
        .filter(|order| order.profile_id == incoming_order.profile_id)
        .map(|order| order.sequence)
        .max()
        .unwrap_or(current_order.sequence);

    if incoming_order.sequence > highest_observed_sequence {
        return Ok(assessment(RevisionDisposition::NewerBySourceSequence));
    }
    if incoming_order.sequence < highest_observed_sequence {
        return Ok(assessment(RevisionDisposition::OlderBySourceSequence));
    }

    Ok(assessment(RevisionDisposition::SourceSequenceCollision))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(
        tenant: &str,
        principal: &str,
        version_id: Option<&str>,
        digest_byte: u8,
        sequence: Option<u64>,
    ) -> SourceRevision {
        SourceRevision {
            logical_key: LogicalResourceKey {
                tenant_id: tenant.to_string(),
                source_principal: principal.to_string(),
                resource_type: "Observation".to_string(),
                logical_id: "obs-1".to_string(),
            },
            version_id: version_id.map(str::to_string),
            payload_digest: [digest_byte; 32],
            canonicalization_profile: "fhir-json-c14n-v1".to_string(),
            source_order: sequence.map(|sequence| SourceOrder {
                profile_id: "ehr-revision-sequence-v1".to_string(),
                sequence,
            }),
            source_last_updated_micros: Some(1_700_000_000_000_000),
            received_at_micros: 1_800_000_000_000_000,
        }
    }

    #[test]
    fn first_revision_can_establish_the_initial_projection() {
        let first = revision("tenant-a", "issuer-a", Some("opaque-v1"), 1, None);
        assert_eq!(
            classify_revision(&first, &[], None).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::FirstRevision,
                projection_effect: ProjectionEffect::Promote,
            }
        );
    }

    #[test]
    fn identical_version_and_digest_is_idempotent() {
        let existing = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(1));
        let mut replay = existing.clone();
        replay.received_at_micros += 5_000_000;
        replay.source_last_updated_micros = Some(1_700_000_500_000_000);

        assert_eq!(
            classify_revision(&replay, std::slice::from_ref(&existing), Some(&existing)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::ExactReplay,
                projection_effect: ProjectionEffect::NoChange,
            }
        );
    }

    #[test]
    fn new_version_promotes_only_with_matching_source_order_profile() {
        let current = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(10));
        let incoming = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(11));
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&current), Some(&current)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::NewerBySourceSequence,
                projection_effect: ProjectionEffect::Promote,
            }
        );
    }

    #[test]
    fn late_arriving_old_revision_is_preserved_without_projection_rollback() {
        let current = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(20));
        let late = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(10));
        assert_eq!(
            classify_revision(&late, std::slice::from_ref(&current), Some(&current)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::OlderBySourceSequence,
                projection_effect: ProjectionEffect::PreserveHistoryOnly,
            }
        );
    }

    #[test]
    fn an_intermediate_revision_cannot_advance_a_projection_past_newer_history() {
        let historical_latest = revision("tenant-a", "issuer-a", Some("v3"), 3, Some(30));
        let current_projection = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(20));
        let incoming = revision("tenant-a", "issuer-a", Some("v25"), 25, Some(25));
        let history = vec![
            revision("tenant-a", "issuer-a", Some("v1"), 1, Some(10)),
            historical_latest,
        ];

        assert_eq!(
            classify_revision(&incoming, &history, Some(&current_projection)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::OlderBySourceSequence,
                projection_effect: ProjectionEffect::PreserveHistoryOnly,
            }
        );
    }

    #[test]
    fn contradictory_duplicate_version_history_is_quarantined_independent_of_order() {
        let exact = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(10));
        let equivocation = revision("tenant-a", "issuer-a", Some("v1"), 9, Some(10));
        let current = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(20));

        for history in [
            vec![exact.clone(), equivocation.clone()],
            vec![equivocation.clone(), exact.clone()],
        ] {
            assert_eq!(
                classify_revision(&exact, &history, Some(&current)).unwrap(),
                RevisionAssessment {
                    disposition: RevisionDisposition::VersionEquivocation,
                    projection_effect: ProjectionEffect::Quarantine,
                }
            );
        }
    }

    #[test]
    fn malformed_matching_history_is_rejected_before_projection_decision() {
        let mut malformed = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(10));
        malformed.canonicalization_profile = "  ".to_string();
        let current = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(20));
        let incoming = revision("tenant-a", "issuer-a", Some("v3"), 3, Some(30));

        assert_eq!(
            classify_revision(&incoming, &[malformed], Some(&current)),
            Err(RevisionError::EmptyCanonicalizationProfile)
        );
    }

    #[test]
    fn unversioned_exact_payload_with_changed_source_order_is_not_silent_replay() {
        let prior = revision("tenant-a", "issuer-a", None, 1, Some(10));
        let incoming = revision("tenant-a", "issuer-a", None, 1, Some(11));

        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&prior), Some(&prior)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::SourceOrderMetadataConflict,
                projection_effect: ProjectionEffect::Quarantine,
            }
        );
    }

    #[test]
    fn same_version_with_different_digest_is_quarantined() {
        let existing = revision("tenant-a", "issuer-a", Some("v7"), 1, Some(7));
        let incoming = revision("tenant-a", "issuer-a", Some("v7"), 2, Some(7));
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&existing), Some(&existing)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::VersionEquivocation,
                projection_effect: ProjectionEffect::Quarantine,
            }
        );
    }

    #[test]
    fn canonicalization_profile_change_requires_quarantine_not_silent_replay() {
        let existing = revision("tenant-a", "issuer-a", Some("v7"), 1, Some(7));
        let mut incoming = existing.clone();
        incoming.canonicalization_profile = "fhir-json-c14n-v2".to_string();
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&existing), Some(&existing)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::CanonicalizationProfileConflict,
                projection_effect: ProjectionEffect::Quarantine,
            }
        );
    }

    #[test]
    fn unversioned_changed_payload_is_history_only() {
        let current = revision("tenant-a", "issuer-a", None, 1, None);
        let incoming = revision("tenant-a", "issuer-a", None, 2, None);
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&current), Some(&current)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::OrderUnknown,
                projection_effect: ProjectionEffect::PreserveHistoryOnly,
            }
        );
    }

    #[test]
    fn unversioned_same_digest_is_an_exact_replay() {
        let existing = revision("tenant-a", "issuer-a", None, 1, None);
        assert_eq!(
            classify_revision(&existing, std::slice::from_ref(&existing), Some(&existing)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::ExactReplay,
                projection_effect: ProjectionEffect::NoChange,
            }
        );
    }

    #[test]
    fn opaque_version_ids_are_never_sorted_lexically_or_numerically() {
        let current = revision("tenant-a", "issuer-a", Some("v10"), 10, None);
        let incoming = revision("tenant-a", "issuer-a", Some("v9"), 9, None);
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&current), Some(&current)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::OrderUnknown,
                projection_effect: ProjectionEffect::PreserveHistoryOnly,
            }
        );
    }

    #[test]
    fn source_last_updated_and_local_receive_time_do_not_create_order() {
        let mut current = revision("tenant-a", "issuer-a", Some("v1"), 1, None);
        let mut incoming = revision("tenant-a", "issuer-a", Some("v2"), 2, None);
        current.source_last_updated_micros = Some(1_900_000_000_000_000);
        current.received_at_micros = 1_900_000_000_000_000;
        incoming.source_last_updated_micros = Some(1_700_000_000_000_000);
        incoming.received_at_micros = 1_600_000_000_000_000;
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&current), Some(&current)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::OrderUnknown,
                projection_effect: ProjectionEffect::PreserveHistoryOnly,
            }
        );
    }

    #[test]
    fn different_ordering_profiles_do_not_establish_comparable_order() {
        let mut current = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(1));
        let mut incoming = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(2));
        incoming.source_order.as_mut().unwrap().profile_id = "other-ehr-profile".to_string();
        current.source_order.as_mut().unwrap().profile_id = "ehr-revision-sequence-v1".to_string();
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&current), Some(&current)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::OrderUnknown,
                projection_effect: ProjectionEffect::PreserveHistoryOnly,
            }
        );
    }

    #[test]
    fn two_distinct_versions_with_one_sequence_are_quarantined() {
        let current = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(5));
        let incoming = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(5));
        assert_eq!(
            classify_revision(&incoming, std::slice::from_ref(&current), Some(&current)).unwrap(),
            RevisionAssessment {
                disposition: RevisionDisposition::SourceSequenceCollision,
                projection_effect: ProjectionEffect::Quarantine,
            }
        );
    }

    #[test]
    fn tenant_and_authenticated_source_principal_partition_logical_identity() {
        let prior = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(1));
        let incoming_other_tenant = revision("tenant-b", "issuer-a", Some("v1"), 1, Some(1));
        assert_eq!(
            classify_revision(&incoming_other_tenant, &[prior], None)
                .unwrap()
                .disposition,
            RevisionDisposition::FirstRevision
        );
    }

    #[test]
    fn source_principal_partitions_logical_identity() {
        let prior = revision("tenant-a", "issuer-a", Some("v1"), 1, Some(1));
        let incoming_other_source = revision("tenant-a", "issuer-b", Some("v1"), 1, Some(1));
        assert_eq!(
            classify_revision(&incoming_other_source, &[prior], None)
                .unwrap()
                .disposition,
            RevisionDisposition::FirstRevision
        );
    }

    #[test]
    fn fixture_catalogue_is_frozen_and_has_unique_case_ids() {
        let mut ids = std::collections::BTreeSet::new();
        let mut rows = 0usize;
        for line in include_str!("../fixtures/v1-cases.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 4, "malformed fixture catalogue row");
            assert!(!fields[0].trim().is_empty(), "fixture ID must not be empty");
            assert!(ids.insert(fields[0]), "duplicate fixture ID: {}", fields[0]);
            assert!(
                [
                    "FirstRevision",
                    "ExactReplay",
                    "VersionEquivocation",
                    "CanonicalizationProfileConflict",
                    "NewerBySourceSequence",
                    "OlderBySourceSequence",
                    "SourceSequenceCollision",
                    "OrderUnknown",
                    "InvalidInput"
                ]
                .contains(&fields[2]),
                "unknown disposition in fixture {}",
                fields[0]
            );
            assert!(
                !fields[3].trim().is_empty(),
                "fixture effect must not be empty"
            );
            rows += 1;
        }
        assert_eq!(rows, 18, "v1 fixture catalogue size changed");
    }

    #[test]
    fn invalid_identity_and_version_ids_are_rejected() {
        let mut invalid = revision("tenant-a", "issuer-a", Some("version/with/slash"), 1, None);
        assert_eq!(
            classify_revision(&invalid, &[], None),
            Err(RevisionError::InvalidVersionId)
        );

        invalid.version_id = Some("v1".to_string());
        invalid.logical_key.source_principal = "  ".to_string();
        assert_eq!(
            classify_revision(&invalid, &[], None),
            Err(RevisionError::EmptySourcePrincipal)
        );
    }

    #[test]
    fn current_projection_must_belong_to_the_same_logical_resource() {
        let incoming = revision("tenant-a", "issuer-a", Some("v2"), 2, Some(2));
        let current = revision("tenant-b", "issuer-a", Some("v1"), 1, Some(1));
        assert_eq!(
            classify_revision(&incoming, &[], Some(&current)),
            Err(RevisionError::CurrentIdentityMismatch)
        );
    }
}
