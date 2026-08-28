// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Privacy-preserving aggregate public-health surveillance evidence contracts.
//!
//! This crate is deliberately **not** a diagnostic system, outbreak detector, or
//! action-authority layer. It defines the evidence envelope consumed by later
//! Mycelix/Symthaea components while keeping several distinctions explicit:
//!
//! - aggregate observation vs individual clinical record;
//! - measurement uncertainty vs source independence;
//! - content identity vs authenticity/trust;
//! - evidence vs hypothesis vs operational decision.
//!
//! The v1 wire contract contains no patient identifier, exact address, latitude,
//! longitude, raw genome, pathogen sequence, or treatment recommendation field.

mod identity;
mod types;

pub use identity::{ObservationId, OBSERVATION_ID_DOMAIN_V1};
pub use types::{
    BoundedUncertainty, CanonicalId, EvidenceProvenance, GeographicPrecision,
    GeographicScope, IndependenceGroup, MetricKind, ObservationWindow, ObservedMetric,
    SignalFamily, SourceKind, SurveillanceError, SurveillanceObservation,
    SURVEILLANCE_SCHEMA_V1,
};
