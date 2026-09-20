#![forbid(unsafe_code)]
//! Reference semantics for the final Patient-v2 cutover composition boundary.
//!
//! This crate does not implement migration, activation, signatures, trusted
//! time, Holochain persistence, or distributed atomicity. It freezes the
//! cross-receipt coherence and durable activation-commit theorem from
//! PATIENT-PRIV-007.

mod model;

pub use model::*;
