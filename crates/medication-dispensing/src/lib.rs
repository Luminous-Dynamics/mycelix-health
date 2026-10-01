#![deny(unsafe_code)]
//! High-assurance medication dispensing preflight.
//!
//! This crate deliberately stops before physical medication release. A successful
//! preflight proves that the exact medication artifact, current activation provenance,
//! pharmacy context, professional `Dispense` authority, requested full-fill semantics,
//! and locally observed prior finalized dispenses were coherent under the selected
//! policy. It does **not** prove exclusive refill-slot allocation across concurrent or
//! partitioned writers.
//!
//! Final dispensing requires a separate runtime adjudication proof before a
//! `MedicationDispenseReceipt` may exist.

mod preflight;

pub use preflight::*;
