#![deny(unsafe_code)]
//! Conflict-preserving read model for DNA-qualified causal attestations.
//!
//! Inputs are assumed to be DHT-valid and complete-for-purpose records supplied by
//! a trusted adapter. The reducer does not query the DHT and never uses last-write-wins.

mod v1;

pub use v1::*;
