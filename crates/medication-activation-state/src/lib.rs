#![deny(unsafe_code)]
//! Provenance-preserving medication activation read model.
//!
//! `Prescription.status == Active` is not clinical qualification evidence. The
//! canonical reducer preserves qualified, emergency-override, terminated, conflict,
//! and legacy-unqualified provenance without flattening them to a boolean.
//!
//! Inputs must already have crossed the relevant Holochain integrity boundary; this
//! crate is a deterministic semantic reducer, not an alternative trust root.

mod reducer;

pub use reducer::*;
