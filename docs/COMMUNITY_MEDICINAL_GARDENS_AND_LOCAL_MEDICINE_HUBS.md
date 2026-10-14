# Community Medicinal Gardens and Licensed Local Medicine Hubs

Status: design proposal for MED-BIO-001. This document defines a safety-conscious system architecture; it is not a clinical guide, horticultural monograph, legal opinion, manufacturing instruction, or authorization to produce or dispense medicines.

## Core thesis

**Grow locally where cultivation is appropriate; manufacture locally where quality systems and economics justify it; dispense and treat through qualified, accountable health services.**

Do not collapse gardens, medicine factories, and clinics into one informal operation. A community garden can build access, skills, biodiversity stewardship and traceable plant supply. It cannot establish that a plant is effective, safe at a particular dose, uncontaminated, legally marketable, or equivalent to a regulated medicine. A clinic may provide health services and medicine access, but regulated product manufacture belongs only in appropriately authorized facilities with validated processes and release controls.

The target is a distributed network of community gardens, grower cooperatives, accredited testing partners, licensed herbal/pharmaceutical manufacturers, pharmacies/clinics, and shared digital infrastructure—not a medicine factory in every back yard.

## 1. Product and activity boundaries

Every record, interface and workflow must declare which lane it belongs to. Data and permissions do not automatically flow between lanes.

### Lane A — garden, learning and household cultivation

Purpose: make it easy to grow, identify and care for locally suitable plants; support education, food, cultural heritage, conservation and non-clinical wellbeing.

Allowed platform functions:
- Site planning from climate, light, water, soil and space information.
- Species identification support with uncertainty; expert-confirmed identity is required before a plant can be tagged as an approved medicinal-botanical crop.
- Growing calendars, irrigation and compost guidance, pest observations, seed/propagation provenance, accessibility and multilingual learning material.
- Safety notices for toxic lookalikes, children/pets, allergy risks, contamination concerns, conservation restrictions and invasive potential.
- Garden journal and optional community exchange of horticultural knowledge.

Guardrails:
- Do not provide individualized treatment recommendations, doses, tincture/extraction recipes, drug substitutions, or claims that a garden plant treats a condition.
- Unknown, misidentified, illegally collected, protected, contaminated or invasive-risk plants are not accepted into a medicinal-crop list.
- Begin with educational and culinary horticulture plus a locally reviewed, jurisdiction-specific list of permitted species. Any therapeutic descriptions must be evidence-reviewed, source-linked, caveated and human-approved.
- People can participate without supplying health records. Do not collect patient-level or sensitive cultural knowledge by default.

### Lane B — traceable botanical material for research or an authorized buyer

Purpose: let communities and small growers supply authenticated plant material into a responsible, quality-managed chain, where lawful.

Required practices:
- Species identity verified by a qualified botanist/herbarium or other accepted method; retain a voucher/reference sample where appropriate.
- Documented seed/propagation source and lawful access; cultivation site, water/soil context, inputs, harvest date, plant part, drying/storage conditions, yield, handlers and transport.
- Distinct lot IDs, retained samples, contamination-risk controls, cleaning records and chain of custody.
- Product specification and acceptance criteria agreed with the authorized buyer before planting at scale.
- Testing strategy selected by a qualified quality team and risk assessment; may include identity, foreign matter, microbial contamination, pesticides, heavy metals, mycotoxins and residual/processing contaminants where relevant.
- Rejection, quarantine, recall and complaint workflows. A failed batch is never blended into a passing batch to hide a failure.

A grower record or Mycelix proof-of-origin is not a certificate of identity, safety, efficacy or regulatory approval. Those require the appropriate independent assessments.

### Lane C — regulated preparation or medicine manufacture

Purpose: make a defined, authorized product reproducibly and supply it through lawful channels.

Only an appropriately licensed/authorized site with trained accountable personnel, quality management, validated methods, suitable equipment, controlled records, testing, batch disposition and regulator-required oversight may manufacture a medicine. Applicable registration/authorization, advertising/claims, dispensing, distribution, import/export, pharmacovigilance and environmental rules must be checked for each jurisdiction and product category.

The software may manage evidence and workflow; it cannot grant the legal authority to make or release a medicine. A local hub could initially perform cultivation support, botanical sorting, training, or contracted quality-preserving post-harvest operations only after a jurisdiction-specific review confirms what activities are permissible. Do not assume that drying, grinding, packaging or making a tea is outside medicines regulation simply because the process seems simple.

Do not begin with chemical API synthesis, sterile production, injectables, biologics, controlled substances, highly potent substances, or amateur extraction/concentration. These require specialized risk controls and are not sensible neighborhood-garden pilot activities.

## 2. Recommended network architecture

### Community layer: small gardens and co-operatives

- Household plots, schools and shared gardens operate as independent sites.
- Offline-first phone/web/PWA workflows, printed QR/lot labels and paper fallback make participation possible with low connectivity.
- Growers control whether their horticultural records are public, shared with a cooperative, or sent to an authorized buyer.
- Garden mentors, qualified botanists, conservation partners and traditional knowledge holders help define locally appropriate content.
- Optional cooperative services include shared seedlings, tools, irrigation, training, soil/water testing, drying equipment for lawful raw material, accessible transport and purchasing commitments.

### Regional support layer: agricultural and quality services

- Agronomists/botanists support species and site selection, cultivation plans and identity verification.
- Environmental/biodiversity experts review conservation status, invasive potential, lawful sourcing and harvest limits.
- Independent laboratories and experienced quality personnel specify tests, chain of custody and acceptance rules.
- Extension services, universities, public-health bodies, co-ops and community organizations provide local capability.
- A licensed buyer/manufacturer states demand, specification, acceptable lot sizes, forecast and rejection rules before production is scaled.

### Clinic and pharmacy layer

Clinics should be accessible health-service points first: prevention, assessment, referrals, evidence-based treatment, medicine access, adherence support and adverse-event reporting. They can connect community-grown supply to responsible research or authorized supply chains. A clinic should not receive an informal unverified botanical lot and convert it directly into a treatment product.

Where lawful and appropriately staffed, a pharmacy or authorized clinical service may offer permitted patient-specific compounding or other preparation; the exact scope must be confirmed with the responsible regulator and professional council. That is a separate workflow from bulk manufacture and from a community garden.

### Regional production layer

Use shared regional facilities where justified by enough predictable, aggregated demand and the cost of quality systems. Depending on the product, these may be:
- a botanical intake and identity/quality-testing service;
- a licensed herbal-medicine manufacturer;
- a licensed conventional generic manufacturer;
- a contracted analytical laboratory;
- a technology-transfer or public/private manufacturing partner.

A garden network should not manufacture every product. Many essential medicines will continue to be best sourced from established quality-assured manufacturers; local systems should improve access through forecasting, pooling, local distribution, technology transfer and selective local production when it is safe and economically justified.

## 3. End-to-end operating loop

1. **Community mandate and governance.** Agree who the system serves, who owns the garden and data, how decisions are made, how costs/revenue/benefits are shared, and how someone exits or disputes a decision.
2. **Define the health or cultivation problem.** Start from a documented access problem or learning goal, not an assumption that a traditional claim must be true. Clinical needs and product selection require qualified public-health/clinical review.
3. **Rights and conservation gate.** Check land access, species status, seed/propagation legality, water rights and applicable bioprospecting/access-and-benefit-sharing obligations. Protect confidential traditional knowledge. Obtain informed consent and the agreements required before recording or commercializing community knowledge or biological resources.
4. **Evidence review.** Separate traditional-use documentation, preclinical evidence, clinical evidence, safety information, product monographs and regulatory status. Preserve uncertainty and disagreement. A culturally important practice can be documented without automatically being represented as proven treatment.
5. **Site and crop suitability.** Use climate, seasonal rainfall, heat, light, soil/water test results, water availability, land access and conservation risk. Symthaea suggests scenarios and candidate protocols; local experts and the community approve the actual plan.
6. **Buyer specification before scale.** If material is intended for an authorized product chain, obtain the buyer's specification, quantity forecast, accepted parts, identity methods, contaminant limits, packaging, storage, test costs and rejection terms before planting for commercial supply.
7. **Cultivation and traceability.** Give each site, propagation source, crop cycle, harvest and sub-lot a durable identity. Record exceptions and interventions, not only successful results.
8. **Independent quality gate.** Hold lots pending identity and required risk-based tests. A sensor, image classifier, grower's self-attestation, or AI confidence score cannot replace required testing.
9. **Authorized processing/manufacture.** Only route accepted lots to the authorized recipient and agreed operation. The receiving party creates a new processing/product batch linked to all source lots and process records.
10. **Release and distribution.** Authorized quality personnel make the product release decision under the site's quality system. The system records where a released lot may lawfully go, its conditions, expiry/retest date where applicable, and recall instructions.
11. **Safety monitoring and learning.** Record complaints, adverse events, contamination, product returns, and efficacy questions through the appropriate protected channel. Qualified humans assess them and report as required. A critical signal suspends the affected lot/product pathway pending review.
12. **Community return of value.** Publish understandable reports of accepted/rejected yields, costs, payment timing, safety events, water use, ecological effects and benefit sharing without exposing private knowledge or patient data.

## 4. Mycelix / Symthaea / Mycelix-Health responsibilities

### Mycelix — evidence, consent and accountable governance

Use Mycelix as an evidence and coordination fabric, not as a claim that data are inherently true because they are recorded. Store signed/versioned events and links to evidence, with permission-aware access and explicit revocation/correction/supersession.

Core records:
- `CommunityCharterV1`: purpose, owners, voting/consent rules, benefit sharing, dispute resolution, withdrawal and data governance.
- `KnowledgePermissionV1`: contributor/community, consent scope, permitted use, publication limits, commercial-use permission, expiry/review and withdrawal/correction process.
- `SiteProfileV1`: site ID, climate/site observations, land/water authority, test references, risk flags and review date.
- `SpeciesProfileV1`: scientific name/version, local names where consented, identity authority/voucher, permitted regions, protected/invasive status, plant part, accepted uses of the data, quality risks and evidence level.
- `PropagationLotV1`, `CultivationBatchV1`, `HarvestLotV1`, `StorageTransferV1`: source, site, dates, operations, handlers, quantity, exception records and links to retained samples.
- `QualityAssessmentV1`: sample/lot link, method/version, lab/person/lab authorization where applicable, results, units, specification, uncertainty, pass/fail/quarantine and signed report reference.
- `BuyerSpecificationV1`: intended supply purpose, destination, exact product/material identity, acceptance specification, volumes, rights, quote/contract and rejection terms.
- `ManufacturingBatchV1`, `ReleaseDecisionV1`, `DistributionLotV1`, `RecallEventV1`: licensed site and authorization reference, input lots, process/equipment/SOP versions, deviations, quality evidence, authorized release, downstream recipients and closure evidence.
- `SafetySignalV1`: privacy-protected complaint/adverse event, product/lot, seriousness/urgency, responsible clinical/quality reviewer, regulator reporting state and follow-up.
- `BenefitSharingReceiptV1`: consented benefit-sharing agreement, accrued and paid benefits, beneficiary-approved visibility and dispute state.

No patient diagnosis, medication list or adverse-event narrative should be put in a public garden log or a public distributed record. Store minimum necessary details in the proper protected clinical system and reference them through permission-checked identifiers. Personal data and confidential traditional knowledge should not be public just because a ledger is transparent.

### Symthaea — reasoning and engineering bootstrap

Symthaea can help build much of the infrastructure and continuously compare plans, but it must not self-authorize a species as medicinal, decide a treatment, waive quality failures or approve release.

Safe, useful jobs:
- Generate site-specific cultivation-plan drafts from approved species catalogues, weather/climate data, water constraints and locally reviewed protocols.
- Simulate seasonal demand/supply, planting schedules, water use, germination/yield uncertainty and supplier or transport disruptions; label assumptions and confidence.
- Draft multilingual, accessible education and SOP/checklist content from version-controlled approved sources.
- Generate Rust domain types, schemas, migrations, Holochain entry/integrity/coordinator zomes, APIs and Leptos interfaces from approved contracts.
- Generate test fixtures, property tests, fault-injection scenarios, reproducibility checks, documentation, Nix development shells/deployment, monitoring and backup/restore scripts.
- Produce bill-of-materials and build plans for non-clinical garden infrastructure such as irrigation monitoring and traceability terminals; require human engineering review before building or controlling physical equipment.
- Monitor data drift, missing readings, unexpected yield changes, sensor failures, recalls and deadline violations, then open review tasks with evidence.

Required engineering pattern:
1. A human-approved requirements/constraint manifest is frozen before code generation.
2. Symthaea generates a patch, design and tests in an isolated branch or sandbox; it does not push directly to a production branch or alter protected policies.
3. An independent checker verifies schemas, invariants, access controls, migrations, dependency licensing, test coverage and negative controls.
4. Exact-head CI must pass; queued is not pass, and stale-head results are not evidence for new code.
5. A second reviewer and the domain owner approve release; clinical, regulatory, quality and environmental changes require the relevant qualified human sign-off.
6. The deployed system uses least privilege, signed releases, reproducible builds, audited configuration, monitoring and a tested rollback.
7. Any failed hard gate results in hold/quarantine or a no-go decision, never a confidence-score override.

Symthaea can bootstrap **software and operational tooling** substantially. It cannot autonomously bootstrap scientific validity, regulatory permission, clinical competence, laboratory accreditation, community consent, safe physical operations, or public trust. Those require human institutions and real-world evidence.

### Mycelix-Health — clinical and product-safety boundary

- Own the jurisdiction/product-policy profiles, evidence tiering, regulated claim boundary and clinical governance workflows.
- Separate education/garden content from health-product claims.
- Keep authorized clinician/pharmacist decisions distinct from AI-generated informational content.
- Support product/lots lookup, issue reporting, recall notices and adverse-event escalation with privacy controls.
- Enforce that unqualified or unverified lots cannot be represented as a dispensed medicine.
- Treat the absence of reported harms as unknown, not proof of safety.

### Optional connected services

- Sol-Atlas can map climate, water constraints, logistics, service coverage and distance to testing/clinical facilities, but must not expose protected sites or traditional-knowledge resources.
- Local weather and sensor services can advise on irrigation scheduling; sensor estimates are not verified agronomic measurements until calibrated.
- Procurement tools can aggregate forecast demand and negotiate with authorized buyers/manufacturers, preserving product specification, quality and price-class comparability.

## 5. Species and product selection policy

Do not start with a global top-100 list of "medicinal plants" or AI-generated remedy suggestions. Use a gated catalogue assembled for each jurisdiction and community.

Candidate admission should require:
- verified taxonomic identity and an accepted local name map;
- evidence and safety review appropriate to the intended statement or activity;
- site/climate viability and sustainable propagation;
- no unresolved protected-species, invasive-species, land-access or water-risk issue;
- known plant part and quality risks;
- documented rights for relevant traditional knowledge and biological resources;
- a defined use-lane (education, research, lawful raw botanical supply, or authorized medicine);
- a qualified human owner responsible for keeping the entry current.

Use a separate evidence grade for identity, cultivation knowledge, traditional-use documentation, clinical efficacy, safety/interactions, quality control and legal status. Never combine these into one reassuring "AI score." Evidence of traditional use is culturally important information, but it does not by itself establish clinical effectiveness or safety. Absence of evidence is labelled as such.

Avoid a starter catalogue centered on rare wild-harvested species. Prefer lawful cultivated propagation and expert-approved species with manageable ecological risk; a locally qualified group must choose the actual species and the intended use. We do not prescribe species, recipes or doses in this design document.

## 6. Quality and safety case

A release path must answer these questions with auditable evidence:
- Is it the correct species and part, without substitution or adulteration?
- Was it cultivated/collected lawfully and sustainably?
- Are water, soil, inputs, drying and storage suitable for the specification?
- Have relevant microbial, chemical and physical contaminants been assessed?
- Is the batch traceable in both directions, including sub-lots and mixtures?
- Are measured active/marker constituents and batch consistency relevant for this specific product, and are the chosen methods fit for purpose?
- Are stability, packaging, storage, shelf-life/retest, transport and recall requirements justified?
- Is there credible evidence for the specific claim and intended population, including interactions/contraindications and vulnerable groups?
- Is this product category and operation authorized in the destination jurisdiction?
- For regulated products and applicable procurement paths, does the evidence bind the exact product identity/listing separately from the manufacturer's licence and any third-party arrangement?
- Is adverse-event monitoring staffed and functional?

A failed answer puts the affected material on hold. It does not mean a person needs to discard a garden plant for educational use; it means the material cannot progress into a medicine supply chain or be represented as an approved treatment without clearing the required gate.

## 7. Governance, rights and ecological sustainability

- Community governance must exist before data collection and commercial activity, not be appended after the project succeeds.
- Traditional knowledge is not a public dataset. Do not collect, translate, publish, train models on, or commercialize sacred, restricted or sensitive knowledge without clear permission and agreed terms.
- The relevant knowledge holders must determine attribution, who can use knowledge, what can be withheld, benefit-sharing, and how consent is reviewed or withdrawn where applicable.
- In South Africa, legal review must address the National Environmental Management: Biodiversity Act and current bioprospecting/access-and-benefit-sharing rules for relevant biological resources/traditional knowledge, plus land, protected-species, municipal and water rules. The DFFE Bioprospecting, Access and Benefit-Sharing (BABS) portal identifies eligible applicant classes and supplies materials-transfer, benefit-sharing and community-resolution documents. Determine whether a specific activity triggers a permit or notification before proceeding; do not infer that every household garden does, or that none does. The Traditional Health Practitioners Act provides a separate framework for traditional healthcare practitioners; it is not a blanket authorization to manufacture medicines.
- Protect biodiversity: cultivate rather than deplete wild stocks where appropriate, avoid invasive species, model water demand, track soil effects, and set harvest ceilings and restoration conditions.
- Define fair payment and benefit sharing early; community members should not bear agronomic risk while downstream manufacturers capture all value.
- Make privacy, accessibility, language inclusion, disability access and low-connectivity operation design requirements.


### Privacy and data minimisation (POPIA-aligned design target)

The Information Regulator's personal privacy principles include collecting only data necessary for a specific defined purpose, securing records, and retaining no more personal information than needed. POPIA sets conditions for lawful processing of personal information by public and private bodies; this garden prototype should be treated as a product-design commitment, not as a substitute for the responsible party's legal assessment.

For the garden layer, that translates into these defaults:

- No account required for paper or local-only planning; do not require an exact household address or GPS coordinate.
- Use a random, locally generated profile identifier and coarse locality from a reviewed catalogue region list—not a user's name or free-text address.
- Do not collect patient records, symptoms, medication data, or reasons for interface accessibility preferences.
- Store the site profile on-device until the person explicitly opts into a separate sync workflow. A stored withdrawal state is not proof that previously synchronized copies were erased; deletion/export must be implemented and verified independently.
- Never store or publish traditional/sacred knowledge as garden onboarding data. Any later collection, translation, model training or commercial use needs a separately governed permission and benefit-sharing path.
- Make profile export disclose only fields the user deliberately selects; local-only does not automatically make printed/exported data anonymous.

Sources: [South African Protection of Personal Information Act 4 of 2013](https://www.gov.za/documents/protection-personal-information-act); [Information Regulator — Personal Privacy Principles](https://eservices.inforegulator.org.za/pps/principles.aspx).

## 8. Symthaea bootstrap plan

Symthaea may develop the software in small, independently reviewable stages.

### Bootstrap A — domain contract and simulator
Generate Rust types for community charters, site profiles, species catalogue evidence, garden plans and batch records. Build deterministic synthetic fixtures only. Add validators for required provenance, rights status, species/site/jurisdiction compatibility, explicit unknowns, and fail-closed permissions.

### Bootstrap B — garden assistant
Build a Leptos interface for site conditions, approved species list, planting/maintenance tasks and issue capture. Add offline capture and paper/QR fallback. No medical advice and no inference that a crop is safe to consume.

### Bootstrap C — batch traceability
Add signed lot creation, parent/child lot relationships, hold/release states, sample records, quality reports and recall graph traversal. Test duplicate labels, mixed lots, missing evidence, revoked permissions, bad clocks, disconnected sites, and correction/supersession.

### Bootstrap D — co-op forecasting and logistics
Use only approved, aggregated demand and grower opt-in supply data. Simulate surplus, shortages, seasonal variance, water constraints and transport costs. Keep the no-go result available when demand, evidence or capacity is insufficient.

### Bootstrap E — authorized partner pilot
Integrate with an actual qualified botanical expert, community partner, independent lab and licensed/authorized buyer or manufacturer. Start in shadow mode; no production medicine release is driven by the prototype. Contracts, responsibilities, quality specifications and legal permissions must be resolved before any batch is accepted for a product.

### Bootstrap F — controlled expansion
Re-use the data and policy contracts with jurisdiction packs. Require fresh legal/regulatory/clinical review per region and product category. Do not copy one country's species list, rules, water assumptions or authorization state into another country.

Every generated artifact should have an input manifest, prompt/spec version, code/tool version, test corpus identity, output digest, review status and approving human. Do not train shared models on community knowledge by default.

## 9. Pilot plan and stage gates

### Stage 0 — establish local ownership and authority
Participants: community/grower organization; local botanist/herbarium; public-health or clinical adviser; regulatory/legal adviser; environmental/biodiversity partner; and potential lab/manufacturer. Deliverables: charter, consent protocol, use-lane decisions, local legal/permit matrix, candidate-species review process and stop-work authority.

**No-go:** no accountable community owner, unresolved permissions, unclear data ownership, or no route for reporting harm.

### Stage 1 — non-clinical garden prototype
One or a few Johannesburg-area community sites; catalogue is manually approved and limited to lawful educational/culinary horticulture. Test onboarding, water/soil checks, maintenance support, offline capture, plant identity uncertainty and accessibility. Do not distribute treatment claims or medicine products.

Metrics: successful onboarding, plant survival, water use, task completion, misidentification detection, incident handling, user comprehension and accessibility.

### Stage 2 — quality-aware cultivation trial
Only after the species and intended supply purpose are approved, grow small documented lots with identity checks, input/harvest records, retained samples and external testing. Treat outcomes as a cultivation/quality dataset, not evidence of clinical efficacy.

Metrics: traceability completeness, yield and variance, contaminant/test failure rate, water per accepted unit, cost per accepted lot, rejection reasons, and worker safety.

### Stage 3 — licensed partner evaluation
A licensed/authorized buyer or manufacturer writes the specification and confirms permissions. The partners determine the legally permissible handoff activities, independent tests and batch disposition. Mycelix records evidence and transitions; qualified staff control physical operations and release.

Metrics: proportion of batches with complete chain of custody, independently accepted yield, full landed cost, time-to-release, recalls/deviations, grower payment reliability and community benefit distribution.

### Stage 4 — local-hub business case
Only scale when there is predictable demand and a documented quality/regulatory/finance plan. Compare:
- purchasing existing quality-assured products;
- pooling procurement and local distribution;
- local cultivation/raw-botanical supply;
- contract manufacturing/technology transfer;
- a licensed regional plant-processing or manufacturing facility.

A new factory wins only if it can deliver dependable quality and total cost over its life cycle; a community garden or local label is not, by itself, a reason to manufacture.

## 10. Success metrics

Do not optimize only for number of plants grown or nominal price.

- **Access:** travel/availability improvements, continuity of essential medicine supply, out-of-pocket cost for the relevant authorized product and who benefits.
- **Quality and safety:** identity/test pass rates, contamination, deviations, recalls, adverse-event reports and closure time.
- **Cultivation:** survival, accepted harvest yield, water/energy/input intensity, soil and biodiversity impact, season-to-season variance.
- **Economics:** cost per accepted lot/product including testing, staff, compliance, wastage, transport, financing, maintenance and downtime; comparison against reliable quality-assured alternatives.
- **Community value:** participation, training completion, fair payment, benefit-sharing compliance and accessible governance.
- **Software integrity:** provenance completeness, deterministic replay, access-control checks, offline recovery, recall reachability, corrected-record lineage and exact-head build/test evidence.

## 11. Hard no-go conditions

Stop the affected pathway if any of the following is true:
- rights, consent, land access, protected-species status, required BABS permit/notification, or a legally eligible applicant/partner (where needed) is unknown;
- species identity is uncertain for a medicinal supply lot;
- a price/product/claim/lot cannot be compared or traced on the defined basis;
- a required quality test is missing, failed or outside specification;
- an operation or site lacks required authorization;
- product or clinical claims exceed accepted evidence or regulatory status;
- an adverse event/contamination signal is not being triaged;
- the independent checker, audit trail, access control or recall pathway fails;
- the system would publish sensitive traditional knowledge or clinical information without authorization.

A no-go state should explain the missing evidence, owner and next review step, while making it impossible for a cheap price, urgent demand, model confidence or user pressure to bypass the gate.

## 12. South Africa-first research references

This is a system-design reference list, not a full legal compliance opinion. Review the current instruments with SAHPRA, qualified counsel and relevant professional/environmental bodies before any activity involving product manufacture, clinical use, commercialization, protected species or traditional knowledge.

### Additional South African implementation findings (checked 14 October 2026)

These findings make the Johannesburg pilot more specific without preselecting a medicinal species or implying that a garden harvest is safe to use as medicine:

- **Treat conservation status as an input to the catalogue, not a footnote.** SANBI's National Biodiversity Assessment reports 143 medicinal-plant species as threatened in its 2024 Red List assessments and a further 69 of conservation concern. The catalogue should have a hard conservation/collection gate before promoting species for commercial growing: [SANBI — importance of South Africa's biodiversity](https://nba.sanbi.org.za/content/context/importance.html).
- **There is a real conservation rationale for cultivation support, but do not generalize from a single case.** SANBI describes a programme supplying cultivated *Warburgia salutaris* plants for home gardens as a way to reduce pressure on wild populations. This supports testing seedling access, grower guidance and traceable cultivated supply—not assuming this intervention transfers unchanged to every species: [SANBI — plant species and conservation intervention](https://nba.sanbi.org.za/content/species/sp_plants.html).
- **Make permit-applicant eligibility an early governance check.** The DFFE BABS portal describes eligible applicant classes and provides material-transfer agreements, benefit-sharing agreements and community-resolution documents. The project should record an activity-specific determination of whether a permit or notification applies and who is legally eligible to submit it, before research or commercial use begins: [DFFE — BABS clearing house](https://www.dffe.gov.za/bc_projectsprogrammes_babs). This is a scoping prompt, not a claim that every household garden is subject to BABS permitting.
- **Keep product identity distinct from manufacturer authority.** The Department of Health's September 2026 guidance for complementary medicines offered in national pharmaceutical tenders says the applicable Section 22C licence must be considered alongside the relevant SAHPRA-approved DA02 Product List; the product must match the approved listing, and using a licensed third-party manufacturer does not by itself establish authorization of the specific product. Therefore the software needs separate, product-scoped evidence for the manufacturer and the finished product. The guidance is tender-specific and should not be generalized beyond its scope without regulatory review: [NDoH — guidance on complementary medicines in national tenders (September 2026)](https://www.health.gov.za/wp-content/uploads/2026/09/Guidance-on-Complementary-Medicines.pdf).

- **Prospective Gauteng capacity map (no partnership or endorsement implied):** SANBI's Walter Sisulu National Botanical Garden lists education programmes for schools and community groups, with themes including sustainable use of plant resources; the University of Johannesburg's Centre for Plant Metabolomics lists indigenous-plant phytochemical research; and the University of Pretoria lists a multidisciplinary Phytomedicine Programme and medicinal-plant science research. These are plausible places to ask separately about biodiversity education, botanical research, and quality-method expertise—not assumed accredited laboratories, regulatory approvers, manufacturers or committed partners: [SANBI Walter Sisulu NBG education](https://www.sanbi.org/community-initiatives/biodiversity-education-overview/walter-sisulu-nbg-biodiversity-education-programme/), [UJ Centre for Plant Metabolomics](https://www.uj.ac.za/faculties/science/departments-2/biochemistry/research/research-centre-for-plant-metabolomics/), [UP Phytomedicine Programme](https://www.up.ac.za/phytomedicine-programme).

Operational consequence: begin with non-clinical growing education, biodiversity review and a community-governed seedling/cultivation pilot. Do not publish a generic “top medicinal plants” list or allow Symthaea to turn traditional-use records into treatment claims. A later supply/manufacturing pathway needs a separate conservation, rights, product-quality, regulator and buyer review.

- WHO Global Traditional Medicine Strategy 2025–2034: https://www.who.int/teams/integrated-health-services/traditional-complementary-and-integrative-medicine/global-strategies
- WHO guidelines on Good Agricultural and Collection Practices for medicinal plants: https://www.who.int/publications/i/item/9241546271
- WHO GMP for manufacture of herbal medicines, TRS 1010 Annex 1: https://www.who.int/publications/m/item/trs1010-annex1
- WHO supplementary GMP guidance for herbal medicines: https://www.who.int/publications/i/item/9789241547161
- WHO quality-control methods for medicinal plant materials: https://www.who.int/publications/i/item/9241545100
- WHO pharmacovigilance for traditional medicine products: https://www.who.int/publications/i/item/10665-259854
- SAHPRA Guide to Good Manufacturing Practice for Medicines, version 10, updated 24 April 2026: https://www.sahpra.org.za/document/sa-guide-to-good-manufacturing-practice-for-medicines/
- SAHPRA pharmaceutical licensing and manufacture/import/export guidance: https://www.sahpra.org.za/pharma-licencing/
- SAHPRA complementary-medicine guidance: https://www.sahpra.org.za/complementary-medicines-guidelines/
- SAHPRA vigilance guidelines: https://www.sahpra.org.za/vigilance-guidelines/
- South African National Environmental Management: Biodiversity Act 10 of 2004: https://www.gov.za/xh/documents/national-environmental-management-biodiversity-act-0
- DFFE Bioprospecting, Access and Benefit-Sharing (BABS) portal, permits and agreement templates: https://www.dffe.gov.za/bc_projectsprogrammes_babs
- SANBI National Biodiversity Assessment, medicinal plants/conservation status: https://nba.sanbi.org.za/content/context/importance.html
- Department of Health, guidance on complementary medicines offered in national pharmaceutical tenders, September 2026: https://www.health.gov.za/wp-content/uploads/2026/09/Guidance-on-Complementary-Medicines.pdf
- SANBI Walter Sisulu NBG biodiversity education programme: https://www.sanbi.org/community-initiatives/biodiversity-education-overview/walter-sisulu-nbg-biodiversity-education-programme/
- University of Johannesburg Centre for Plant Metabolomics: https://www.uj.ac.za/faculties/science/departments-2/biochemistry/research/research-centre-for-plant-metabolomics/
- University of Pretoria Phytomedicine Programme: https://www.up.ac.za/phytomedicine-programme
- Traditional Health Practitioners Act 22 of 2007: https://www.gov.za/ss/documents/traditional-health-practitioners-act
- WHO sustainable local production ecosystem, 20 July 2026: https://www.who.int/news/item/20-07-2026-who-and-partners-strengthen-global-commitment-to-sustainable-local-manufacturing-for-health
- South African Protection of Personal Information Act 4 of 2013: https://www.gov.za/documents/protection-personal-information-act
- Information Regulator, Personal Privacy Principles (data minimisation and security): https://eservices.inforegulator.org.za/pps/principles.aspx

## Decision requested for the engineering plan

Proceed with the non-clinical garden and provenance software prototype first. Keep the regulated manufacturing path as a future, partner-led workstream with independent quality and regulatory gates. Do not connect live medicine production or clinical recommendations to Symthaea until governance, evidence, authorization and real-world operating partners exist.
