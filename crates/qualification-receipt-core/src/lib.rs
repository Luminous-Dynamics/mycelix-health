#![forbid(unsafe_code)]
//! Reference semantics for governed qualification receipts.
//!
//! Cryptographic signature verification, trust-store loading, trusted time,
//! durable/distributed ledger persistence, and hash computation are adapter
//! theorems outside this crate.

mod model;

pub use model::*;
