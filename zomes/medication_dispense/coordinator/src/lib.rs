#![deny(unsafe_code)]
//! Thin coordinator for medication dispense slot adjudication.
//!
//! Authorization lives in the integrity zome. These functions intentionally do not
//! duplicate trust checks that a modified coordinator could bypass.

use hdk::prelude::*;
pub use medication_dispense_integrity::*;

#[hdk_extern]
pub fn issue_dispense_slot_authorization(
    authorization: MedicationDispenseSlotAuthorization,
) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::MedicationDispenseSlotAuthorization(
        authorization,
    ))
}

#[hdk_extern]
pub fn finalize_medication_dispense(
    finalized: FinalizedMedicationDispense,
) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::FinalizedMedicationDispense(finalized))
}

#[hdk_extern]
pub fn report_allocator_equivocation(
    evidence: AllocatorEquivocationEvidence,
) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::AllocatorEquivocationEvidence(evidence))
}
