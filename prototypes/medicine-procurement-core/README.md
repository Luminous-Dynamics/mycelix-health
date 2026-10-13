# Medicine Procurement Core (Prototype)

A standalone, dependency-free Rust baseline for allocating synthetic normalized medicine units across qualified offers. It is a deterministic greedy baseline, not a global optimizer or purchasing authority.

## Safety and scope

- No prescribing, therapeutic substitution, patient-level allocation, EHR integration, live supplier connection, tender award, purchase order or payment authority.
- An offer is eligible only when product presentation and unit basis exactly match; the price observation is current; source reuse is permitted; quality evidence is qualified; market authorization and licence territory are explicitly permitted; and capacity, lead time, shelf life and policy-required delivery history meet the policy.
- Lower price can never compensate for a failed hard eligibility gate.
- Price, quality, authorization, licensing, capacity and delivery reliability remain separate evidence dimensions.
- Source, rights, quality, authorization and capacity digests are supplied by an upstream evidence layer and syntax-checked only. This crate does not fetch/authenticate source artifacts or verify that a digest matches bytes. The policy declares an intended source-use scope; the offer must explicitly list that scope. This is a fail-closed consistency check, not legal verification of the declared permission.
- Costs are per normalized medicine unit in one scenario currency. There is no implicit FX conversion, strength/form/package equivalence, dose conversion or clinical substitution.
- Expected expiry/wastage and shortage-impact costs may be scenario estimates. The caller must record their provenance, uncertainty and assumptions in the input manifest.

## What it does

1. Validates scenario and digest syntax.
2. Excludes offers failing product comparability, price freshness, source-reuse, quality, authorization, licensing, shelf-life, lead-time, capacity or policy gates, with machine-readable reasons.
3. Sorts eligible offers by the transparent sum of declared per-unit lifecycle cost components.
4. Seeds the required number of distinct suppliers at their selected offer minimum orders, then fills demand in cost order subject to offer availability and aggregate supplier-concentration caps.
5. Reports allocated and unmet units, extended costs, excluded offers, delivery-history caveats and an expected on-time delivery estimate only when every selected offer has sufficient traceable rate evidence.
6. Marks unmet demand or unsatisfied sourcing constraints as InfeasibleWithinObservedOffers. Missing offers/prices never become zero-cost success.
7. Emits deterministic output for the same policy, manifest digests and offer set.

The method is deliberately named DeterministicGreedyBaselineV1. It is not proven globally optimal, and a greedy run may be infeasible even when a different allocation exists. A separate verify_plan_v1 checker re-evaluates award gates and recomputes quantities, concentration, supplier count, unit/extended cost, total cost, delivery arithmetic and status. A future exact/robust optimizer must pass that checker and compare itself against this baseline.

## Cost accounting

For each normalized unit, the baseline sums acquisition, freight, insurance, duties/tax, quality testing, storage/cold-chain, financing, expiry/wastage, and expected shortage impact. Amounts use the scenario currency's minor units with checked integer arithmetic. Different currencies and unit bases are not compared without an explicit normalization step. A current price observation does not prove that the quoted price is actually available.

## Tests and verification

The source contains 24 uniquely named synthetic unit tests covering failed quality/authorization/licence gates, mismatched presentation/unit basis, stale price, restricted source reuse, missing delivery history, duplicate offer identities, arithmetic overflow, input-order determinism, insufficient supply, concentration limits, invalid policy/date inputs, and rejection of manually injected ineligible allocations or altered cost receipts. This is a source inventory, not evidence those tests have executed.

Run the following from the repository root:

    cargo fmt --manifest-path prototypes/medicine-procurement-core/Cargo.toml -- --check
    cargo test --manifest-path prototypes/medicine-procurement-core/Cargo.toml --locked
    cargo run --manifest-path prototypes/medicine-procurement-core/Cargo.toml --example synthetic_global_scenario --locked > medicine-procurement-synthetic-report.csv
    cargo clippy --manifest-path prototypes/medicine-procurement-core/Cargo.toml --locked -- -D warnings

The executable scenario corpus covers 12 synthetic presentation IDs, 3 hypothetical regions and 6 stress cases (216 rows). Each row compares the raw lowest acquisition quote, a lowest-eligible-landed-cost baseline and a two-supplier / 60%-concentration plan. Stress cases include one supplier failure, a 50% freight shock, extra expiry/wastage, loss of one supplier's quality evidence, and an explicitly synthetic 20% imported-cost shock. Both plans must pass the independent checker and fully satisfy the synthetic demand for the report to be emitted. CI checks the report row count and uploads the CSV artifact for 14 days on both configured toolchains.

All corpus labels and costs are fictional; costs are synthetic ZAR minor units, not actual drug prices, price comparisons or savings estimates. The imported-cost case models a declared upstream cost shock, not currency conversion. Digest-shaped fixture values are sentinels, not authenticated source artifacts. Source-level test counts and queued CI are not pass evidence.

A dedicated workflow tests Rust 1.96.0 and 1.99.0. Remaining qualification steps are property-based/adversarial tests for corrupted manifests and ambiguous source evidence, plus ingestion of carefully licensed public price observations only after preserving price class, currency, package/unit basis, date, jurisdiction and source reuse terms.

Before live price ingestion, review the rights and data contract in [Medicine Price Source Ingestion Contract v1](../../docs/MEDICINE_PRICE_SOURCE_INGESTION_CONTRACT.md). In particular, South Africa's NDoH website terms require prior written permission for commercial use; downloadable is not the same as licensed for commercial analysis or redistribution.

Tracked in Mycelix-Health issue #273: https://github.com/Luminous-Dynamics/mycelix-health/issues/273
