#![deny(unsafe_code)]
//! Policy-bound qualification for clinical causal assessments.
//!
//! The public v1 API accepts only a `PolicyBoundAuthorityPermit`; the earlier
//! caller-bound `AuthorityPermit` path is intentionally not exported.

mod v1;

pub use v1::*;
