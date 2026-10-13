// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reproducible synthetic comparison across 12 presentations, 3 hypothetical
//! regions and 6 declared stress cases (216 scenario rows total).
//!
//! All identifiers, costs, quantities, and reliability observations are
//! fictional. Currency fields are synthetic ZAR minor units, not market prices.
//! The imported-cost stress is an explicit upstream scenario shock; this code
//! does not perform FX conversion or claim to model actual exchange rates.
//! Digest values are syntactically valid sentinels, not cryptographic evidence
//! for the in-memory fixture. The core deliberately does not authenticate them.

use medicine_procurement_core::{
    plan_procurement_v1, verify_plan_v1, LicenceTerritoryState, MarketAuthorizationState,
    ProcurementPlanV1, ProcurementPolicyV1, QualityEvidenceState, SourceReuseState,
    SourceUseScopeV1, SupplyOfferV1, UnitCostBreakdownV1,
};

const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DEMAND_UNITS: u64 = 100;
const EXPECTED_ROWS: usize = 12 * 3 * 6;

#[derive(Clone, Copy, Debug)]
enum StressCase {
    Baseline,
    SupplierFailure,
    FreightShock50Pct,
    ExpiryWastageShock,
    EvidenceLoss,
    ImportedCostShock20Pct,
}

impl StressCase {
    const ALL: [Self; 6] = [
        Self::Baseline,
        Self::SupplierFailure,
        Self::FreightShock50Pct,
        Self::ExpiryWastageShock,
        Self::EvidenceLoss,
        Self::ImportedCostShock20Pct,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::SupplierFailure => "supplier_1_failure",
            Self::FreightShock50Pct => "freight_plus_50pct",
            Self::ExpiryWastageShock => "expiry_wastage_plus_20_minor",
            Self::EvidenceLoss => "supplier_1_quality_evidence_lost",
            Self::ImportedCostShock20Pct => "imported_cost_plus_20pct_proxy",
        }
    }
}

const REGIONS: [&str; 3] = ["SIM-NORTH", "SIM-CENTRAL", "SIM-COAST"];

fn policy(
    medicine_id: &str,
    region: &str,
    minimum_supplier_count: usize,
    concentration_basis_points: u16,
    policy_digest: &str,
) -> ProcurementPolicyV1 {
    ProcurementPolicyV1 {
        medicine_presentation_id: medicine_id.to_string(),
        jurisdiction: region.to_string(),
        currency_code: "ZAR".to_string(),
        normalized_unit_basis: format!("synthetic-unit-{medicine_id}"),
        as_of_yyyymmdd: 20261013,
        demand_units: DEMAND_UNITS,
        maximum_lead_time_days: 30,
        minimum_shelf_life_days_at_delivery: 180,
        maximum_supplier_concentration_basis_points: concentration_basis_points,
        minimum_supplier_count,
        minimum_delivery_observations: 2,
        required_source_use_scope: SourceUseScopeV1::CommercialAnalysis,
        input_manifest_sha256: DIGEST_A.to_string(),
        policy_sha256: policy_digest.to_string(),
    }
}

fn offers_for(
    medicine_number: usize,
    region_number: usize,
    medicine_id: &str,
    region: &str,
    stress: StressCase,
) -> Vec<SupplyOfferV1> {
    let supplier_ids = [
        "supplier-00-unqualified",
        "supplier-01-local",
        "supplier-02-regional",
        "supplier-03-reserve",
    ];
    let unit_basis = format!("synthetic-unit-{medicine_id}");
    let eligible_base = 40 + (medicine_number as u64 * 2) + region_number as u64;
    let mut offers = Vec::with_capacity(supplier_ids.len());

    for (supplier_number, supplier_id) in supplier_ids.iter().enumerate() {
        let acquisition = match supplier_number {
            0 => 1, // Intentionally tempting price; quality evidence is unknown.
            1 => eligible_base,
            2 => eligible_base + 12,
            _ => eligible_base + 26,
        };
        let mut costs = UnitCostBreakdownV1 {
            acquisition_minor: acquisition,
            freight_minor: 3 + (supplier_number as u64 * 2) + region_number as u64,
            insurance_minor: 1,
            duties_and_tax_minor: if supplier_number == 1 { 0 } else { 2 },
            quality_testing_minor: 2,
            storage_and_cold_chain_minor: if supplier_number == 3 { 2 } else { 0 },
            financing_minor: 1,
            expiry_and_wastage_minor: match supplier_number {
                1 => 4,
                2 => 2,
                _ => 1,
            },
            expected_shortage_impact_minor: match supplier_number {
                1 => 6,
                2 => 3,
                _ => 1,
            },
        };

        match stress {
            StressCase::Baseline | StressCase::SupplierFailure | StressCase::EvidenceLoss => {}
            StressCase::FreightShock50Pct => {
                costs.freight_minor += costs.freight_minor.div_ceil(2);
            }
            StressCase::ExpiryWastageShock => costs.expiry_and_wastage_minor += 20,
            StressCase::ImportedCostShock20Pct if supplier_number >= 2 => {
                // Explicit synthetic upstream cost shock; not an FX conversion.
                costs.acquisition_minor = costs.acquisition_minor * 6 / 5;
                costs.freight_minor = costs.freight_minor * 6 / 5;
            }
            StressCase::ImportedCostShock20Pct => {}
        }

        let available_units = if matches!(stress, StressCase::SupplierFailure)
            && supplier_number == 1
        {
            0
        } else {
            150
        };
        let quality_state = if supplier_number == 0
            || (matches!(stress, StressCase::EvidenceLoss) && supplier_number == 1)
        {
            QualityEvidenceState::Unknown
        } else {
            QualityEvidenceState::Qualified
        };
        let delivery_rate = match supplier_number {
            1 => 9_200,
            2 => 9_700,
            _ => 9_900,
        };

        offers.push(SupplyOfferV1 {
            offer_id: format!("offer-{medicine_number:02}-{region_number:02}-{supplier_number:02}"),
            supplier_id: supplier_id.to_string(),
            medicine_presentation_id: medicine_id.to_string(),
            jurisdiction: region.to_string(),
            currency_code: "ZAR".to_string(),
            unit_basis: unit_basis.clone(),
            price_observation_sha256: DIGEST_A.to_string(),
            quality_evidence_sha256: DIGEST_B.to_string(),
            authorization_evidence_sha256: DIGEST_A.to_string(),
            licence_evidence_sha256: DIGEST_B.to_string(),
            capacity_evidence_sha256: DIGEST_C.to_string(),
            source_permission_evidence_sha256: DIGEST_B.to_string(),
            permitted_source_use_scopes: vec![SourceUseScopeV1::CommercialAnalysis],
            source_reuse: SourceReuseState::Permitted,
            quality_state,
            authorization_state: MarketAuthorizationState::Authorized,
            licence_state: LicenceTerritoryState::Permitted,
            price_observed_yyyymmdd: 20261001,
            price_valid_until_yyyymmdd: 20261231,
            available_units,
            minimum_order_units: 10,
            lead_time_days: 5 + (supplier_number as u16 * 4),
            shelf_life_days_at_delivery: 365,
            unit_cost: costs,
            on_time_delivery_basis_points: Some(delivery_rate),
            delivery_observation_count: 24,
            delivery_evidence_sha256: Some(DIGEST_C.to_string()),
        });
    }
    offers
}

fn checked_plan(
    policy: &ProcurementPolicyV1,
    offers: &[SupplyOfferV1],
    label: &str,
) -> Result<ProcurementPlanV1, String> {
    let plan = plan_procurement_v1(policy, offers)
        .map_err(|error| format!("{label}: policy rejected: {error:?}"))?;
    let violations = verify_plan_v1(policy, offers, &plan);
    if !violations.is_empty() {
        return Err(format!("{label}: independent checker rejected plan: {violations:?}"));
    }
    if plan.status != medicine_procurement_core::PlanStatusV1::FeasibleWithinObservedOffers
        || plan.allocated_units != policy.demand_units
        || plan.unmet_demand_units != 0
    {
        return Err(format!(
            "{label}: synthetic scenario not feasible: status={:?}, allocated={}, unmet={}",
            plan.status, plan.allocated_units, plan.unmet_demand_units
        ));
    }
    Ok(plan)
}

fn csv_row(
    medicine_id: &str,
    region: &str,
    stress: StressCase,
    raw_offer: &SupplyOfferV1,
    raw_offer_eligible: bool,
    low_cost: &ProcurementPlanV1,
    resilient: &ProcurementPlanV1,
) -> String {
    let delta = resilient.total_allocated_cost_minor as i128
        - low_cost.total_allocated_cost_minor as i128;
    format!(
        "{medicine_id},{region},{},{},{},{},{},{},{},{},{},{},{},{},{},0,0",
        stress.label(),
        raw_offer.offer_id,
        raw_offer.unit_cost.acquisition_minor,
        raw_offer_eligible,
        low_cost.allocated_units,
        low_cost.total_allocated_cost_minor,
        low_cost.supplier_count,
        resilient.allocated_units,
        resilient.total_allocated_cost_minor,
        resilient.supplier_count,
        delta,
        low_cost.expected_on_time_delivery_basis_points.unwrap_or_default(),
        resilient.expected_on_time_delivery_basis_points.unwrap_or_default(),
    )
}

fn run() -> Result<(), String> {
    println!(
        "{}",
        concat!(
            "medicine_id,region,stress,raw_lowest_offer_id,raw_lowest_acquisition_minor,",
            "raw_lowest_offer_eligible,lowest_eligible_allocated_units,",
            "lowest_eligible_total_cost_minor,lowest_eligible_supplier_count,",
            "resilient_allocated_units,resilient_total_cost_minor,resilient_supplier_count,",
            "resilience_cost_delta_minor,lowest_eligible_expected_on_time_bps,",
            "resilient_expected_on_time_bps,lowest_eligible_checker_violations,",
            "resilient_checker_violations"
        )
    );

    let mut rows = 0_usize;
    let mut raw_cheapest_unqualified = 0_usize;

    for medicine_number in 1..=12 {
        let medicine_id = format!("synthetic-presentation-{medicine_number:02}");
        for (region_number, region) in REGIONS.iter().enumerate() {
            for stress in StressCase::ALL {
                let offers = offers_for(
                    medicine_number,
                    region_number,
                    &medicine_id,
                    region,
                    stress,
                );
                let lowest_policy = policy(&medicine_id, region, 1, 10_000, DIGEST_B);
                let resilient_policy = policy(&medicine_id, region, 2, 6_000, DIGEST_C);

                let lowest_plan = checked_plan(
                    &lowest_policy,
                    &offers,
                    "lowest eligible landed-cost baseline",
                )?;
                let resilient_plan =
                    checked_plan(&resilient_policy, &offers, "resilience-constrained baseline")?;

                let raw_offer = offers
                    .iter()
                    .min_by(|left, right| {
                        left.unit_cost
                            .acquisition_minor
                            .cmp(&right.unit_cost.acquisition_minor)
                            .then_with(|| left.offer_id.cmp(&right.offer_id))
                    })
                    .ok_or_else(|| "synthetic corpus unexpectedly has no offers".to_string())?;
                let raw_offer_eligible = lowest_plan
                    .offer_evaluations
                    .iter()
                    .find(|evaluation| evaluation.offer_id == raw_offer.offer_id)
                    .map(|evaluation| evaluation.eligible)
                    .ok_or_else(|| "raw quote missing from offer evaluation".to_string())?;

                if !raw_offer_eligible {
                    raw_cheapest_unqualified += 1;
                }
                println!(
                    "{}",
                    csv_row(
                        &medicine_id,
                        region,
                        stress,
                        raw_offer,
                        raw_offer_eligible,
                        &lowest_plan,
                        &resilient_plan,
                    )
                );
                rows += 1;
            }
        }
    }

    if rows != EXPECTED_ROWS {
        return Err(format!("expected {EXPECTED_ROWS} rows, generated {rows}"));
    }
    if raw_cheapest_unqualified != EXPECTED_ROWS {
        return Err(format!(
            concat!(
                "expected the deliberately unqualified raw-lowest quote to be rejected ",
                "in every row; observed {raw_cheapest_unqualified}/{EXPECTED_ROWS}"
            )
        ));
    }
    eprintln!(
        concat!(
            "synthetic-corpus-check: PASS rows={rows} medicines=12 regions=3 stress_cases=6 ",
            "raw_lowest_unqualified={raw_cheapest_unqualified}; ",
            "both policy plans per row independently checked"
        )
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic-corpus-check: FAIL: {error}");
        std::process::exit(1);
    }
}
