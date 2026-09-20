#![forbid(unsafe_code)]
//! Reference semantics for the final Patient-v2 cutover composition boundary.
//!
//! This crate composes already-verified lower-layer receipts. It does not perform
//! migration, activation, signature verification, trusted time, Holochain I/O,
//! hashing, or distributed transactions.

mod canonical;

pub use canonical::*;
