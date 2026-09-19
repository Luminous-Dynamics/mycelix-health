#![deny(unsafe_code)]
//! Thin coordinator for positive clinical distribution evaluator leases.
//!
//! This zome materializes a stable network-backed observation of one exact lease
//! lineage. It does not inspect the separate admission-revocation links and does
//! not return an unqualified `Current` or `Trusted` evaluator state.

use clinical_distribution_lease_integrity::*;
use clinical_distribution_trust_integrity::QualifiedDistributionEvaluatorAdmission;
use hdk::prelude::*;
use std::collections::{HashMap, HashSet};

const MAX_LEASES_PER_ADMISSION: usize = 256;

#[derive(Clone, Serialize, Deserialize)]
pub struct ObservedDistributionEvaluatorLeaseV1 {
    pub action_hash: ActionHash,
    pub lease: ClinicalDistributionEvaluatorLeaseV1,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistributionEvaluatorLeaseReadBoundaryV1 {
    NetworkBackedStableDoubleRead,
}

/// Positive lease evidence observed for one exact evaluator admission.
///
/// The selected lease is the unique highest-sequence lease in the single observed
/// supersession lineage and must be valid at `observation_completed_at`.
///
/// This is not evaluator currentness by itself. Consumers must separately prove
/// the admission is not *known* revoked using the revocation snapshot boundary.
#[derive(Clone, Serialize, Deserialize)]
pub struct DistributionEvaluatorPositiveLeaseObservationV1 {
    pub admission_action_hash: ActionHash,
    pub admission_proposal_digest: [u8; 32],
    pub selected_lease_action_hash: ActionHash,
    pub selected_lease: ClinicalDistributionEvaluatorLeaseV1,
    pub observed_lease_target_count: usize,
    pub read_boundary: DistributionEvaluatorLeaseReadBoundaryV1,
    pub observation_started_at: Timestamp,
    pub observation_completed_at: Timestamp,
}

#[hdk_extern]
pub fn issue_distribution_evaluator_lease(
    lease: ClinicalDistributionEvaluatorLeaseV1,
) -> ExternResult<Record> {
    let admission_hash = lease.admission_hash.clone();
    let lease_hash = create_entry(&EntryTypes::ClinicalDistributionEvaluatorLeaseV1(lease))?;
    create_link(
        admission_hash,
        lease_hash.clone(),
        LinkTypes::AdmissionToLeases,
        (),
    )?;
    get_required_record(lease_hash)
}

#[hdk_extern]
pub fn get_distribution_evaluator_lease(
    hash: ActionHash,
) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::network())
}

/// Materialize the positive lease lineage observed for one exact admission.
///
/// The observed lease-link target set must remain unchanged across a network-backed
/// double read. Competing lease lineages, sequence ambiguity, expired/future
/// selected leases, missing supersession ancestors, or target substitution fail
/// closed.
#[hdk_extern]
pub fn materialize_distribution_evaluator_positive_lease_observation(
    admission_hash: ActionHash,
) -> ExternResult<DistributionEvaluatorPositiveLeaseObservationV1> {
    let observation_started_at = sys_time()?;

    let admission_record = get(admission_hash.clone(), GetOptions::network())?.ok_or(
        wasm_error!(WasmErrorInner::Guest(
            "Qualified distribution evaluator admission was not found".to_string()
        )),
    )?;
    let admission: QualifiedDistributionEvaluatorAdmission =
        decode_entry(&admission_record, "qualified distribution evaluator admission")?;
    let proposal_digest = admission.proposal.digest()?;

    let first_targets = observed_lease_targets(&admission_hash)?;
    if first_targets.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "No positive evaluator lease is observed for this admission".to_string()
        )));
    }

    let mut leases = Vec::with_capacity(first_targets.len());
    for lease_hash in &first_targets {
        let record = get(lease_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Observed distribution evaluator lease target could not be materialized"
                    .to_string()
            )
        ))?;
        let lease: ClinicalDistributionEvaluatorLeaseV1 =
            decode_entry(&record, "clinical distribution evaluator lease")?;
        if lease.admission_hash != admission_hash {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed evaluator lease targets another admission".to_string()
            )));
        }
        if lease.admission_proposal_digest != proposal_digest {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed evaluator lease crosses admission-proposal lineage".to_string()
            )));
        }
        if lease.detector != admission.proposal.detector
            || lease.distribution_policy_digest != admission.proposal.distribution_policy_digest
            || lease.trust_policy_digest != admission.proposal.trust_policy_digest
        {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed evaluator lease crosses detector or policy lineage".to_string()
            )));
        }
        leases.push(ObservedDistributionEvaluatorLeaseV1 {
            action_hash: lease_hash.clone(),
            lease,
        });
    }

    let second_targets = observed_lease_targets(&admission_hash)?;
    let observation_completed_at = sys_time()?;
    if first_targets != second_targets {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Distribution evaluator lease link set changed during materialization; retry from a fresh observation"
                .to_string()
        )));
    }

    let selected = select_unique_current_lease_lineage(&leases, observation_completed_at)?;
    if observation_completed_at < admission.proposal.valid_from {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Evaluator admission is not yet valid at lease observation time".to_string()
        )));
    }
    if admission
        .proposal
        .valid_until
        .is_some_and(|until| observation_completed_at >= until)
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Evaluator admission expired before lease observation completed".to_string()
        )));
    }

    Ok(DistributionEvaluatorPositiveLeaseObservationV1 {
        admission_action_hash: admission_hash,
        admission_proposal_digest: proposal_digest,
        selected_lease_action_hash: selected.action_hash.clone(),
        selected_lease: selected.lease.clone(),
        observed_lease_target_count: first_targets.len(),
        read_boundary: DistributionEvaluatorLeaseReadBoundaryV1::NetworkBackedStableDoubleRead,
        observation_started_at,
        observation_completed_at,
    })
}

fn select_unique_current_lease_lineage<'a>(
    leases: &'a [ObservedDistributionEvaluatorLeaseV1],
    observed_at: Timestamp,
) -> ExternResult<&'a ObservedDistributionEvaluatorLeaseV1> {
    let max_sequence = leases
        .iter()
        .map(|observed| observed.lease.lease_sequence)
        .max()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No evaluator lease candidates were supplied".to_string()
        )))?;

    let mut highest = leases
        .iter()
        .filter(|observed| observed.lease.lease_sequence == max_sequence);
    let selected = highest.next().ok_or(wasm_error!(WasmErrorInner::Guest(
        "No highest-sequence evaluator lease exists".to_string()
    )))?;
    if highest.next().is_some() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Ambiguous evaluator lease lineage: multiple leases share the highest sequence"
                .to_string()
        )));
    }

    if observed_at < selected.lease.valid_from || observed_at >= selected.lease.valid_until {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Highest-sequence positive evaluator lease is not currently valid".to_string()
        )));
    }

    let by_hash: HashMap<ActionHash, &ObservedDistributionEvaluatorLeaseV1> = leases
        .iter()
        .map(|observed| (observed.action_hash.clone(), observed))
        .collect();
    if by_hash.len() != leases.len() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Duplicate evaluator lease action hash observed".to_string()
        )));
    }

    let mut chain_hashes = HashSet::new();
    let mut cursor = selected;
    loop {
        if !chain_hashes.insert(cursor.action_hash.clone()) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Evaluator lease supersession lineage contains a cycle".to_string()
            )));
        }
        match (&cursor.lease.supersedes_lease_hash, cursor.lease.lease_sequence) {
            (None, 0) => break,
            (Some(previous_hash), sequence) if sequence > 0 => {
                let previous = by_hash.get(previous_hash).copied().ok_or(wasm_error!(
                    WasmErrorInner::Guest(
                        "Observed evaluator lease lineage is missing a superseded ancestor"
                            .to_string()
                    )
                ))?;
                if previous.lease.lease_sequence.checked_add(1) != Some(sequence) {
                    return Err(wasm_error!(WasmErrorInner::Guest(
                        "Observed evaluator lease lineage has a non-consecutive sequence"
                            .to_string()
                    )));
                }
                cursor = previous;
            }
            _ => {
                return Err(wasm_error!(WasmErrorInner::Guest(
                    "Observed evaluator lease lineage has inconsistent sequence/supersession shape"
                        .to_string()
                )));
            }
        }
    }

    if chain_hashes.len() != leases.len() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Competing evaluator lease lineage observed; refusing ambiguous currentness evidence"
                .to_string()
        )));
    }

    Ok(selected)
}

fn observed_lease_targets(admission_hash: &ActionHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(admission_hash.clone(), LinkTypes::AdmissionToLeases)?,
        GetStrategy::Network,
    )?;
    if links.len() > MAX_LEASES_PER_ADMISSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Distribution evaluator admission has more than {MAX_LEASES_PER_ADMISSION} observed lease links; refusing unbounded materialization"
        ))));
    }

    let mut targets = Vec::with_capacity(links.len());
    for link in links {
        let target = link.target.into_action_hash().ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Distribution evaluator lease link target must be an ActionHash".to_string()
            )
        ))?;
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets.sort_by(|left, right| left.get_raw_36().cmp(right.get_raw_36()));
    Ok(targets)
}

fn get_required_record(hash: ActionHash) -> ExternResult<Record> {
    get(hash, GetOptions::network())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Committed clinical distribution lease record could not be read back".to_string()
    )))
}

fn decode_entry<T>(record: &Record, label: &'static str) -> ExternResult<T>
where
    T: TryFrom<SerializedBytes, Error = SerializedBytesError>,
{
    record
        .entry()
        .to_app_option::<T>()
        .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "Referenced {label} entry is missing or wrong type"
        ))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(sequence: u64, supersedes: Option<ActionHash>, seed: u8) -> ObservedDistributionEvaluatorLeaseV1 {
        ObservedDistributionEvaluatorLeaseV1 {
            action_hash: ActionHash::from_raw_36(vec![seed; 36]),
            lease: ClinicalDistributionEvaluatorLeaseV1 {
                schema_version: 1,
                lease_id: format!("lease-{sequence}-{seed}"),
                admission_hash: ActionHash::from_raw_36(vec![9; 36]),
                admission_proposal_digest: [8u8; 32],
                detector: clinical_distribution_trust_integrity::RuntimeArtifactIdentityV1 {
                    name: "detector".into(),
                    version: "1".into(),
                    digest: clinical_distribution_trust_integrity::RuntimeContentDigestV1 {
                        algorithm: "blake3-256".into(),
                        value: "detector".into(),
                    },
                },
                distribution_policy_digest: clinical_distribution_trust_integrity::RuntimeContentDigestV1 {
                    algorithm: "blake3-256".into(),
                    value: "distribution".into(),
                },
                trust_policy_digest: clinical_distribution_trust_integrity::RuntimeContentDigestV1 {
                    algorithm: "blake3-256".into(),
                    value: "trust".into(),
                },
                lease_sequence: sequence,
                supersedes_lease_hash: supersedes,
                valid_from: Timestamp::from_micros(100),
                valid_until: Timestamp::from_micros(200),
            },
        }
    }

    #[test]
    fn highest_unique_sequence_must_be_current() {
        let first = observed(0, None, 1);
        let second = observed(1, Some(first.action_hash.clone()), 2);
        let leases = vec![first, second];
        let selected =
            select_unique_current_lease_lineage(&leases, Timestamp::from_micros(150)).unwrap();
        assert_eq!(selected.lease.lease_sequence, 1);
    }

    #[test]
    fn expired_highest_sequence_blocks_older_lease_reuse() {
        let mut first = observed(0, None, 1);
        first.lease.valid_until = Timestamp::from_micros(300);
        let second = observed(1, Some(first.action_hash.clone()), 2);
        let leases = vec![first, second];
        assert!(select_unique_current_lease_lineage(&leases, Timestamp::from_micros(250)).is_err());
    }

    #[test]
    fn competing_root_lineages_fail_closed() {
        let first = observed(0, None, 1);
        let competing = observed(0, None, 2);
        let leases = vec![first, competing];
        assert!(select_unique_current_lease_lineage(&leases, Timestamp::from_micros(150)).is_err());
    }

    #[test]
    fn missing_superseded_ancestor_fails_closed() {
        let missing = ActionHash::from_raw_36(vec![7; 36]);
        let lease = observed(1, Some(missing), 2);
        assert!(select_unique_current_lease_lineage(&[lease], Timestamp::from_micros(150)).is_err());
    }
}
