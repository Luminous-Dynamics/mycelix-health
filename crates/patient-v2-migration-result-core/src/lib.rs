#![forbid(unsafe_code)]
//! Reference semantics for binding one exact frozen Patient-v1 source lineage to
//! one verified protected-v2 destination lineage.
//!
//! This crate does not perform migration, encryption, Holochain I/O, signature
//! verification, cleanup, or activation. It freezes the evidence/replay/abort
//! theorem required by PATIENT-PRIV-006 before a production activation gate may
//! accept a migration result.

mod model;
mod strict;

pub use strict::*;
