// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent qualification checks for procurement allocation proposals.
//!
//! This module intentionally does not call the allocator or consume its
//! per-offer eligibility evaluations. It recomputes the award gates and costs.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlanViolationV1 {
    InvalidPolicy,
    SchemaVersionMismatch,
    AllocationMethodMismatch,
    ManifestMismatch,
    PolicyDigestMismatch,
    CurrencyMismatch,
    UnitBasisMismatch,
    DemandMismatch,
    DuplicateSourceOfferIdentity,
    DuplicateAllocation,
    UnknownOffer,
    AllocatedOfferIneligible,
    AllocationSupplierMismatch,
    AllocationBelowMinimumOrder,
    OfferCapacityExceeded,
    SupplierConcentrationExceeded,
    SupplierCountBelowMinimum,
    AllocatedUnitsMismatch,
    UnmetDemandMismatch,
    UnitCostMismatch,
    ExtendedCostMismatch,
    TotalCostMismatch,
    ExpectedReliabilityMismatch,
    PlanStatusMismatch,
    ArithmeticOverflow,
}

/// Verify that a proposed allocation is safe to consider for an authorized
/// human procurement review. A non-empty violation list means the plan must not
/// be treated as a qualified allocation. This does not itself grant award or
/// purchase authority.
pub fn verify_plan_v1(
    policy: &ProcurementPolicyV1,
    offers: &[SupplyOfferV1],
    plan: &ProcurementPlanV1,
) -> Vec<PlanViolationV1> {
    let mut violations = Vec::new();
    if validate_policy(policy).is_err() {
        return vec![PlanViolationV1::InvalidPolicy];
    }
    if plan.schema_version != SCHEMA_VERSION {
        violations.push(PlanViolationV1::SchemaVersionMismatch);
    }
    if plan.allocation_method != ALLOCATION_METHOD {
        violations.push(PlanViolationV1::AllocationMethodMismatch);
    }
    if plan.input_manifest_sha256 != policy.input_manifest_sha256 {
        violations.push(PlanViolationV1::ManifestMismatch);
    }
    if plan.policy_sha256 != policy.policy_sha256 {
        violations.push(PlanViolationV1::PolicyDigestMismatch);
    }
    if plan.currency_code != policy.currency_code {
        violations.push(PlanViolationV1::CurrencyMismatch);
    }
    if plan.normalized_unit_basis != policy.normalized_unit_basis {
        violations.push(PlanViolationV1::UnitBasisMismatch);
    }
    if plan.demand_units != policy.demand_units {
        violations.push(PlanViolationV1::DemandMismatch);
    }

    let mut offer_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for offer in offers {
        *offer_counts.entry(offer.offer_id.as_str()).or_default() += 1;
    }
    let unique_offers: BTreeMap<&str, &SupplyOfferV1> = offers
        .iter()
        .filter(|offer| offer_counts.get(offer.offer_id.as_str()) == Some(&1))
        .map(|offer| (offer.offer_id.as_str(), offer))
        .collect();
    if offer_counts.values().any(|count| *count > 1) {
        violations.push(PlanViolationV1::DuplicateSourceOfferIdentity);
    }

    let mut allocation_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for allocation in &plan.allocations {
        *allocation_counts.entry(allocation.offer_id.as_str()).or_default() += 1;
    }
    if allocation_counts.values().any(|count| *count > 1) {
        violations.push(PlanViolationV1::DuplicateAllocation);
    }

    let supplier_cap = ((policy.demand_units as u128)
        * (policy.maximum_supplier_concentration_basis_points as u128)
        / BASIS_POINTS) as u64;
    let mut supplier_units: BTreeMap<String, u64> = BTreeMap::new();
    let mut recomputed_units = 0_u64;
    let mut recomputed_total_cost = 0_u128;
    let mut all_reliability_known = true;
    let mut reliability_weight = 0_u128;
    let mut cost_computation_complete = true;

    for allocation in &plan.allocations {
        let Some(offer) = unique_offers.get(allocation.offer_id.as_str()).copied() else {
            violations.push(PlanViolationV1::UnknownOffer);
            cost_computation_complete = false;
            continue;
        };

        if !award_eligible_independently(policy, offer) {
            violations.push(PlanViolationV1::AllocatedOfferIneligible);
        }
        if allocation.supplier_id != offer.supplier_id {
            violations.push(PlanViolationV1::AllocationSupplierMismatch);
        }
        if allocation.allocated_units == 0
            || allocation.allocated_units < offer.minimum_order_units
        {
            violations.push(PlanViolationV1::AllocationBelowMinimumOrder);
        }
        if allocation.allocated_units > offer.available_units {
            violations.push(PlanViolationV1::OfferCapacityExceeded);
        }

        let recalculated_unit_cost = match offer.unit_cost.total_minor() {
            Some(value) => value,
            None => {
                violations.push(PlanViolationV1::ArithmeticOverflow);
                cost_computation_complete = false;
                continue;
            }
        };
        if allocation.unit_total_cost_minor != recalculated_unit_cost {
            violations.push(PlanViolationV1::UnitCostMismatch);
        }
        let recalculated_extended = match (allocation.allocated_units as u128)
            .checked_mul(recalculated_unit_cost as u128)
        {
            Some(value) => value,
            None => {
                violations.push(PlanViolationV1::ArithmeticOverflow);
                cost_computation_complete = false;
                continue;
            }
        };
        if allocation.extended_cost_minor != recalculated_extended {
            violations.push(PlanViolationV1::ExtendedCostMismatch);
        }
        recomputed_total_cost = match recomputed_total_cost.checked_add(recalculated_extended) {
            Some(value) => value,
            None => {
                violations.push(PlanViolationV1::ArithmeticOverflow);
                cost_computation_complete = false;
                continue;
            }
        };
        recomputed_units = match recomputed_units.checked_add(allocation.allocated_units) {
            Some(value) => value,
            None => {
                violations.push(PlanViolationV1::ArithmeticOverflow);
                continue;
            }
        };
        *supplier_units.entry(offer.supplier_id.clone()).or_default() +=
            allocation.allocated_units;

        if let Some(rate) = offer.on_time_delivery_basis_points {
            if rate <= 10_000 {
                reliability_weight +=
                    (allocation.allocated_units as u128) * (rate as u128);
            } else {
                all_reliability_known = false;
            }
        } else {
            all_reliability_known = false;
        }
    }

    for units in supplier_units.values() {
        if *units > supplier_cap {
            violations.push(PlanViolationV1::SupplierConcentrationExceeded);
        }
    }
    if supplier_units.len() < policy.minimum_supplier_count {
        violations.push(PlanViolationV1::SupplierCountBelowMinimum);
    }
    if recomputed_units != plan.allocated_units {
        violations.push(PlanViolationV1::AllocatedUnitsMismatch);
    }
    if plan.unmet_demand_units != policy.demand_units.saturating_sub(recomputed_units) {
        violations.push(PlanViolationV1::UnmetDemandMismatch);
    }
    if recomputed_units > policy.demand_units {
        violations.push(PlanViolationV1::DemandMismatch);
    }
    if cost_computation_complete && recomputed_total_cost != plan.total_allocated_cost_minor {
        violations.push(PlanViolationV1::TotalCostMismatch);
    }

    let expected_reliability = if all_reliability_known && recomputed_units > 0 {
        Some((reliability_weight / recomputed_units as u128).min(BASIS_POINTS) as u16)
    } else {
        None
    };
    if plan.expected_on_time_delivery_basis_points != expected_reliability {
        violations.push(PlanViolationV1::ExpectedReliabilityMismatch);
    }

    let expected_status = if recomputed_units == policy.demand_units
        && supplier_units.len() >= policy.minimum_supplier_count
        && !violations.contains(&PlanViolationV1::ArithmeticOverflow)
    {
        PlanStatusV1::FeasibleWithinObservedOffers
    } else if violations.contains(&PlanViolationV1::ArithmeticOverflow) {
        PlanStatusV1::ArithmeticOverflow
    } else {
        PlanStatusV1::InfeasibleWithinObservedOffers
    };
    if plan.status != expected_status {
        violations.push(PlanViolationV1::PlanStatusMismatch);
    }
    violations.sort();
    violations.dedup();
    violations
}

fn award_eligible_independently(
    policy: &ProcurementPolicyV1,
    offer: &SupplyOfferV1,
) -> bool {
    if offer.unit_cost.total_minor().is_none() {
        return false;
    }
    let date_is_current = is_valid_date(offer.price_observed_yyyymmdd)
        && is_valid_date(offer.price_valid_until_yyyymmdd)
        && offer.price_observed_yyyymmdd <= policy.as_of_yyyymmdd
        && offer.price_valid_until_yyyymmdd >= policy.as_of_yyyymmdd
        && offer.price_valid_until_yyyymmdd >= offer.price_observed_yyyymmdd;
    let evidence_digests_are_valid = [
        offer.price_observation_sha256.as_str(),
        offer.quality_evidence_sha256.as_str(),
        offer.authorization_evidence_sha256.as_str(),
        offer.licence_evidence_sha256.as_str(),
        offer.capacity_evidence_sha256.as_str(),
    ]
    .into_iter()
    .all(is_sha256_hex);
    let delivery_evidence_is_acceptable = match (
        offer.on_time_delivery_basis_points,
        offer.delivery_observation_count,
        offer.delivery_evidence_sha256.as_deref(),
    ) {
        (Some(rate), count, Some(digest)) => {
            rate <= 10_000
                && count >= policy.minimum_delivery_observations
                && count > 0
                && is_sha256_hex(digest)
        }
        (None, 0, None) => policy.minimum_delivery_observations == 0,
        _ => false,
    };

    is_identifier(&offer.offer_id)
        && is_identifier(&offer.supplier_id)
        && offer.medicine_presentation_id == policy.medicine_presentation_id
        && offer.jurisdiction == policy.jurisdiction
        && offer.currency_code == policy.currency_code
        && offer.unit_basis == policy.normalized_unit_basis
        && date_is_current
        && evidence_digests_are_valid
        && offer.source_reuse == SourceReuseState::Permitted
        && offer.quality_state == QualityEvidenceState::Qualified
        && offer.authorization_state == MarketAuthorizationState::Authorized
        && offer.licence_state == LicenceTerritoryState::Permitted
        && offer.available_units > 0
        && offer.minimum_order_units > 0
        && offer.minimum_order_units <= offer.available_units
        && offer.minimum_order_units
            <= ((policy.demand_units as u128)
                * policy.maximum_supplier_concentration_basis_points as u128
                / BASIS_POINTS) as u64
        && offer.lead_time_days <= policy.maximum_lead_time_days
        && offer.shelf_life_days_at_delivery >= policy.minimum_shelf_life_days_at_delivery
        && delivery_evidence_is_acceptable
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn policy() -> ProcurementPolicyV1 {
        ProcurementPolicyV1 {
            medicine_presentation_id: "med-amoxicillin-500mg-capsule-21".to_string(),
            jurisdiction: "ZA".to_string(),
            currency_code: "ZAR".to_string(),
            normalized_unit_basis: "capsule".to_string(),
            as_of_yyyymmdd: 20261013,
            demand_units: 100,
            maximum_lead_time_days: 30,
            minimum_shelf_life_days_at_delivery: 180,
            maximum_supplier_concentration_basis_points: 7000,
            minimum_supplier_count: 2,
            minimum_delivery_observations: 2,
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
            price_observation_sha256: DIGEST_A.to_string(),
            quality_evidence_sha256: DIGEST_A.to_string(),
            authorization_evidence_sha256: DIGEST_A.to_string(),
            licence_evidence_sha256: DIGEST_A.to_string(),
            capacity_evidence_sha256: DIGEST_A.to_string(),
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
    fn independent_checker_accepts_generated_feasible_plan() {
        let policy = policy();
        let offers = vec![
            offer("offer-b", "supplier-b", 20, 100),
            offer("offer-a", "supplier-a", 10, 100),
        ];
        let plan = plan_procurement_v1(&policy, &offers).expect("policy");
        assert!(verify_plan_v1(&policy, &offers, &plan).is_empty());
    }

    #[test]
    fn independent_checker_rejects_manually_inserted_unqualified_offer() {
        let policy = policy();
        let mut unsafe_offer = offer("offer-low", "supplier-low", 1, 100);
        unsafe_offer.quality_state = QualityEvidenceState::Rejected;
        let offers = vec![
            offer("offer-a", "supplier-a", 10, 100),
            offer("offer-b", "supplier-b", 20, 100),
            unsafe_offer.clone(),
        ];
        let mut plan = plan_procurement_v1(&policy, &offers).expect("policy");
        let index = plan
            .allocations
            .iter()
            .position(|allocation| allocation.offer_id == "offer-a")
            .expect("eligible allocation");
        let units = plan.allocations[index].allocated_units;
        let low_unit_cost = unsafe_offer.unit_cost.total_minor().expect("valid cost");
        plan.allocations[index].offer_id = unsafe_offer.offer_id.clone();
        plan.allocations[index].supplier_id = unsafe_offer.supplier_id.clone();
        plan.allocations[index].unit_total_cost_minor = low_unit_cost;
        plan.allocations[index].extended_cost_minor = (units as u128) * (low_unit_cost as u128);
        plan.allocations
            .sort_by(|left, right| left.offer_id.cmp(&right.offer_id));
        plan.total_allocated_cost_minor = plan
            .allocations
            .iter()
            .map(|allocation| allocation.extended_cost_minor)
            .sum();

        let violations = verify_plan_v1(&policy, &offers, &plan);
        assert!(violations.contains(&PlanViolationV1::AllocatedOfferIneligible));
    }

    #[test]
    fn independent_checker_detects_modified_cost_receipt() {
        let policy = policy();
        let offers = vec![
            offer("offer-a", "supplier-a", 10, 100),
            offer("offer-b", "supplier-b", 20, 100),
        ];
        let mut plan = plan_procurement_v1(&policy, &offers).expect("policy");
        plan.total_allocated_cost_minor += 1;
        let violations = verify_plan_v1(&policy, &offers, &plan);
        assert!(violations.contains(&PlanViolationV1::TotalCostMismatch));
    }
}
