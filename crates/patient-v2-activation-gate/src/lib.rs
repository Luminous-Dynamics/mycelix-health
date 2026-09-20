#![forbid(unsafe_code)]
//! Fail-closed reference semantics for Patient v2 activation qualification.
//!
//! The crate separates evidence qualification, write-blocking cutover freeze,
//! migration rehearsal, explicit activation authority, time-bounded abort, and
//! actual activation. It does not itself verify external evidence/signatures,
//! perform migration, freeze production writes, or activate a Holochain DNA.

mod core;
mod recovery;

pub use recovery::*;
