#![forbid(unsafe_code)]
//! Fail-closed reference semantics for Patient v2 activation qualification.
//!
//! This crate does not inspect GitHub, run CI, verify signatures, hash WASM, or
//! activate a Holochain DNA. It freezes evidence aggregation, cutover freeze,
//! migration rehearsal, activation authority, and post-activation fail-closed
//! read/write semantics so individually valid artifacts cannot silently become
//! deployment authority.

mod model_v2;

pub use model_v2::*;
