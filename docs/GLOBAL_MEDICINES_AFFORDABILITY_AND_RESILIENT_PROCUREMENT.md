# Global Medicines Affordability and Resilient Procurement

**Status:** design and synthetic-pilot proposal; not a purchasing authority, regulatory determination, clinical recommendation, or proof of realized savings.  
**Goal:** the lowest sustainable, quality-assured landed cost with dependable availability and equitable access.

## Executive position

Yes, many medicines can become substantially cheaper and more consistently available through better generic competition, pooled demand, transparent comparisons, better tender design, appropriate licensing, and lower distribution/waste costs. It is not credible to promise that every drug can always be produced at the same minimum price everywhere. Production complexity, patent and licensing scope, regulatory requirements, country registration, market size, cold chain, supplier concentration, and supply risk differ.

The optimization target is therefore not the lowest quoted unit price. It is the lowest **total expected lifecycle cost subject to hard quality, legal, availability, resilience, and equity constraints**. A medicine that is cheapest at tender but repeatedly unavailable, too close to expiry, or not quality-assured is not a successful procurement.

The first implementation slice is tracked in [medicine-procurement core PR #274](https://github.com/Luminous-Dynamics/mycelix-health/pull/274). It is a synthetic-data-first deterministic greedy baseline with an independent allocation verifier; its runner checks are queued, so it is not yet a qualified result or evidence of savings.

## Public evidence and the initial data baseline

WHO defines essential medicines around priority health needs, efficacy/safety evidence, affordability and assured quality. WHO's 2025 Model List contains over 520 medicines, including 374 for children, and is intended to help countries focus selection, procurement and supply. It is a useful demand-prioritization reference, not a substitute for national formularies, guidelines or market authorization.

Existing mechanisms demonstrate that this can be operational, not merely theoretical:

- WHO's country pricing policy guidance recommends combining approaches such as quality-assured generic and biosimilar uptake, price transparency, tendering, mark-up rules, reference pricing, pooled procurement and tax policy. These measures have different effects in different contexts and should be compared, not presumed universally effective.
- The Global Fund reports that its Pooled Procurement Mechanism managed approximately US$1.02 billion in orders for implementers in 81 countries during 2025. It aggregates demand and negotiates price and delivery terms; it also references total cost of ownership, including freight, insurance and quality-testing costs.
- UNICEF Supply Division publishes antimalarial price observations tied to multiyear supplier agreements, including price ranges where different discounts or tiers apply. Such ceilings and contract price observations must not be confused with retail prices or universally available offers.
- The Medicines Patent Pool's AccessPaL tracks the status of licensed medicines, including product filings, approvals and supply progress across licensing territories. A patent licence does not itself establish local registration, procurement eligibility or actual supply.
- South Africa's National Department of Health publishes a medicine-price database with Single Exit Prices, ex-manufacturer prices and logistics fees. This provides a valuable local baseline, but private-sector SEP observations, public tenders and landed procurement prices are different price concepts.
- WHO prequalification is a trusted quality benchmark for eligible products in international procurement. It is not, by itself, marketing authorization for every country or every procurement channel. The buyer must define and verify the acceptable evidence route for the jurisdiction and product category.

Primary sources:
- [WHO Essential Medicines fact sheet](https://www.who.int/news-room/fact-sheets/detail/essential-medicines)
- [WHO Model List of Essential Medicines, 2025](https://www.who.int/publications/i/item/B09474)
- [WHO country pharmaceutical pricing policy guidance](https://www.who.int/teams/health-product-and-policy-standards/medicines-selection-ip-and-affordability/affordability-pricing)
- [Global Fund Pooled Procurement Mechanism](https://www.theglobalfund.org/en/sourcing-management/procurement-tools/)
- [UNICEF antimalarial price data, May 2026](https://www.unicef.org/supply/documents/antimalarials-price-data)
- [Medicines Patent Pool AccessPaL](https://medicinespatentpool.org/accesspal)
- [South African National Department of Health: Pharmaceutical Economic Evaluation and medicine price databases](https://www.health.gov.za/nhi-pee/)
- [WHO prequalification of medicines](https://www.who.int/news-room/fact-sheets/detail/prequalification-of-medicines-by-who)
- [WHO substandard and falsified medical products](https://www.who.int/health-topics/substandard-and-falsified-medical-products)

## The optimization target

For medicine presentation `m`, region `r`, period `t`, supplier `s` and allocated quantity `q`, minimize an explicit model such as:

`expected_total_cost = acquisition + freight + insurance + duties/tax + quality testing + warehousing/cold-chain + financing + expiry/wastage + shortage/stockout impact`

This is an analytical accounting boundary, not a license to assign invented monetary values to every harm. Each cost term must disclose its source, unit, currency, time window, uncertainty and whether it is observed, estimated or only a scenario assumption.

The optimization must be subject to **hard eligibility constraints**. Cost weights cannot compensate for an unregistered/unlawfully supplied product, failed quality evidence, incompatible strength/form/package, unknown licensing territory, inadequate shelf life, or a delivery schedule that violates essential-service requirements.

Other constraints may include:
- the demand interval and budget for a region and period;
- qualified supplier capacity, lead time and minimum order size;
- minimum stock and maximum acceptable stockout probability;
- expiry risk, storage needs and cold-chain availability;
- concentration ceilings or minimum qualified backup supply;
- contract obligations, tender law and award rules;
- region-specific affordability and equitable allocation commitments.

Do not report one global currency-normalized number as a complete measure of affordability. Report at least purchase price, total landed cost, expected availability, patient out-of-pocket burden where data are lawful and appropriate, and percentage of health-budget spent. Compare price observations only after making product presentation, package basis, currency, date, procurement channel, contract terms and included cost components explicit.

## Why “cheapest” can be a false economy

A quoted price may exclude freight, insurance, duties, quality testing, cold-chain, financing or domestic logistics. A low price may also come with weak delivery performance, short expiry at arrival, a single-site source, a fragile active pharmaceutical ingredient supply, or an excessively concentrated market that fails when a supplier exits. On the other hand, adding a speculative risk penalty can also bias awards unfairly.

Therefore:
- measured prices, estimated costs and policy penalties must be separate typed values;
- uncertain costs need sensitivity analysis and scenario bounds;
- the buyer must see the lowest-price baseline alongside the constrained resilient option;
- a model must never smuggle preferences into one opaque weighted score;
- any policy exception must be explicit, justified, signed by an authorized buyer and auditable;
- supplier diversification is a constrained resilience choice, not a universal rule that two suppliers are always best.

## Levers to compare, not blindly assume

### 1. Quality-assured generics and biosimilars

Promote competitive versions when the active ingredient, strength, dosage form, route, quality and intended use are genuinely comparable and local rules permit substitution. Generic and biosimilar products are not interchangeable by merely matching a text name; the relevant scientific, regulatory and presentation constraints differ.

### 2. Pooled and regional procurement

Aggregate credible demand forecasts and tender volumes across institutions or countries to strengthen negotiation and reduce fragmented procurement overhead. The pool needs a common product specification, compatible delivery window, transparent allocation rules, and governance for disputes and late payments. It must also prevent a single winning supplier from becoming an unmanageable single point of failure.

### 3. Multi-winner tenders and supply commitments

Compare single-award lowest-price procurement with multi-winner or split-volume awards, qualified backup contracts and delivery performance clauses. Evaluate whole-life cost, supplier capacity, delivery history, shelf life and competition. Do not infer capacity or quality from reputation scores.

### 4. Price transparency and correct comparisons

Combine public tender notices, medicine-price databases, contract observations and supplier quotes only after representing what each price actually means. South African SEP is a useful local comparison point for private-sector pricing but should not be treated as interchangeable with the price a public buyer pays after freight, quality testing or tender terms.

### 5. Patent pools and licensing

Map product patents, voluntary licence scope, territory, sublicence terms, expected generic development, local registration and supply readiness. A licence can enable entry but is not itself a finished product or an immediate price reduction. Respect patent rights, applicable competition and procurement law, and legitimate access pathways.

### 6. Production and distribution efficiencies

Evaluate technology transfer, regional manufacturing, API-source diversification, packaging, formulation, continuous manufacturing, demand planning, distribution routes and cold-chain efficiency with explicit capital, energy, QA, scale and delivered-cost assumptions. Local production can improve resilience and shorten lead times, but it is not automatically cheaper or more reliable at every volume.

### 7. Research and access obligations

For publicly funded research and advance purchase commitments, test transparent access, licensing, affordability and supply-continuity terms as part of the procurement/R&D agreement. Reduce avoidable discovery and formulation duplication through open methods and reproducible evidence, without pretending that open-source software removes GMP, clinical trial, regulatory or manufacturing costs.

### 8. Taxes, distribution mark-ups and patient co-payments

Model the legal and distributive impact of taxes, mark-ups, dispensing fees and co-payments. A lower acquisition price is insufficient if downstream charges keep treatment unaffordable. Policy changes require jurisdiction-specific analysis; they should not be automated by the optimizer.

## Versioned data contracts

### MedicinePresentationV1

- normalized INN/active ingredient and mapping provenance;
- strength, dosage form, route, release characteristics and package size;
- manufacturer/product and market authorization identity where available;
- jurisdiction, indication/uses only where legally supported, and regulatory state;
- explicit comparability group. Missing strength/form/package information means incomparable, not “same medicine”.

### PriceObservationV1

- observed amount, currency, unit basis, date and validity window;
- channel (public tender, private SEP, wholesaler, supplier quote, international agreement, etc.);
- price class and included/excluded freight, insurance, tax, testing, logistics and margin;
- purchase quantity, volume tier, delivery terms and contract duration;
- source identity/version, exact captured artifact reference and permitted reuse/licence state;
- status such as current, historical, estimated, confidential/not reusable, or incomparable.

A currency conversion requires an explicit dated FX source and method; it must not overwrite the native-currency observation.

### QualityAndEligibilityEvidenceV1

- jurisdiction-specific authorization pathway and product registration state;
- relevant WHO prequalification, stringent/regional/national authority or other accepted evidence;
- manufacturer/site and GMP evidence applicable to the procurement;
- recalls, alerts, product identity, strength/form compatibility and expiry/shelf-life criteria;
- reviewer identity/scope and the exact evidence snapshot used.

Unknown evidence is not eligible-by-default. WHO prequalification or an approval in one jurisdiction must not be represented as universal approval.

### SupplyOfferV1

- qualified manufacturer/site and authorized supplier identity;
- offer quantity and time-phased capacity, lead time, price tiers and minimum order;
- batch and delivery shelf-life commitments, storage/cold-chain requirements;
- historical observed delivery reliability, its corpus and uncertainty;
- upstream/API dependencies and known concentration risks;
- contract terms, penalties, cancellation rules and source provenance.

### DemandScenarioV1

- aggregated demand forecast and uncertainty interval per region/time period;
- existing inventory, expected deliveries, lead-time demand, safety-stock rule;
- explicit service-level objective and expiry/wastage assumptions;
- disease seasonality or outbreak scenario when grounded in evidence;
- no names, patient identifiers or person-level treatment inference in the public simulator.

### ProcurementDecisionReceiptV1

Bind the exact input manifest, product specification, eligibility policy, price-normalization policy, optimizer version, scenario parameters, candidate and excluded offers, selected quantity allocation, cost breakdown, sensitivity results, concentration/resilience facts, counter-evidence, conflicts of interest and authorized human approval. It records a recommendation/award decision; it does not itself authorize purchase-order execution or payment.

## Luminous / Mycelix system architecture

- **Mycelix — evidence and governance:** canonical source snapshots/selectors, price/quality/licence claims, contract and review provenance, human award and correction receipts. Do not duplicate the canonical EPI snapshot/selector ontology or use a universal trust score.
- **Symthaea — analysis:** demand forecasting, shortage-risk estimation, scenario comparisons, counterfactuals and uncertainty. Must compare with a transparent deterministic baseline and be advisory only.
- **Sol-Atlas — visibility:** country/time maps and timelines for price ranges, qualified supply, registration/licensing coverage, shortages, evidence gaps and scenario trade-offs. A visualization never confers authority.
- **Mycelix-Health — domain contracts:** medicine presentation, product eligibility, safety/recall boundary, procurement rules and authorized human-review process.
- **Nix/CI — reproducibility:** pin toolchains, public synthetic corpora, price-normalization policy, FX snapshots and exact analysis manifests.

Separate state machines are required for (a) price comparability, (b) regulatory/quality eligibility, (c) supply availability, (d) affordability, and (e) award authority. No global `trusted=true`, single score or Boolean should collapse these dimensions.

## Synthetic pilot before real procurement

Use 10–20 generic essential-medicine presentations across 3–5 hypothetical regions, 3–5 offers per presentation, distinct currencies, differing package sizes, shipping and expiry assumptions, delivery variance, and at least one unqualified low-price offer. Include recurring seasonal demand and a supplier outage scenario. Do not label synthetic price values as real market data.

Compare at least these baselines:
1. lowest quoted price only;
2. lowest eligible landed unit cost;
3. constrained allocation with capacity, delivery/service-level, expiry and concentration constraints;
4. stress case with the largest qualified supplier unavailable and/or FX and freight changes.

Report cost and availability separately, and disclose whether a “saving” is merely simulated relative to a baseline. The independent checker must recompute eligibility and arithmetic from the frozen input manifest without trusting the optimizer's chosen winner.

### Minimum adversarial cases

- same ingredient but different strength, dosage form, release type or package size;
- duplicate offer and inconsistent package quantity;
- same price amount but different currency, date, price class or included logistics;
- price observation has expired or source reuse licence is unclear;
- low-price offer is not registered/qualified in the destination market;
- WHO prequalification is incorrectly treated as universal registration;
- patent licence excludes destination territory;
- delivery capacity is less than the tendered volume;
- a single supplier wins every lot despite a resilience threshold;
- low supplier reliability makes apparent price savings disappear under stress;
- product shelf life at delivery is insufficient;
- no price or no offer is interpreted as zero cost or guaranteed supply;
- altered FX/freight assumptions cannot be traced to changed output;
- the optimizer proposes an ineligible offer and an independent checker must reject it;
- unauthorized tool/output attempts to create a purchase order or payment;
- price-only objective shifts stock toward higher-budget regions contrary to explicit equity requirements.

## Phased implementation and gates

**Phase 0 — schema and data dictionary:** define product-presentation identity, price types, currency/date/unit normalization, lawful source reuse and quality evidence.

**Phase 1 — deterministic simulator:** synthetic corpus, normalized cost calculation, transparent eligibility, multi-offer allocation and replay manifest.

**Phase 2 — independent qualification:** separate checker, property/adversarial tests, scenario perturbations, and comparisons against price-only and lowest-eligible-cost baselines.

**Phase 3 — public-data calibration:** add selected public observations from WHO/UNICEF/Global Fund/South African NDoH only when each source's reuse terms, price basis, date and comparability are recorded. Unknown source licence or unit means “not usable for this purpose,” not an inferred value.

**Phase 4 — retrospective pilot:** after source and data governance review, compare simulator recommendations against historical procurement outcomes in shadow mode with an authorized procurement partner. Publish aggregate methods, actual measured metrics and failure cases, not supplier-confidential contract details or health data.

**Phase 5 — operational decision support:** only after independent evidence review, lawful procurement and regulatory review, access control, human sign-off, correction/appeal mechanisms and a local implementation agreement. A future system may prepare draft tender analysis; purchase orders, contract awards and payments remain behind explicit authorized buyer capabilities and separate human approval.

A stage advances only on its evidence gates. A green build, synthetic saving, quoted price, patent licence, WHO listing or model score by itself is not evidence that medicine is affordable, available or appropriate in a destination market.

## Success metrics

- median and interquartile range of comparable landed cost per standardized presentation;
- absolute price and affordability metrics reported separately;
- stockout days / order fill rate and delivery lead-time distribution;
- expiry and wastage;
- percent of demand covered by qualified sources;
- supplier concentration and scenario resilience after source failure;
- procurement cycle time and tender administrative cost;
- share of eligible volume supplied through quality-assured competition;
- equity of access across regions and purchasing power;
- difference between simulated and actually realized savings;
- false eligibility / false exclusion rates in independent review.

Never claim realized savings from a simulator, or equate low price with quality, access or clinical benefit.

## Next implementation issue

Implementation and acceptance criteria are tracked in [MED-PRICE-001 / Mycelix-Health issue #273](https://github.com/Luminous-Dynamics/mycelix-health/issues/273).

## Explicit non-claims

This design does not establish that a particular medicine is clinically appropriate, quality-assured, legally importable, or cheaper than an alternative. It is not a global buying cartel, a purchasing agent, a drug manufacturer, or an authorization to bypass a patent, licence, market authorization, GMP, pharmacovigilance or tender law. Its purpose is to support accountable buyers with better evidence and reproducible analysis.
