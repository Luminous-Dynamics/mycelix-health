#![deny(unsafe_code)]
//! Computable medication-administration occurrence identities.
//!
//! Human/admin record IDs are audit metadata, not clinical occurrence identity.
//! V1 derives occurrence identity from exact medication intent, patient binding,
//! dosage instruction, and explicit schedule/PRN occurrence material while keeping
//! resolver provenance outside the clinical identity.

mod v1;

pub use v1::*;
