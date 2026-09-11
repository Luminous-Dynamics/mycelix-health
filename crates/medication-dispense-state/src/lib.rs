#![deny(unsafe_code)]
//! Conflict-preserving finalized medication dispense read model.
//!
//! This crate reduces already DHT-validated medication-dispense records. It does
//! not establish Holochain validity itself, and it never treats absence from a
//! supplied read set as proof of global absence.
//!
//! The reducer preserves duplicate/idempotent publication, allocator equivocation,
//! incomplete predecessor history, and conflicting final receipts explicitly. It
//! never converts those conditions into a clean refill count by timestamp or
//! arrival order.

mod reducer;

pub use reducer::*;
