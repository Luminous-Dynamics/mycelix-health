#![deny(unsafe_code)]
#![allow(clippy::collapsible_match)]
//! Canonical admission index for privacy-minimized causal attestations.
//!
//! A causal attestation may exist on the DHT without an admission link, but only
//! attestations admitted through this integrity boundary participate in the
//! high-assurance commitment-scoped read model. The deterministic link base is
//! derived solely from the attestation's already-public opaque receipt commitment.

use clinical_causality_integrity::{
    CausalCommitmentSchemeV1, OpaqueCausalCommitmentV1, QualifiedCausalAssessmentAttestation,
};
use hdi::prelude::*;

const INDEX_SCHEMA_VERSION: u16 = 1;

pub use clinical_causality_integrity::{
    CausalAttestationCorrection, QualifiedCausalAssessmentAttestationV1,
};

/// Synthetic anchor used only to derive a deterministic `EntryHash` link base.
/// The anchor entry itself is never required to be committed.
#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct CausalCommitmentIndexAnchorV1 {
    pub schema_version: u16,
    pub commitment: OpaqueCausalCommitmentV1,
}

impl CausalCommitmentIndexAnchorV1 {
    pub fn new(commitment: OpaqueCausalCommitmentV1) -> ExternResult<Self> {
        if commitment.value == [0u8; 32] {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Causal commitment index anchor cannot use a zero commitment".to_string()
            )));
        }
        Ok(Self {
            schema_version: INDEX_SCHEMA_VERSION,
            commitment,
        })
    }
}

/// Compute the canonical commitment-scoped publication index base.
pub fn commitment_index_anchor_hash(
    commitment: OpaqueCausalCommitmentV1,
) -> ExternResult<EntryHash> {
    hash_entry(&CausalCommitmentIndexAnchorV1::new(commitment)?)
}

/// Reserved synthetic entry type. V1 rejects committing it; only its deterministic
/// entry hash is used as a link base.
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    CommitmentIndexAnchor(CausalCommitmentIndexAnchorV1),
}

#[hdk_link_types]
pub enum LinkTypes {
    CommitmentToAttestations,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(_) => invalid(
            "Clinical causality commitment index has no committed entries; anchors are synthetic",
        ),
        FlatOp::RegisterUpdate(_) => invalid(
            "Clinical causality commitment index entries cannot be updated",
        ),
        FlatOp::RegisterDelete(_) => invalid(
            "Clinical causality commitment index entries cannot be deleted",
        ),
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            action,
            ..
        } => validate_create_link(link_type, base_address, target_address, action),
        FlatOp::RegisterDeleteLink { .. } => invalid(
            "Canonical causal commitment admission links are append-only and cannot be deleted",
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
        LinkTypes::CommitmentToAttestations => {
            let base = base_address.into_entry_hash().ok_or(wasm_error!(
                WasmErrorInner::Guest(
                    "Causal commitment admission link base must be an EntryHash".to_string()
                )
            ))?;
            let target = target_address.into_action_hash().ok_or(wasm_error!(
                WasmErrorInner::Guest(
                    "Causal commitment admission link target must be an ActionHash".to_string()
                )
            ))?;

            let record = must_get_valid_record(target)?;
            let attestation: QualifiedCausalAssessmentAttestation =
                decode_entry(&record, "qualified causal attestation")?;

            let shape = attestation.attestation.validate()?;
            if !matches!(shape, ValidateCallbackResult::Valid) {
                return Ok(shape);
            }

            let expected = commitment_index_anchor_hash(
                attestation.attestation.qualified_receipt_commitment,
            )?;
            if base != expected {
                return invalid(
                    "Causal commitment admission link base does not match target receipt commitment",
                );
            }

            // Canonical admission must be performed by the same verifier that authored
            // the target attestation. A third party cannot canonize someone else's claim.
            if record.action().author() != &action.author {
                return invalid(
                    "Causal commitment admission link must be authored by attestation author",
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

    fn commitment(seed: u8) -> OpaqueCausalCommitmentV1 {
        OpaqueCausalCommitmentV1 {
            scheme: CausalCommitmentSchemeV1::Blake3Keyed,
            value: [seed; 32],
        }
    }

    #[test]
    fn same_commitment_has_same_anchor() {
        assert_eq!(
            commitment_index_anchor_hash(commitment(7)).unwrap(),
            commitment_index_anchor_hash(commitment(7)).unwrap()
        );
    }

    #[test]
    fn different_commitment_changes_anchor() {
        assert_ne!(
            commitment_index_anchor_hash(commitment(7)).unwrap(),
            commitment_index_anchor_hash(commitment(8)).unwrap()
        );
    }

    #[test]
    fn commitment_scheme_is_part_of_anchor_identity() {
        let mut other = commitment(7);
        other.scheme = CausalCommitmentSchemeV1::HmacSha256;
        assert_ne!(
            commitment_index_anchor_hash(commitment(7)).unwrap(),
            commitment_index_anchor_hash(other).unwrap()
        );
    }

    #[test]
    fn zero_commitment_is_rejected() {
        let value = OpaqueCausalCommitmentV1 {
            scheme: CausalCommitmentSchemeV1::Blake3Keyed,
            value: [0u8; 32],
        };
        assert!(commitment_index_anchor_hash(value).is_err());
    }
}
