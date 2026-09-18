#![deny(unsafe_code)]
//! Thin coordinator for DNA-rooted clinical distribution evaluator admission.
//!
//! Deliberately contains no evaluator-trust decision logic. The coordinator only
//! commits/reads entries and links; integrity validation independently enforces
//! DNA-rooted exact-target authorization and append-only revocation semantics.
//!
//! The read side exposes an explicit bounded stable-double-read snapshot for one
//! known evaluator admission. This is a local/network observation boundary, not
//! proof of global DHT completeness or absence of an unpropagated revocation.

use clinical_distribution_trust_integrity::*;
use hdk::prelude::*;

const MAX_REVOCATIONS_PER_ADMISSION: usize = 256;

#[derive(Clone, Serialize, Deserialize)]
pub struct ObservedDistributionEvaluatorRevocationV1 {
    pub action_hash: ActionHash,
    pub revocation: DistributionEvaluatorAdmissionRevocation,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistributionEvaluatorSnapshotReadBoundaryV1 {
    NetworkBackedStableDoubleRead,
}

/// One exact evaluator-admission publication plus every revocation target
/// observed by this conductor across a stable network-backed double read.
///
/// This does not prove that an unpropagated revocation does not exist. Consumers
/// must preserve `read_boundary` and must not rename this result to unqualified
/// `Current`/`Active` trust.
#[derive(Clone, Serialize, Deserialize)]
pub struct DistributionEvaluatorAdmissionSnapshotV1 {
    pub admission_action_hash: ActionHash,
    pub admission: QualifiedDistributionEvaluatorAdmission,
    pub revocations: Vec<ObservedDistributionEvaluatorRevocationV1>,
    pub read_boundary: DistributionEvaluatorSnapshotReadBoundaryV1,
    pub observed_revocation_target_count: usize,
    pub observation_started_at: Timestamp,
    pub observation_completed_at: Timestamp,
}

#[hdk_extern]
pub fn issue_distribution_evaluator_authorization(
    authorization: DistributionEvaluatorVerifierAuthorization,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::DistributionEvaluatorVerifierAuthorization(
        authorization,
    ))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn admit_distribution_evaluator(
    admission: QualifiedDistributionEvaluatorAdmission,
) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::QualifiedDistributionEvaluatorAdmission(admission))?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn revoke_distribution_evaluator(
    revocation: DistributionEvaluatorAdmissionRevocation,
) -> ExternResult<Record> {
    let admission_hash = revocation.admission_hash.clone();
    let hash = create_entry(&EntryTypes::DistributionEvaluatorAdmissionRevocation(
        revocation,
    ))?;
    create_link(
        admission_hash,
        hash.clone(),
        LinkTypes::AdmissionToRevocations,
        (),
    )?;
    get_required_record(hash)
}

#[hdk_extern]
pub fn get_distribution_evaluator_admission(
    hash: ActionHash,
) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::network())
}

/// Materialize one known evaluator admission and the revocation links observed by
/// this conductor, requiring the observed target set to remain unchanged around
/// materialization.
#[hdk_extern]
pub fn materialize_distribution_evaluator_admission_snapshot(
    admission_hash: ActionHash,
) -> ExternResult<DistributionEvaluatorAdmissionSnapshotV1> {
    let observation_started_at = sys_time()?;

    let admission_record = get(admission_hash.clone(), GetOptions::network())?.ok_or(
        wasm_error!(WasmErrorInner::Guest(
            "Qualified distribution evaluator admission was not found".to_string()
        )),
    )?;
    let admission: QualifiedDistributionEvaluatorAdmission =
        decode_entry(&admission_record, "qualified distribution evaluator admission")?;
    let proposal_digest = admission.proposal.digest()?;

    let first_targets = observed_revocation_targets(&admission_hash)?;
    let mut revocations = Vec::with_capacity(first_targets.len());

    for revocation_hash in &first_targets {
        let record = get(revocation_hash.clone(), GetOptions::network())?.ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Observed distribution evaluator revocation target could not be materialized"
                    .to_string()
            )
        ))?;
        let revocation: DistributionEvaluatorAdmissionRevocation =
            decode_entry(&record, "distribution evaluator admission revocation")?;

        if revocation.admission_hash != admission_hash {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed evaluator revocation targets another admission".to_string()
            )));
        }
        if revocation.admission_proposal_digest != proposal_digest {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Observed evaluator revocation crosses admission-proposal lineage".to_string()
            )));
        }

        revocations.push(ObservedDistributionEvaluatorRevocationV1 {
            action_hash: revocation_hash.clone(),
            revocation,
        });
    }

    let second_targets = observed_revocation_targets(&admission_hash)?;
    let observation_completed_at = sys_time()?;
    if first_targets != second_targets {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Distribution evaluator revocation link set changed during materialization; retry from a fresh snapshot"
                .to_string()
        )));
    }

    Ok(DistributionEvaluatorAdmissionSnapshotV1 {
        admission_action_hash: admission_hash,
        admission,
        revocations,
        read_boundary: DistributionEvaluatorSnapshotReadBoundaryV1::NetworkBackedStableDoubleRead,
        observed_revocation_target_count: first_targets.len(),
        observation_started_at,
        observation_completed_at,
    })
}

fn observed_revocation_targets(admission_hash: &ActionHash) -> ExternResult<Vec<ActionHash>> {
    let links = get_links(
        LinkQuery::try_new(
            admission_hash.clone(),
            LinkTypes::AdmissionToRevocations,
        )?,
        GetStrategy::Network,
    )?;

    if links.len() > MAX_REVOCATIONS_PER_ADMISSION {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Distribution evaluator admission has more than {MAX_REVOCATIONS_PER_ADMISSION} observed revocation links; refusing unbounded materialization"
        ))));
    }

    let mut targets = Vec::with_capacity(links.len());
    for link in links {
        let target = link.target.into_action_hash().ok_or(wasm_error!(
            WasmErrorInner::Guest(
                "Distribution evaluator revocation link target must be an ActionHash".to_string()
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
        "Committed clinical distribution trust record could not be read back".to_string()
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
