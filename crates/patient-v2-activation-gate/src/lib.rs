#![forbid(unsafe_code)]
//! Fail-closed reference semantics for Patient v2 activation qualification.
//!
//! The core separates evidence qualification, a write-blocking cutover freeze,
//! migration rehearsal, explicit activation authority, and actual activation.
//! It does not itself verify CI/signatures, perform migration, or activate a DNA.

mod core;

pub use core::*;
