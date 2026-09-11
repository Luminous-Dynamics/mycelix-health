#![deny(unsafe_code)]
//! Evidence-first clinical causal assessment primitives.
//!
//! Observation, temporal association, and causal attribution are separate evidence
//! layers. See [`v1`] for the experimental v1 contract.

mod v1;

pub use v1::*;
