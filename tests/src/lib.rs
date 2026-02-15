//! Mycelix-Health Test Suite
//!
//! Comprehensive tests for all healthcare zomes including:
//! - Unit tests for entry validation
//! - Integration tests for zome interactions
//! - HIPAA compliance verification tests
//! - Clinical trials FDA compliance tests
//! - Cross-hApp bridge protocol tests
//! - Byzantine fault tolerance tests
//! - Access control enforcement tests

pub mod patient;
pub mod provider;
pub mod records;
pub mod prescriptions;
pub mod consent;
pub mod trials;
pub mod insurance;
pub mod bridge;
pub mod hipaa_compliance;
pub mod byzantine;
pub mod access_control;
pub mod delegation;
pub mod notifications;
pub mod care_teams;

// Revolutionary Features (Phase 2)
pub mod advocate;     // AI Health Advocate
pub mod zkhealth;     // ZK Health Proofs
pub mod twin;         // Health Twin MVP
pub mod dividends;    // Data Dividends

// Differential Privacy Testing (Phase 3)
pub mod dp_property_tests;  // Property-based DP tests
