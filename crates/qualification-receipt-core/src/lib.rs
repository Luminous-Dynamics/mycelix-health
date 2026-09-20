#![forbid(unsafe_code)]
//! Reference semantics for governed qualification receipts.
//!
//! Cryptographic signature verification, trust-store loading, trusted time,
//! durable/distributed ledger persistence, and hash computation are adapter
//! theorems outside this crate.

// The low-level model and governed verifier remain crate-private. Only the
// semantic-scope-bound wrapper is externally exported.
#[allow(dead_code)]
mod model;
mod strict;
mod scoped;

pub use scoped::*;
