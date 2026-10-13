// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, quality-gated procurement baseline.
//!
//! This crate allocates synthetic normalized medicine units across eligible
//! supplier offers. It is a transparent greedy baseline, not a global optimizer,
//! purchasing agent, regulatory authority, or clinical decision system.
//!
//! All digests are supplied by an upstream evidence/snapshot layer and are
//! syntax-checked only. This crate does not fetch, hash, authenticate, or verify
//! the artifacts those digests are supposed to identify.

use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u16 = 1;
pub const ALLOCATION_METHOD: &str = "DeterministicGreedyBaselineV1";
const BASIS_POINTS: u128 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityEvidenceState {
    Qualified,
    Unknown,
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketAuthorizationState {
    Authorized,
    Unknown,
    NotAuthorized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LicenceTerritoryState {
    Permitted,
    Unknown,
    Excluded,
}

/// Broad economic meaning of a price; source-specific basis is kept separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriceKindV1 {
    PublishedReferencePrice,
    SupplierQuote,
    AwardedContractPrice,
    TransactionPrice,
    ReimbursementPrice,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReuseState {
    Permitted,
    MetadataOnly,
    ReviewRequired,
    Prohibited,
}

/// Exact permitted purpose declared by an upstream, rights-reviewed manifest.
/// Variants are not hierarchical: permission for one does not imply another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceUseScopeV1 {
    NonCommercialReference,
    CommercialAnalysis,
    PublicDisplay,
    RawRedistribution,
}

/// Costs are per one normalized unit in the scenario currency's minor units.
/// Their provenance and whether they are observed vs scenario-estimated must
/// be documented in the caller's immutable manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitCostBreakdownV1 {
    pub acquisition_minor: u64,
    pub freight_minor: u64,
    pub insurance_minor: u64,
    pub duties_and_tax_minor: u64,
    pub quality_testing_minor: u64,
    pub storage_and_cold_chain_minor: u64,
    pub financing_minor: u64,
    pub expiry_and_wastage_minor: u64,
    pub expected_shortage_impact_minor: u64,
}

impl UnitCostBreakdownV1 {
    pub fn total_minor(&self) -> Option<u64> {
        [
            self.acquisition_minor,
            self.freight_minor,
            self.insurance_minor,
            self.duties_and_tax_minor,
            self.quality_testing_minor,
            self.storage_and_cold_chain_minor,
            self.financing_minor,
            self.expiry_and_wastage_minor,
            self.expected_shortage_impact_minor,
        ]
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcurementPolicyV1 {
    /// Exact product presentation identity; ingredient-only equivalence is not enough.
    pub medicine_presentation_id: String,
    pub jurisdiction: String,
    pub currency_code: String,
    /// Price meaning and exact source-defined basis to compare; unknown is invalid.
    pub required_price_kind: PriceKindV1,
    pub required_price_class_id: String,
    /// Exact unit basis, e.g. "tablet", "vial", or a versioned normalized pack unit.
    pub normalized_unit_basis: String,
    pub as_of_yyyymmdd: u32,
    pub demand_units: u64,
    pub maximum_lead_time_days: u16,
    pub minimum_shelf_life_days_at_delivery: u16,
    pub maximum_supplier_concentration_basis_points: u16,
    pub minimum_supplier_count: usize,
    /// Zero means reliability history is optional, but remains visible as a warning.
    pub minimum_delivery_observations: u32,
    /// Intended use the source permission must explicitly authorize.
    pub required_source_use_scope: SourceUseScopeV1,
    /// Supplied by the canonical evidence layer; checked for SHA-256 syntax only.
    pub input_manifest_sha256: String,
    pub policy_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplyOfferV1 {
    pub offer_id: String,
    pub supplier_id: String,
    pub medicine_presentation_id: String,
    pub jurisdiction: String,
    pub currency_code: String,
    pub unit_basis: String,
    pub price_kind: PriceKindV1,
    pub price_class_id: String,

    pub price_observation_sha256: String,
    pub quality_evidence_sha256: String,
    pub authorization_evidence_sha256: String,
    pub licence_evidence_sha256: String,
    pub capacity_evidence_sha256: String,
    pub source_permission_evidence_sha256: String,
    pub permitted_source_use_scopes: Vec<SourceUseScopeV1>,
    pub source_reuse: SourceReuseState,

    pub quality_state: QualityEvidenceState,
    pub authorization_state: MarketAuthorizationState,
    pub licence_state: LicenceTerritoryState,

    pub price_observed_yyyymmdd: u32,
    pub price_valid_until_yyyymmdd: u32,
    pub available_units: u64,
    pub minimum_order_units: u64,
    pub lead_time_days: u16,
    pub shelf_life_days_at_delivery: u16,

    pub unit_cost: UnitCostBreakdownV1,

    /// Basis points on-time delivery rate, only when backed by the evidence below.
    pub on_time_delivery_basis_points: Option<u16>,
    pub delivery_observation_count: u32,
    pub delivery_evidence_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OfferExclusionReason {
    InvalidIdentifier,
    InvalidEvidenceDigest,
    DuplicateOfferId,
    PresentationMismatch,
    JurisdictionMismatch,
    CurrencyMismatch,
    UnitBasisMismatch,
    InvalidDate,
    PriceObservationFromFuture,
    PriceObservationExpired,
    UnknownPriceClass,
    PriceClassMismatch,
    SourceReuseNotPermitted,
    SourceUseScopeNotPermitted,
    QualityNotQualified,
    QualityUnknown,
    MarketNotAuthorized,
    MarketAuthorizationUnknown,
    LicenceTerritoryExcluded,
    LicenceTerritoryUnknown,
    InvalidCapacity,
    InvalidMinimumOrder,
    LeadTimeExceedsPolicy,
    InsufficientShelfLife,
    CostArithmeticOverflow,
    DeliveryEvidenceInvalid,
    DeliveryEvidenceInsufficient,
    ConcentrationCapBelowMinimumOrder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OfferWarning {
    DeliveryReliabilityUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferEvaluationV1 {
    pub offer_id: String,
    pub eligible: bool,
    pub unit_total_cost_minor: Option<u64>,
    pub exclusion_reasons: Vec<OfferExclusionReason>,
    pub warnings: Vec<OfferWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplierAllocationV1 {
    pub offer_id: String,
    pub supplier_id: String,
    pub allocated_units: u64,
    pub unit_total_cost_minor: u64,
    pub extended_cost_minor: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStatusV1 {
    FeasibleWithinObservedOffers,
    InfeasibleWithinObservedOffers,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcurementPlanV1 {
    pub schema_version: u16,
    pub allocation_method: String,
    pub status: PlanStatusV1,
    pub input_manifest_sha256: String,
    pub policy_sha256: String,
    pub currency_code: String,
    pub normalized_unit_basis: String,
    pub demand_units: u64,
    pub allocated_units: u64,
    pub unmet_demand_units: u64,
    pub total_allocated_cost_minor: u128,
    pub expected_on_time_delivery_basis_points: Option<u16>,
    pub supplier_count: usize,
    pub minimum_supplier_count: usize,
    pub offer_evaluations: Vec<OfferEvaluationV1>,
    pub allocations: Vec<SupplierAllocationV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyError {
    InvalidIdentifier,
    InvalidCurrencyCode,
    InvalidDate,
    ZeroDemand,
    InvalidConcentrationPolicy,
    InvalidSupplierCount,
    InvalidPriceClass,
    InvalidManifestDigest,
    InvalidPolicyDigest,
}

struct Candidate {
    offer_index: usize,
    unit_cost_minor: u64,
}

pub fn plan_procurement_v1(
    policy: &ProcurementPolicyV1,
    offers: &[SupplyOfferV1],
) -> Result<ProcurementPlanV1, PolicyError> {
    validate_policy(policy)?;

    let concentration_cap = ((policy.demand_units as u128)
        .checked_mul(policy.maximum_supplier_concentration_basis_points as u128)
        .ok_or(PolicyError::InvalidConcentrationPolicy)?
        / BASIS_POINTS) as u64;

    let mut offer_id_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for offer in offers {
        *offer_id_counts.entry(&offer.offer_id).or_default() += 1;
    }

    let mut evaluations = Vec::with_capacity(offers.len());
    let mut candidates = Vec::new();

    for (index, offer) in offers.iter().enumerate() {
        let (mut evaluation, unit_cost) = evaluate_offer(policy, offer, concentration_cap);
        if offer_id_counts.get(offer.offer_id.as_str()).copied().unwrap_or(0) > 1 {
            evaluation.exclusion_reasons.push(OfferExclusionReason::DuplicateOfferId);
        }
        evaluation.exclusion_reasons.sort();
        evaluation.exclusion_reasons.dedup();
        evaluation.eligible = evaluation.exclusion_reasons.is_empty();

        if evaluation.eligible {
            if let Some(cost) = unit_cost {
                candidates.push(Candidate {
                    offer_index: index,
                    unit_cost_minor: cost,
                });
            }
        }
        evaluations.push(evaluation);
    }

    candidates.sort_by(|left, right| {
        left.unit_cost_minor
            .cmp(&right.unit_cost_minor)
            .then_with(|| {
                offers[left.offer_index]
                    .offer_id
                    .cmp(&offers[right.offer_index].offer_id)
            })
    });
    evaluations.sort_by(|left, right| {
        left.offer_id
            .cmp(&right.offer_id)
            .then_with(|| left.unit_total_cost_minor.cmp(&right.unit_total_cost_minor))
            .then_with(|| left.exclusion_reasons.cmp(&right.exclusion_reasons))
            .then_with(|| left.warnings.cmp(&right.warnings))
    });

    let mut allocations_by_offer: BTreeMap<usize, u64> = BTreeMap::new();
    let mut supplier_allocations: BTreeMap<String, u64> = BTreeMap::new();
    let mut allocated_units = 0_u64;

    // Seed the minimum number of distinct suppliers at each offer's MOQ.
    // This makes source-diversity requirements explicit instead of hoping that
    // a price-only greedy pass will happen to use more than one supplier.
    let mut seeded_suppliers = BTreeSet::new();
    for candidate in &candidates {
        if seeded_suppliers.len() >= policy.minimum_supplier_count {
            break;
        }
        let offer = &offers[candidate.offer_index];
        if seeded_suppliers.contains(&offer.supplier_id) {
            continue;
        }
        if offer.minimum_order_units > offer.available_units
            || offer.minimum_order_units > concentration_cap
            || offer.minimum_order_units > policy.demand_units.saturating_sub(allocated_units)
        {
            continue;
        }

        allocations_by_offer.insert(candidate.offer_index, offer.minimum_order_units);
        *supplier_allocations.entry(offer.supplier_id.clone()).or_default() +=
            offer.minimum_order_units;
        allocated_units += offer.minimum_order_units;
        seeded_suppliers.insert(offer.supplier_id.clone());
    }

    if seeded_suppliers.len() < policy.minimum_supplier_count {
        return Ok(build_plan(
            policy,
            offers,
            evaluations,
            allocations_by_offer,
            allocated_units,
            PlanStatusV1::InfeasibleWithinObservedOffers,
        ));
    }

    // Fill remaining demand in deterministic landed-cost order while respecting
    // offer capacity and aggregate supplier concentration.
    for candidate in &candidates {
        if allocated_units >= policy.demand_units {
            break;
        }
        let offer = &offers[candidate.offer_index];
        let current_offer_units = allocations_by_offer
            .get(&candidate.offer_index)
            .copied()
            .unwrap_or(0);
        let supplier_units = supplier_allocations
            .get(&offer.supplier_id)
            .copied()
            .unwrap_or(0);
        let remaining_demand = policy.demand_units - allocated_units;
        let remaining_offer_capacity = offer.available_units.saturating_sub(current_offer_units);
        let remaining_supplier_capacity = concentration_cap.saturating_sub(supplier_units);
        let addable = remaining_demand
            .min(remaining_offer_capacity)
            .min(remaining_supplier_capacity);

        if addable == 0 {
            continue;
        }
        if current_offer_units == 0 && addable < offer.minimum_order_units {
            continue;
        }

        *allocations_by_offer.entry(candidate.offer_index).or_default() += addable;
        *supplier_allocations.entry(offer.supplier_id.clone()).or_default() += addable;
        allocated_units += addable;
    }

    let status = if allocated_units == policy.demand_units
        && supplier_allocations.len() >= policy.minimum_supplier_count
    {
        PlanStatusV1::FeasibleWithinObservedOffers
    } else {
        PlanStatusV1::InfeasibleWithinObservedOffers
    };

    Ok(build_plan(
        policy,
        offers,
        evaluations,
        allocations_by_offer,
        allocated_units,
        status,
    ))
}

fn build_plan(
    policy: &ProcurementPolicyV1,
    offers: &[SupplyOfferV1],
    offer_evaluations: Vec<OfferEvaluationV1>,
    allocations_by_offer: BTreeMap<usize, u64>,
    allocated_units: u64,
    status: PlanStatusV1,
) -> ProcurementPlanV1 {
    let mut allocations = Vec::new();
    let mut total_cost_minor = 0_u128;
    let mut total_rate_weight = 0_u128;
    let mut rate_complete = true;
    let mut used_suppliers = BTreeSet::new();
    let mut arithmetic_overflow = false;

    for (index, units) in allocations_by_offer {
        if units == 0 {
            continue;
        }
        let offer = &offers[index];
        let unit_cost_minor = match offer.unit_cost.total_minor() {
            Some(cost) => cost,
            None => {
                arithmetic_overflow = true;
                continue;
            }
        };
        let extended_cost_minor = match (units as u128).checked_mul(unit_cost_minor as u128) {
            Some(cost) => cost,
            None => {
                arithmetic_overflow = true;
                continue;
            }
        };
        total_cost_minor = match total_cost_minor.checked_add(extended_cost_minor) {
            Some(total) => total,
            None => {
                arithmetic_overflow = true;
                continue;
            }
        };
        if let Some(rate) = offer.on_time_delivery_basis_points {
            total_rate_weight += (units as u128) * (rate as u128);
        } else {
            rate_complete = false;
        }
        used_suppliers.insert(offer.supplier_id.clone());
        allocations.push(SupplierAllocationV1 {
            offer_id: offer.offer_id.clone(),
            supplier_id: offer.supplier_id.clone(),
            allocated_units: units,
            unit_total_cost_minor,
            extended_cost_minor,
        });
    }

    allocations.sort_by(|left, right| left.offer_id.cmp(&right.offer_id));
    let unmet = policy.demand_units.saturating_sub(allocated_units);
    let status = if arithmetic_overflow {
        PlanStatusV1::ArithmeticOverflow
    } else {
        status
    };
    let expected_on_time_delivery_basis_points = if rate_complete && allocated_units > 0 {
        Some((total_rate_weight / allocated_units as u128).min(BASIS_POINTS) as u16)
    } else {
        None
    };

    ProcurementPlanV1 {
        schema_version: SCHEMA_VERSION,
        allocation_method: ALLOCATION_METHOD.to_string(),
        status,
        input_manifest_sha256: policy.input_manifest_sha256.clone(),
        policy_sha256: policy.policy_sha256.clone(),
        currency_code: policy.currency_code.clone(),
        normalized_unit_basis: policy.normalized_unit_basis.clone(),
        demand_units: policy.demand_units,
        allocated_units,
        unmet_demand_units: unmet,
        total_allocated_cost_minor: total_cost_minor,
        expected_on_time_delivery_basis_points,
        supplier_count: used_suppliers.len(),
        minimum_supplier_count: policy.minimum_supplier_count,
        offer_evaluations,
        allocations,
    }
}

fn evaluate_offer(
    policy: &ProcurementPolicyV1,
    offer: &SupplyOfferV1,
    concentration_cap: u64,
) -> (OfferEvaluationV1, Option<u64>) {
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();
    let unit_cost = offer.unit_cost.total_minor();

    for id in [
        offer.offer_id.as_str(),
        offer.supplier_id.as_str(),
        offer.medicine_presentation_id.as_str(),
        offer.jurisdiction.as_str(),
        offer.unit_basis.as_str(),
        offer.price_class_id.as_str(),
    ] {
        if !is_identifier(id) {
            reasons.push(OfferExclusionReason::InvalidIdentifier);
            break;
        }
    }

    if offer.medicine_presentation_id != policy.medicine_presentation_id {
        reasons.push(OfferExclusionReason::PresentationMismatch);
    }
    if offer.jurisdiction != policy.jurisdiction {
        reasons.push(OfferExclusionReason::JurisdictionMismatch);
    }
    if offer.currency_code != policy.currency_code {
        reasons.push(OfferExclusionReason::CurrencyMismatch);
    }
    if offer.price_kind == PriceKindV1::Unknown {
        reasons.push(OfferExclusionReason::UnknownPriceClass);
    } else if offer.price_kind != policy.required_price_kind
        || offer.price_class_id != policy.required_price_class_id
    {
        reasons.push(OfferExclusionReason::PriceClassMismatch);
    }
    if offer.unit_basis != policy.normalized_unit_basis {
        reasons.push(OfferExclusionReason::UnitBasisMismatch);
    }

    if !is_valid_date(offer.price_observed_yyyymmdd)
        || !is_valid_date(offer.price_valid_until_yyyymmdd)
        || offer.price_valid_until_yyyymmdd < offer.price_observed_yyyymmdd
    {
        reasons.push(OfferExclusionReason::InvalidDate);
    } else if offer.price_observed_yyyymmdd > policy.as_of_yyyymmdd {
        reasons.push(OfferExclusionReason::PriceObservationFromFuture);
    } else if offer.price_valid_until_yyyymmdd < policy.as_of_yyyymmdd {
        reasons.push(OfferExclusionReason::PriceObservationExpired);
    }

    if ![
        offer.price_observation_sha256.as_str(),
        offer.quality_evidence_sha256.as_str(),
        offer.authorization_evidence_sha256.as_str(),
        offer.licence_evidence_sha256.as_str(),
        offer.capacity_evidence_sha256.as_str(),
        offer.source_permission_evidence_sha256.as_str(),
    ]
    .into_iter()
    .all(is_sha256_hex)
    {
        reasons.push(OfferExclusionReason::InvalidEvidenceDigest);
    }

    if offer.source_reuse != SourceReuseState::Permitted {
        reasons.push(OfferExclusionReason::SourceReuseNotPermitted);
    }
    if !offer
        .permitted_source_use_scopes
        .contains(&policy.required_source_use_scope)
    {
        reasons.push(OfferExclusionReason::SourceUseScopeNotPermitted);
    }
    match offer.quality_state {
        QualityEvidenceState::Qualified => {}
        QualityEvidenceState::Unknown => reasons.push(OfferExclusionReason::QualityUnknown),
        QualityEvidenceState::Rejected => reasons.push(OfferExclusionReason::QualityNotQualified),
    }
    match offer.authorization_state {
        MarketAuthorizationState::Authorized => {}
        MarketAuthorizationState::Unknown => {
            reasons.push(OfferExclusionReason::MarketAuthorizationUnknown)
        }
        MarketAuthorizationState::NotAuthorized => {
            reasons.push(OfferExclusionReason::MarketNotAuthorized)
        }
    }
    match offer.licence_state {
        LicenceTerritoryState::Permitted => {}
        LicenceTerritoryState::Unknown => {
            reasons.push(OfferExclusionReason::LicenceTerritoryUnknown)
        }
        LicenceTerritoryState::Excluded => {
            reasons.push(OfferExclusionReason::LicenceTerritoryExcluded)
        }
    }

    if offer.available_units == 0 {
        reasons.push(OfferExclusionReason::InvalidCapacity);
    }
    if offer.minimum_order_units == 0 || offer.minimum_order_units > offer.available_units {
        reasons.push(OfferExclusionReason::InvalidMinimumOrder);
    }
    if offer.lead_time_days > policy.maximum_lead_time_days {
        reasons.push(OfferExclusionReason::LeadTimeExceedsPolicy);
    }
    if offer.shelf_life_days_at_delivery < policy.minimum_shelf_life_days_at_delivery {
        reasons.push(OfferExclusionReason::InsufficientShelfLife);
    }
    if offer.minimum_order_units > concentration_cap {
        reasons.push(OfferExclusionReason::ConcentrationCapBelowMinimumOrder);
    }
    if unit_cost.is_none() {
        reasons.push(OfferExclusionReason::CostArithmeticOverflow);
    }

    match (
        offer.on_time_delivery_basis_points,
        offer.delivery_observation_count,
        offer.delivery_evidence_sha256.as_deref(),
    ) {
        (Some(rate), count, Some(digest))
            if rate <= 10_000 && count > 0 && is_sha256_hex(digest) =>
        {
            if count < policy.minimum_delivery_observations {
                reasons.push(OfferExclusionReason::DeliveryEvidenceInsufficient);
            }
        }
        (None, 0, None) => {
            if policy.minimum_delivery_observations > 0 {
                reasons.push(OfferExclusionReason::DeliveryEvidenceInsufficient);
            } else {
                warnings.push(OfferWarning::DeliveryReliabilityUnavailable);
            }
        }
        _ => reasons.push(OfferExclusionReason::DeliveryEvidenceInvalid),
    }

    reasons.sort();
    reasons.dedup();
    let eligible = reasons.is_empty();

    (
        OfferEvaluationV1 {
            offer_id: offer.offer_id.clone(),
            eligible,
            unit_total_cost_minor: unit_cost,
            exclusion_reasons: reasons,
            warnings,
        },
        if eligible { unit_cost } else { None },
    )
}

fn validate_policy(policy: &ProcurementPolicyV1) -> Result<(), PolicyError> {
    if !is_identifier(&policy.medicine_presentation_id)
        || !is_identifier(&policy.jurisdiction)
        || !is_identifier(&policy.normalized_unit_basis)
    {
        return Err(PolicyError::InvalidIdentifier);
    }
    if !is_currency_code(&policy.currency_code) {
        return Err(PolicyError::InvalidCurrencyCode);
    }
    if !is_valid_date(policy.as_of_yyyymmdd) {
        return Err(PolicyError::InvalidDate);
    }
    if policy.demand_units == 0 {
        return Err(PolicyError::ZeroDemand);
    }
    if policy.maximum_supplier_concentration_basis_points == 0
        || policy.maximum_supplier_concentration_basis_points > 10_000
    {
        return Err(PolicyError::InvalidConcentrationPolicy);
    }
    if policy.minimum_supplier_count == 0 {
        return Err(PolicyError::InvalidSupplierCount);
    }
    if policy.required_price_kind == PriceKindV1::Unknown
        || !is_identifier(&policy.required_price_class_id)
    {
        return Err(PolicyError::InvalidPriceClass);
    }
    if !is_sha256_hex(&policy.input_manifest_sha256) {
        return Err(PolicyError::InvalidManifestDigest);
    }
    if !is_sha256_hex(&policy.policy_sha256) {
        return Err(PolicyError::InvalidPolicyDigest);
    }
    Ok(())
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_valid_date(value: u32) -> bool {
    let year = value / 10_000;
    let month = (value / 100) % 100;
    let day = value % 100;
    if year == 0 || year > 9999 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=last_day).contains(&day)
}

mod source_contract;
pub use source_contract::{
    normalize_price_observation_v1, NormalizedPriceObservationV1, PriceTemporalStateV1,
    SourceContractRejectionV1, SourceIngestionPolicyV1, SourceListingStatusV1,
    SourceManifestV1, SourceOperationV1, SourcePermissionValidityV1, SourcePriceRecordV1,
    SourceRightsReviewStateV1,
};

mod verify;
pub use verify::{verify_plan_v1, PlanViolationV1};

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn policy() -> ProcurementPolicyV1 {
        ProcurementPolicyV1 {
            medicine_presentation_id: "med-amoxicillin-500mg-capsule-21".to_string(),
            jurisdiction: "ZA".to_string(),
            currency_code: "ZAR".to_string(),
            required_price_kind: PriceKindV1::SupplierQuote,
            required_price_class_id: "synthetic-supplier-quote-v1".to_string(),
            normalized_unit_basis: "capsule".to_string(),
            as_of_yyyymmdd: 20261013,
            demand_units: 100,
            maximum_lead_time_days: 30,
            minimum_shelf_life_days_at_delivery: 180,
            maximum_supplier_concentration_basis_points: 7000,
            minimum_supplier_count: 2,
            minimum_delivery_observations: 2,
            required_source_use_scope: SourceUseScopeV1::CommercialAnalysis,
            input_manifest_sha256: DIGEST_A.to_string(),
            policy_sha256: DIGEST_B.to_string(),
        }
    }

    fn offer(id: &str, supplier: &str, acquisition: u64, supply: u64) -> SupplyOfferV1 {
        SupplyOfferV1 {
            offer_id: id.to_string(),
            supplier_id: supplier.to_string(),
            medicine_presentation_id: "med-amoxicillin-500mg-capsule-21".to_string(),
            jurisdiction: "ZA".to_string(),
            currency_code: "ZAR".to_string(),
            unit_basis: "capsule".to_string(),
            price_kind: PriceKindV1::SupplierQuote,
            price_class_id: "synthetic-supplier-quote-v1".to_string(),
            price_observation_sha256: DIGEST_A.to_string(),
            quality_evidence_sha256: DIGEST_A.to_string(),
            authorization_evidence_sha256: DIGEST_A.to_string(),
            licence_evidence_sha256: DIGEST_A.to_string(),
            capacity_evidence_sha256: DIGEST_A.to_string(),
            source_permission_evidence_sha256: DIGEST_B.to_string(),
            permitted_source_use_scopes: vec![SourceUseScopeV1::CommercialAnalysis],
            source_reuse: SourceReuseState::Permitted,
            quality_state: QualityEvidenceState::Qualified,
            authorization_state: MarketAuthorizationState::Authorized,
            licence_state: LicenceTerritoryState::Permitted,
            price_observed_yyyymmdd: 20261001,
            price_valid_until_yyyymmdd: 20261231,
            available_units: supply,
            minimum_order_units: 10,
            lead_time_days: 10,
            shelf_life_days_at_delivery: 365,
            unit_cost: UnitCostBreakdownV1 {
                acquisition_minor: acquisition,
                freight_minor: 2,
                insurance_minor: 1,
                duties_and_tax_minor: 0,
                quality_testing_minor: 1,
                storage_and_cold_chain_minor: 0,
                financing_minor: 0,
                expiry_and_wastage_minor: 1,
                expected_shortage_impact_minor: 0,
            },
            on_time_delivery_basis_points: Some(9800),
            delivery_observation_count: 12,
            delivery_evidence_sha256: Some(DIGEST_B.to_string()),
        }
    }

    #[test]
    fn landed_cost_includes_all_declared_components() {
        let costs = UnitCostBreakdownV1 {
            acquisition_minor: 100,
            freight_minor: 2,
            insurance_minor: 3,
            duties_and_tax_minor: 4,
            quality_testing_minor: 5,
            storage_and_cold_chain_minor: 6,
            financing_minor: 7,
            expiry_and_wastage_minor: 8,
            expected_shortage_impact_minor: 9,
        };
        assert_eq!(costs.total_minor(), Some(144));
    }

    #[test]
    fn eligible_offers_fill_demand_using_cost_order_and_supplier_cap() {
        let policy = policy();
        let offers = vec![
            offer("offer-b", "supplier-b", 20, 100),
            offer("offer-a", "supplier-a", 10, 100),
        ];
        let plan = plan_procurement_v1(&policy, &offers).expect("valid policy");
        assert_eq!(plan.status, PlanStatusV1::FeasibleWithinObservedOffers);
        assert_eq!(plan.allocated_units, 100);
        assert_eq!(plan.unmet_demand_units, 0);
        assert_eq!(plan.supplier_count, 2);
        assert_eq!(plan.total_allocated_cost_minor, 70 * 15 + 30 * 25);
        assert!(plan.allocations.iter().all(|allocation| allocation.allocated_units <= 70));
    }

    #[test]
    fn low_price_never_overrides_failed_quality_gate() {
        let policy = policy();
        let mut cheaper = offer("offer-low", "supplier-low", 1, 100);
        cheaper.quality_state = QualityEvidenceState::Rejected;
        let offers = vec![
            cheaper,
            offer("offer-b", "supplier-b", 20, 100),
            offer("offer-c", "supplier-c", 21, 100),
        ];
        let plan = plan_procurement_v1(&policy, &offers).expect("valid policy");
        let rejected = plan.offer_evaluations.iter().find(|e| e.offer_id == "offer-low").unwrap();
        assert!(rejected.exclusion_reasons.contains(&OfferExclusionReason::QualityNotQualified));
        assert!(!plan.allocations.iter().any(|a| a.offer_id == "offer-low"));
        assert_eq!(plan.status, PlanStatusV1::FeasibleWithinObservedOffers);
    }

    #[test]
    fn market_authorization_and_licence_unknown_fail_closed() {
        let mut no_auth = offer("offer-a", "supplier-a", 10, 100);
        no_auth.authorization_state = MarketAuthorizationState::Unknown;
        let mut no_licence = offer("offer-b", "supplier-b", 11, 100);
        no_licence.licence_state = LicenceTerritoryState::Unknown;
        let plan = plan_procurement_v1(&policy(), &[no_auth, no_licence]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.allocations.is_empty());
    }

    #[test]
    fn mismatched_pack_or_unit_basis_is_not_comparable() {
        let mut wrong_pack = offer("offer-a", "supplier-a", 10, 100);
        wrong_pack.medicine_presentation_id = "med-amoxicillin-250mg-capsule-21".to_string();
        let mut wrong_basis = offer("offer-b", "supplier-b", 11, 100);
        wrong_basis.unit_basis = "pack-of-21".to_string();
        let plan = plan_procurement_v1(&policy(), &[wrong_pack, wrong_basis]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations.iter().all(|e| !e.eligible));
    }

    #[test]
    fn expired_price_observation_is_excluded() {
        let mut stale = offer("offer-a", "supplier-a", 1, 100);
        stale.price_valid_until_yyyymmdd = 20261012;
        let plan = plan_procurement_v1(&policy(), &[stale]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::PriceObservationExpired));
    }

    #[test]
    fn permission_for_noncommercial_reference_does_not_authorize_commercial_analysis() {
        let policy = policy();
        let mut reference_only = offer("offer-a", "supplier-a", 10, 100);
        reference_only.permitted_source_use_scopes =
            vec![SourceUseScopeV1::NonCommercialReference];
        let plan = plan_procurement_v1(&policy, &[reference_only]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::SourceUseScopeNotPermitted));
    }

    #[test]
    fn published_reference_price_is_not_eligible_as_a_supplier_quote() {
        let mut reference = offer("offer-a", "supplier-a", 1, 100);
        reference.price_kind = PriceKindV1::PublishedReferencePrice;
        reference.price_class_id = "za-ndoh.sep".to_string();
        let plan = plan_procurement_v1(&policy(), &[reference]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::PriceClassMismatch));
    }

    #[test]
    fn source_specific_price_classes_remain_distinct_within_reference_prices() {
        let mut policy = policy();
        policy.required_price_kind = PriceKindV1::PublishedReferencePrice;
        policy.required_price_class_id = "za-ndoh.ex-manufacturer".to_string();
        let mut sep = offer("offer-a", "supplier-a", 1, 100);
        sep.price_kind = PriceKindV1::PublishedReferencePrice;
        sep.price_class_id = "za-ndoh.sep".to_string();
        let plan = plan_procurement_v1(&policy, &[sep]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::PriceClassMismatch));
    }

    #[test]
    fn unknown_price_kind_is_rejected_by_policy_validation() {
        let mut policy = policy();
        policy.required_price_kind = PriceKindV1::Unknown;
        assert_eq!(
            plan_procurement_v1(&policy, &[]).unwrap_err(),
            PolicyError::InvalidPriceClass
        );
    }

    #[test]
    fn malformed_evidence_digest_has_a_specific_exclusion_reason() {
        let mut malformed = offer("offer-a", "supplier-a", 10, 100);
        malformed.quality_evidence_sha256 = "sha256-is-not-hex".to_string();
        let plan = plan_procurement_v1(&policy(), &[malformed]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::InvalidEvidenceDigest));
        assert!(!plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::InvalidIdentifier));
    }

    #[test]
    fn source_data_without_reuse_permission_is_not_used_as_price_evidence() {
        let mut restricted = offer("offer-a", "supplier-a", 1, 100);
        restricted.source_reuse = SourceReuseState::ReviewRequired;
        let plan = plan_procurement_v1(&policy(), &[restricted]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::SourceReuseNotPermitted));
    }

    #[test]
    fn missing_delivery_history_is_excluded_when_policy_requires_it() {
        let mut unknown = offer("offer-a", "supplier-a", 10, 100);
        unknown.on_time_delivery_basis_points = None;
        unknown.delivery_observation_count = 0;
        unknown.delivery_evidence_sha256 = None;
        let plan = plan_procurement_v1(&policy(), &[unknown]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::DeliveryEvidenceInsufficient));
    }

    #[test]
    fn missing_delivery_history_can_be_used_only_with_explicit_optional_policy() {
        let mut policy = policy();
        policy.minimum_delivery_observations = 0;
        policy.minimum_supplier_count = 1;
        policy.maximum_supplier_concentration_basis_points = 10_000;
        let mut unknown = offer("offer-a", "supplier-a", 10, 100);
        unknown.on_time_delivery_basis_points = None;
        unknown.delivery_observation_count = 0;
        unknown.delivery_evidence_sha256 = None;
        let plan = plan_procurement_v1(&policy, &[unknown]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::FeasibleWithinObservedOffers);
        assert_eq!(plan.expected_on_time_delivery_basis_points, None);
        assert!(plan.offer_evaluations[0]
            .warnings
            .contains(&OfferWarning::DeliveryReliabilityUnavailable));
    }

    #[test]
    fn duplicate_offer_ids_never_double_count_capacity() {
        let plan = plan_procurement_v1(
            &policy(),
            &[
                offer("same-offer", "supplier-a", 10, 100),
                offer("same-offer", "supplier-b", 11, 100),
            ],
        )
        .expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations.iter().all(|evaluation| {
            evaluation
                .exclusion_reasons
                .contains(&OfferExclusionReason::DuplicateOfferId)
        }));
        assert!(plan.allocations.is_empty());
    }

    #[test]
    fn cost_arithmetic_overflow_is_fail_closed() {
        let mut invalid = offer("offer-a", "supplier-a", u64::MAX, 100);
        invalid.unit_cost.freight_minor = 1;
        let plan = plan_procurement_v1(&policy(), &[invalid]).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan.offer_evaluations[0]
            .exclusion_reasons
            .contains(&OfferExclusionReason::CostArithmeticOverflow));
    }

    #[test]
    fn deterministic_output_does_not_depend_on_input_offer_order() {
        let offers = vec![
            offer("offer-c", "supplier-c", 30, 100),
            offer("offer-a", "supplier-a", 10, 100),
            offer("offer-b", "supplier-b", 20, 100),
        ];
        let first = plan_procurement_v1(&policy(), &offers).expect("policy");
        let mut reversed = offers;
        reversed.reverse();
        let second = plan_procurement_v1(&policy(), &reversed).expect("policy");
        assert_eq!(first, second);
    }

    #[test]
    fn inadequate_supply_reports_unmet_demand_instead_of_zero_cost_success() {
        let offers = vec![
            offer("offer-a", "supplier-a", 10, 20),
            offer("offer-b", "supplier-b", 20, 20),
        ];
        let plan = plan_procurement_v1(&policy(), &offers).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert_eq!(plan.allocated_units, 40);
        assert_eq!(plan.unmet_demand_units, 60);
        assert!(plan.total_allocated_cost_minor > 0);
    }

    #[test]
    fn excessive_concentration_makes_plan_infeasible() {
        let mut policy = policy();
        policy.maximum_supplier_concentration_basis_points = 5000;
        policy.minimum_supplier_count = 2;
        let offers = vec![
            offer("offer-a", "supplier-a", 10, 100),
            offer("offer-b", "supplier-b", 20, 20),
        ];
        let plan = plan_procurement_v1(&policy, &offers).expect("policy");
        assert_eq!(plan.status, PlanStatusV1::InfeasibleWithinObservedOffers);
        assert!(plan
            .allocations
            .iter()
            .all(|allocation| allocation.allocated_units <= 50));
    }

    #[test]
    fn invalid_policy_digests_are_rejected() {
        let mut policy = policy();
        policy.policy_sha256 = "not-a-digest".to_string();
        assert_eq!(
            plan_procurement_v1(&policy, &[]).unwrap_err(),
            PolicyError::InvalidPolicyDigest
        );
    }

    #[test]
    fn invalid_calendar_dates_are_rejected() {
        let mut policy = policy();
        policy.as_of_yyyymmdd = 20260230;
        assert_eq!(
            plan_procurement_v1(&policy, &[]).unwrap_err(),
            PolicyError::InvalidDate
        );
    }
}
