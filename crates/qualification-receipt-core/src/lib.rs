#![forbid(unsafe_code)]
//! Reference semantics for governed qualification receipts.
//!
//! Cryptographic signature verification, trust-store loading, trusted time,
//! durable/distributed ledger persistence, and hash computation are adapter
//! theorems outside this crate.

// The low-level model intentionally contains adapter-only helpers that are not
// part of the external crate API. The strict module is the only public surface.
#[allow(dead_code)]
mod model;
mod strict;

pub use strict::*;
