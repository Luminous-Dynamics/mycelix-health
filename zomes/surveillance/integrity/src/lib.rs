#![deny(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Policy-bound aggregate public-health surveillance integrity zome.
//!
//! This DNA stores only observations that have already crossed the aggregate
//! evidence boundary defined by `health-surveillance-core`. Publication policy
//! is a DNA property so every peer validates the same structural privacy floor.
//! If no release policy is configured, publication fails closed.
//!
//! This zome does not authenticate the institutional meaning of a producer label,
//! diagnose disease, declare an outbreak, recommend treatment, or authorize an
//! emergency response.

use hdi::prelude::*;
pub use health_surveillance_core::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReleasePolicyProperties {
    /// Human/audit identity for this exact deployment policy revision.
    pub policy_revision: String,
    pub min_cohort_size: u64,
    pub min_window_s: u64,
    pub max_geographic_precision: GeographicPrecision,
}

#[dna_properties]
#[derive(Clone, Debug, PartialEq)]
pub struct SurveillanceDnaProperties {
    /// `None` intentionally disables publication. Deployments must choose and
    /// pin a non-zero release policy before accepting aggregate evidence.
    pub release_policy: Option<ReleasePolicyProperties>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfiguredReleasePolicy {
    pub policy_revision: CanonicalId,
    pub policy: AggregateReleasePolicy,
}

pub fn configured_release_policy() -> ExternResult<ConfiguredReleasePolicy> {
    let properties = SurveillanceDnaProperties::try_from_dna_properties()?;
    let configured = properties.release_policy.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "Public-health surveillance publication is disabled: no DNA release_policy is configured"
                .to_string()
        ))
    })?;

    configured_release_policy_from_properties(&configured).map_err(|message| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Invalid surveillance DNA release policy: {message}"
        )))
    })
}

pub fn configured_release_policy_from_properties(
    properties: &ReleasePolicyProperties,
) -> Result<ConfiguredReleasePolicy, String> {
    let policy_revision = CanonicalId::new(properties.policy_revision.clone())
        .map_err(|e| format!("invalid policy_revision: {e}"))?;
    let policy = AggregateReleasePolicy::new(
        properties.min_cohort_size,
        properties.min_window_s,
        properties.max_geographic_precision,
    )
    .map_err(|e| e.to_string())?;

    Ok(ConfiguredReleasePolicy {
        policy_revision,
        policy,
    })
}

/// One aggregate observation admitted for publication by the policy frozen in
/// this DNA's integrity properties.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ReleasedSurveillanceObservation {
    pub observation: SurveillanceObservation,
    /// Exact release decision recomputed by every validating peer.
    pub release_assessment: ReleaseAssessment,
    /// Human/audit revision echoed from the DNA property contract.
    pub policy_revision: CanonicalId,
    /// Agent that authored this DHT entry. This authenticates only the Holochain
    /// author key; it does not prove the institutional producer label in the
    /// observation's provenance.
    pub publisher: AgentPubKey,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ReleasedSurveillanceObservation(ReleasedSurveillanceObservation),
}

/// v1 intentionally exposes no index links. A future indexing tranche must
/// define and validate query semantics separately rather than making links look
/// more authoritative than the released entries they point to.
#[hdk_link_types]
pub enum LinkTypes {
    Reserved,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::ReleasedSurveillanceObservation(entry) => {
                    validate_released_entry(&action.author, &entry)
                }
            },
            OpEntry::UpdateEntry { .. } => Ok(ValidateCallbackResult::Invalid(
                "Released surveillance observations are append-only and cannot be updated".into(),
            )),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => Ok(ValidateCallbackResult::Invalid(
            "Released surveillance observations are append-only and cannot be updated".into(),
        )),
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Invalid(
            "Released surveillance observations are evidence records and cannot be deleted".into(),
        )),
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Invalid(
            "Surveillance v1 defines no publishable link operations".into(),
        )),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Invalid(
            "Surveillance v1 defines no deletable link operations".into(),
        )),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_released_entry(
    action_author: &AgentPubKey,
    entry: &ReleasedSurveillanceObservation,
) -> ExternResult<ValidateCallbackResult> {
    let configured = configured_release_policy()?;
    match validate_released_entry_against_policy(action_author, entry, &configured) {
        Ok(()) => Ok(ValidateCallbackResult::Valid),
        Err(message) => Ok(ValidateCallbackResult::Invalid(message)),
    }
}

pub fn validate_released_entry_against_policy(
    action_author: &AgentPubKey,
    entry: &ReleasedSurveillanceObservation,
    configured: &ConfiguredReleasePolicy,
) -> Result<(), String> {
    if &entry.publisher != action_author {
        return Err("publisher must equal the Holochain action author".to_string());
    }
    if entry.policy_revision != configured.policy_revision {
        return Err("entry policy_revision does not match the DNA release policy".to_string());
    }

    entry
        .observation
        .validate()
        .map_err(|e| format!("invalid surveillance observation: {e}"))?;

    let expected = configured
        .policy
        .assess(&entry.observation)
        .map_err(|e| format!("release assessment failed: {e}"))?;

    if !expected.eligible_for_release() {
        return Err(format!(
            "observation does not satisfy the DNA release policy: {:?}",
            expected.reasons()
        ));
    }
    if entry.release_assessment != expected {
        return Err(
            "stored release assessment does not match deterministic policy evaluation".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ConfiguredReleasePolicy {
        configured_release_policy_from_properties(&ReleasePolicyProperties {
            policy_revision: "district-release-v1".to_string(),
            min_cohort_size: 50,
            min_window_s: 3_600,
            max_geographic_precision: GeographicPrecision::District,
        })
        .unwrap()
    }

    fn observation(cohort_size: u64) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::LaboratoryAggregate,
            "lab-feed-a",
            IndependenceGroup::new("lab-lineage-a").unwrap(),
            GeographicScope::new("health-district", "district-17", GeographicPrecision::District)
                .unwrap(),
            ObservationWindow::new(10_000, 13_600).unwrap(),
            13_700,
            cohort_size,
            ObservedMetric::new(
                MetricKind::FractionPositive,
                0.20,
                BoundedUncertainty::new(0.15, 0.25).unwrap(),
                "fraction",
            )
            .unwrap(),
            EvidenceProvenance::new(
                "lab-a",
                "aggregate-protocol-v1",
                "rev-1",
                Some("upstream-lab-a"),
                SourceRecordDigest::sha256([7; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn agent(byte: u8) -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![byte; 36])
    }

    #[test]
    fn missing_or_zero_policy_is_not_constructible_as_configured_policy() {
        let result = configured_release_policy_from_properties(&ReleasePolicyProperties {
            policy_revision: "policy-v1".to_string(),
            min_cohort_size: 0,
            min_window_s: 3_600,
            max_geographic_precision: GeographicPrecision::District,
        });
        assert!(result.is_err());
    }

    #[test]
    fn exact_policy_assessment_and_author_are_required() {
        let configured = policy();
        let observation = observation(100);
        let assessment = configured.policy.assess(&observation).unwrap();
        let publisher = agent(1);
        let entry = ReleasedSurveillanceObservation {
            observation,
            release_assessment: assessment,
            policy_revision: configured.policy_revision.clone(),
            publisher: publisher.clone(),
        };

        assert!(validate_released_entry_against_policy(&publisher, &entry, &configured).is_ok());
        assert!(validate_released_entry_against_policy(&agent(2), &entry, &configured).is_err());
    }

    #[test]
    fn undersized_aggregate_cannot_be_published_even_with_forged_pass_receipt() {
        let configured = policy();
        let good = observation(100);
        let forged_assessment = configured.policy.assess(&good).unwrap();
        let publisher = agent(1);
        let entry = ReleasedSurveillanceObservation {
            observation: observation(10),
            release_assessment: forged_assessment,
            policy_revision: configured.policy_revision.clone(),
            publisher: publisher.clone(),
        };

        assert!(validate_released_entry_against_policy(&publisher, &entry, &configured).is_err());
    }

    #[test]
    fn policy_revision_substitution_is_rejected() {
        let configured = policy();
        let observation = observation(100);
        let assessment = configured.policy.assess(&observation).unwrap();
        let publisher = agent(1);
        let entry = ReleasedSurveillanceObservation {
            observation,
            release_assessment: assessment,
            policy_revision: CanonicalId::new("different-policy").unwrap(),
            publisher: publisher.clone(),
        };

        assert!(validate_released_entry_against_policy(&publisher, &entry, &configured).is_err());
    }
}
