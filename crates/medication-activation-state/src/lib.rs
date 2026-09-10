#![deny(unsafe_code)]
//! Provenance-preserving medication activation read model.
//!
//! `Prescription.status == Active` is not clinical qualification evidence. The
//! canonical reducer preserves qualified, emergency-override, terminated, conflict,
//! and legacy-unqualified provenance without flattening them to a boolean.

mod reducer;

pub use reducer::*;
