# Medicine Price Source Ingestion Contract v1

Status: design contract; not an authorization to scrape, store, redistribute, or commercially use any source.

## Purpose

Make real-price comparisons legally usable, product-comparable, quality-gated, current, and reproducible. The ingestion boundary must reject unknown permissions and ambiguous units before prices can influence an allocation. It must not turn a downloadable file into a presumed open dataset.

## Verified source characteristics

### South Africa: National Department of Health medicine-price database

The Department of Health's Pharmaceutical Economic Evaluation page publishes dated medicine-price database downloads and states that they include Single Exit Prices (SEP), ex-manufacturer prices, and logistics fees for active and discontinued medicines. It explicitly warns that discontinued records remain listed because registration status does not prove market availability.

The department's website terms distinguish non-commercial informational/reference use from commercial use. For commercial use, the terms require prior written permission; they also state that access to website data does not itself grant a licence. Therefore:

- Treat the NDoH price database as a high-value **candidate source**, not as pre-authorized input to a commercial product.
- Before downloading for bulk ingestion, storing, transforming, redistributing or displaying its records in Luminous Dynamics / Mycelix services, seek and record written permission covering the intended use.
- Until permission is verified for the concrete use, record only source-discovery metadata allowed by the terms; do not ingest database rows into a commercial analytical corpus.
- Even after permission, a price record is not proof of current stock, wholesaler offer, patient out-of-pocket cost or procurement award price.
- Preserve the original price class; never relabel SEP, ex-manufacturer price, logistics fee or another value as the same thing.

Official pages:
- https://www.health.gov.za/nhi-pee/
- https://www.health.gov.za/terms-and-conditions/

### WHO prequalification

WHO publishes downloadable listings of prequalified finished pharmaceutical products and public assessment/inspection reports. These support quality-evidence discovery, subject to the scope and limitations of the relevant listing. WHO prequalification must remain a separate evidence dimension: it does not by itself establish destination-country registration, current stock, current price, or an eligible licence for every transaction.

Official pages:
- https://extranet.who.int/prequal/medicines/prequalified/finished-pharmaceutical-products
- https://extranet.who.int/prequal/medicines/prequalification-reports
- https://www.who.int/news-room/fact-sheets/detail/prequalification-of-medicines-by-who

### FHIR projection

HL7 FHIR R5 `MedicationKnowledge` can represent medication cost, effective date, cost type, and the source/owner of pricing information. Use it as a downstream interoperability projection where suitable—not as the complete canonical price/provenance/rights contract. The source's actual maturity and implementation profile must be documented, and missing FHIR fields must not silently erase rights, comparability, uncertainty, or evidence lineage.

- https://hl7.org/fhir/R5/medicationknowledge.html

## Required source manifest

Every source adapter must emit a versioned manifest before rows are processed:

- `source_id`, publisher, canonical source URL, source revision/publication date and retrieval timestamp.
- Original file URL and filename; content digest; adapter version; parser version; row count; rejected-row count; and a deterministic parse-report digest.
- Exact terms URL and retrieved terms revision/digest where possible.
- Permission decision, reviewer/authority, permitted purpose, permitted geography, storage/caching rights, transformation rights, redistribution/display rights, commercial-use status, expiry/review date and any attribution obligations.
- Source availability and freshness limits, known limitations, and contact/escalation route.
- Whether original bytes may be retained. When not permitted, preserve only the metadata and evidence that are lawful to keep.

Permission is scoped. A right to view a page is not presumed to grant bulk extraction; non-commercial reference use is not presumed to permit commercial analytics; permission to analyze records is not presumed to permit republishing them. Unknown, conflicting, expired or insufficient rights fail closed. The prototype now compares the policy's required scope with the offer's declared permitted scopes and checks a separate permission-evidence digest's syntax. Those fields remain assertions from the upstream rights review: the prototype cannot determine whether a licence text legally grants them.

## Canonical price observation

Each parsed record should preserve, at minimum:

- Stable record identity and exact medicinal presentation identity; the source's original product and pack identifiers; ingredient(s), strength, dosage form, route where provided, pack size, unit count and normalization method/version.
- Jurisdiction and submarket/channel; price class/type exactly as published; amount as a decimal-safe integer in explicitly named minor units (or a decimal representation with currency exponent); ISO currency code; tax and fee inclusions/exclusions; and quoted unit/pack basis.
- Effective-from/effective-to dates, observed/publication date, source update date, and source's discontinued/withdrawn status.
- Status class: `published_reference_price`, `quoted_offer`, `awarded_contract_price`, `transaction_price`, `reimbursement_price`, or `unknown_price_class`. Never conflate these classes.
- Separate assertions for registered/authorized in destination market, quality status, licensing territory, supplier identity, stock/available quantity, quote validity, lead time and shelf life. Each assertion carries source and timestamp; none is inferred from a price row.
- Observed versus imputed/scenario-estimated components, estimation method, uncertainty/range, and a link back to the raw field or source span.
- Source-use permission reference and lineage digest.
- Exact intended-use scope required by the procurement policy plus the explicitly authorized scopes recorded by the rights-reviewed source manifest. Scope values are non-hierarchical; permission for one purpose never implies another.

Never use a product-name string alone as a cross-market join key. Match strength, formulation, release type, route, package unit, manufacturer and local authorization; ambiguous matches stay unresolved. Brand/generic or pack normalization must be explicit and independently reviewed. No therapeutic substitution or dose-equivalence is inferred by the price engine.

## Eligibility and comparison rules

1. **Rights first.** Only pass a row to downstream processing when the source manifest authorizes this exact intended use. A valid content digest proves byte identity only, not lawful reuse, correctness or freshness.
2. **Identity and price class before price.** Require exact presentation, normalized unit-basis, broad price-kind and source-defined price-class agreement. A published reference price is not a supplier quote; SEP, ex-manufacturer price and logistics fee must remain different strata. If package/unit conversion is required, retain the source quantity, formula, rounding policy, and reviewer-approved conversion receipt. Fail closed on unknown conversion.
3. **Price classes stay separate.** Compare like-for-like jurisdiction, channel, price class, effective period and tax/fee treatment. An SEP and a tender award should appear in different strata unless a documented analytical question explicitly relates them.
4. **No stock inference.** Registration/listing, historical price, supplier capacity, and observed availability are different records. A discontinued row is not an available offer. A quote is not a purchase.
5. **Quality is a hard gate.** Link to a regulator decision, WHO prequalification/assessment evidence, or another policy-accepted evidence source. Preserve scope, date and product/manufacturer/site mapping. Unknown quality is not treated as pass.
6. **Local authorization and licence are independent.** Resolve actual destination authorization and licence territory separately; WHO listing, patent-pool membership, or another jurisdiction's authorization does not replace this step.
7. **Comparable price requires landed-cost accounting.** Add only supported or explicitly scenario-estimated acquisition, freight, insurance, duties/tax, quality verification, storage/cold-chain, financing, wastage/expiry and expected shortage-impact components. Label unknowns instead of defaulting them to zero.
8. **Freshness and conflicts are explicit.** Record effective periods and freshness thresholds; retain conflicting observations as conflicts rather than silently averaging or selecting one.
9. **Human review precedes use.** Source terms, parser results, product matching, legal status, price-class semantics and quality evidence need review before live observations can enter operational decisions.

## Evidence and integration boundaries

- **Mycelix:** source rights, versioned observations, evidence links, approval/revocation, correction/supersession, and reproducible decision receipts. A receipt must bind the policy digest, exact source-manifest digest, parser version and observation-set digest.
- **Symthaea:** demand forecasts and scenario uncertainty only after versioned demand assumptions are supplied. It must not turn speculative demand or estimated shortage cost into fact.
- **Sol-Atlas:** display source date, price class, jurisdiction, unit basis, permission-safe citation, uncertainty, freshness and comparability warnings. A ranking must not hide rejected alternatives or unavailable-stock uncertainty.
- **Mycelix-Health:** enforce market/quality/licence gates and the authorized workflow; no autonomous purchase orders, payments, prescribing or therapeutic substitution.
- **Independent verifier:** reconstruct eligibility and arithmetic from the frozen manifest and accepted evidence; never trust the planner's own eligibility/total-cost receipt as proof.

No patient-level data is needed for this price-ingestion project. Keep PHI out of source manifests and scenario corpora.

## Qualification gates before any live source adapter

- Written rights decision for the intended non-commercial/commercial use, data retention, transformation and publication.
- Golden fixtures with source-file hashes and reviewed expected rows; parser rejects malformed currencies, dates, units, duplicate identities and conflicting revisions.
- Deterministic re-run produces an identical normalized output and manifest digest.
- A reviewer can trace every normalized price back to an original field without losing price class or unit basis.
- No row passes if rights, product identity, quality, destination authorization or licence status is unknown.
- Add a correction/supersession test and an expired-source test.
- Keep any pilot in shadow analysis; no procurement decision or price claim should be based on this prototype until applicable legal, regulatory, clinical-safety and procurement reviews have accepted it.

## Explicit unresolved questions

This contract does not assume that any candidate source grants open-data or commercial redistribution rights. It does not claim actual savings, market availability, regulatory approval, or product equivalence. Source-specific rights should be resolved before live ingestion is implemented.
