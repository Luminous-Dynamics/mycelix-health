#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! Canonical admission index for DNA-trusted population accountant states.
//!
//! Individual accountant states may exist without admission. Only states linked
//! through this integrity boundary participate in the high-assurance canonical
//! lineage read model. The deterministic synthetic anchor is derived only from the
//! already-public release-policy/accountant-instance/accountant-method tuple.

use hdi::prelude::*;
use mycelix_clinical_integrity::StoredDigest;
use mycelix_clinical_population_release::{
    PopulationReleaseArtifactKindV1, PopulationReleaseDigestV1,
};
use population_accountant_integrity::TrustedPopulationAccountantState;

const INDEX_SCHEMA_VERSION: u16 = 1;

#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct PopulationAccountantLineageAnchorV1 {
    pub schema_version: u16,
    pub release_policy_digest: PopulationReleaseDigestV1,
    pub accountant_instance_digest: StoredDigest,
    pub accountant_method_digest: StoredDigest,
}

impl PopulationAccountantLineageAnchorV1 {
    pub fn new(
        release_policy_digest: PopulationReleaseDigestV1,
        accountant_instance_digest: StoredDigest,
        accountant_method_digest: StoredDigest,
    ) -> ExternResult<Self> {
        release_policy_digest
            .validate()
            .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
        if release_policy_digest.kind != PopulationReleaseArtifactKindV1::ReleasePolicy {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Accountant lineage anchor requires release-policy digest".into()
            )));
        }
        accountant_instance_digest
            .validate_shape()
            .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
        accountant_method_digest
            .validate_shape()
            .map_err(|error| wasm_error!(WasmErrorInner::Guest(error.to_string())))?;
        Ok(Self {
            schema_version: INDEX_SCHEMA_VERSION,
            release_policy_digest,
            accountant_instance_digest,
            accountant_method_digest,
        })
    }

    pub fn from_state(state: &TrustedPopulationAccountantState) -> ExternResult<Self> {
        Self::new(
            state.projection.release_policy_digest,
            state.projection.accountant_instance_digest,
            state.projection.accountant_method_digest,
        )
    }
}

pub fn accountant_lineage_anchor_hash(
    anchor: &PopulationAccountantLineageAnchorV1,
) -> ExternResult<EntryHash> {
    if anchor.schema_version != INDEX_SCHEMA_VERSION {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Unsupported population accountant lineage anchor version".into()
        )));
    }
    hash_entry(anchor)
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    LineageAnchor(PopulationAccountantLineageAnchorV1),
}

#[hdk_link_types]
pub enum LinkTypes {
    LineageToStates,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(_) => invalid(
            "Population accountant lineage index has no committed entries; anchors are synthetic",
        ),
        FlatOp::RegisterUpdate(_) => invalid(
            "Population accountant lineage index entries cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Population accountant lineage index entries cannot be deleted",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Canonical population accountant admission links are append-only",
        ),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_link(
    link_type: LinkTypes,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    action: CreateLink,
) -> ExternResult<ValidateCallbackResult> {
    match link_type {
        LinkTypes::LineageToStates => {
            let base = base_address.into_entry_hash().ok_or(wasm_error!(
                WasmErrorInner::Guest(
                    "Population accountant admission link base must be an EntryHash".into()
                )
            ))?;
            let target = target_address.into_action_hash().ok_or(wasm_error!(
                WasmErrorInner::Guest(
                    "Population accountant admission link target must be an ActionHash".into()
                )
            ))?;

            let record = must_get_valid_record(target)?;
            let state: TrustedPopulationAccountantState =
                decode_entry(&record, "trusted population accountant state")?;
            let shape = state.projection.validate_shape()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }
            let anchor = PopulationAccountantLineageAnchorV1::from_state(&state)?;
            let expected = accountant_lineage_anchor_hash(&anchor)?;
            if base != expected {
                return invalid(
                    "Population accountant admission link base does not match state lineage",
                );
            }
            if record.action().author() != &action.author {
                return invalid(
                    "Population accountant admission link must be authored by state author",
                );
            }
            Ok(ValidateCallbackResult::Valid)
        }
    }
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

fn invalid(message: impl Into<String>) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain};

    fn stored(seed: u8) -> StoredDigest {
        StoredDigest {
            algorithm: DigestAlgorithm::Blake3_256,
            domain: DigestDomain::ClinicalArtifact,
            value: [seed; 32],
        }
    }

    fn policy(seed: u8) -> PopulationReleaseDigestV1 {
        PopulationReleaseDigestV1 {
            kind: PopulationReleaseArtifactKindV1::ReleasePolicy,
            value: [seed; 32],
        }
    }

    fn anchor() -> PopulationAccountantLineageAnchorV1 {
        PopulationAccountantLineageAnchorV1::new(policy(1), stored(2), stored(3)).unwrap()
    }

    #[test]
    fn same_lineage_has_same_anchor() {
        assert_eq!(
            accountant_lineage_anchor_hash(&anchor()).unwrap(),
            accountant_lineage_anchor_hash(&anchor()).unwrap()
        );
    }

    #[test]
    fn policy_is_part_of_anchor_identity() {
        let a = anchor();
        let b = PopulationAccountantLineageAnchorV1::new(policy(4), stored(2), stored(3)).unwrap();
        assert_ne!(
            accountant_lineage_anchor_hash(&a).unwrap(),
            accountant_lineage_anchor_hash(&b).unwrap()
        );
    }

    #[test]
    fn accountant_instance_is_part_of_anchor_identity() {
        let a = anchor();
        let b = PopulationAccountantLineageAnchorV1::new(policy(1), stored(4), stored(3)).unwrap();
        assert_ne!(
            accountant_lineage_anchor_hash(&a).unwrap(),
            accountant_lineage_anchor_hash(&b).unwrap()
        );
    }

    #[test]
    fn accountant_method_is_part_of_anchor_identity() {
        let a = anchor();
        let b = PopulationAccountantLineageAnchorV1::new(policy(1), stored(2), stored(4)).unwrap();
        assert_ne!(
            accountant_lineage_anchor_hash(&a).unwrap(),
            accountant_lineage_anchor_hash(&b).unwrap()
        );
    }

    #[test]
    fn wrong_policy_digest_kind_is_rejected() {
        let wrong = PopulationReleaseDigestV1 {
            kind: PopulationReleaseArtifactKindV1::ReleaseReceipt,
            value: [1; 32],
        };
        assert!(PopulationAccountantLineageAnchorV1::new(wrong, stored(2), stored(3)).is_err());
    }
}
