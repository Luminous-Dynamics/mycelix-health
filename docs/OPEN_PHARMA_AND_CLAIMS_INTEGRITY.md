# Open Pharmaceuticals and Health Integrity Program

**Status:** proposal and engineering roadmap; not a clinical, regulatory, reimbursement, or fraud determination system.  
**Initial scope:** open scientific evidence, safe research workflows, and auditable claims-review signals.  
**Core rule:** a model prediction is a lead to investigate, never proof of efficacy, safety, fraud, or misconduct.

## 1. Why this program

Symthaea, Mycelix, Sol-Atlas, and Mycelix-Health could form a useful open infrastructure stack for two connected public-interest goals:

1. Make pharmaceutical research more reproducible, inspectable, and directed toward unmet health needs.
2. Help health systems find billing errors, unsafe inconsistencies, and suspicious claim patterns while protecting patients and providers from unsupported accusations or automated denial.

These goals should share an evidence and provenance layer, but must not be collapsed into one authority system. Research hypotheses, clinical judgments, claims anomalies, and confirmed misconduct are different kinds of statements and need different evidence and review.

This document proposes a staged research program. It does **not** claim that the current software has discovered a medicine, validated a clinical predictor, or demonstrated fraud-detection performance.

## Workstream 0 — Affordable, quality-assured medicines

Open pharmaceutical research is only part of the access problem. Many established medicines already exist but remain too expensive, unavailable, or vulnerable to interrupted supply. We should work toward the **lowest sustainable quality-assured landed cost**, not simply the lowest quoted price.

A dedicated proposal now covers an auditable medicine-affordability and resilient-procurement model: [Global Medicines Affordability and Resilient Procurement](GLOBAL_MEDICINES_AFFORDABILITY_AND_RESILIENT_PROCUREMENT.md), tracked in [MED-PRICE-001 / #273](https://github.com/Luminous-Dynamics/mycelix-health/issues/273).

The initial implementation is synthetic-data-first. It will compare price-only, lowest-eligible-landed-cost and resilience-constrained procurement scenarios; normalize product presentation, currency, tender channel and logistics basis; hard-gate quality/regulatory/licensing eligibility; and account for demand uncertainty, expiry, stockout risk and supplier concentration. Public price data may be used only when source terms, date, product/unit comparability and evidence provenance are recorded.

Useful existing mechanisms include quality-assured generic/biosimilar competition, pooled/regional procurement, transparent tenders, patent-pool licensing, and more reliable demand/distribution planning. None guarantees lower cost in every context; the simulator must disclose assumptions, risks and sensitivity rather than presenting hypothetical savings as realized outcomes. Clinical safety, GMP, market authorization, patent/licensing obligations and procurement law remain hard boundaries.

Luminous stack boundary: Mycelix owns canonical evidence identity and decision receipts; Symthaea can propose forecasts and scenario analysis; Sol-Atlas can expose time/geography comparisons; Mycelix-Health owns medicine and procurement domain contracts. No single score decides both medicine quality and price, and no optimizer directly awards or pays a supplier.

## 2. Division of responsibility

| Component | Proposed responsibility | Must not become |
|---|---|---|
| **Sol-Atlas** | Render and explore scientific datasets and publications, preserving source/version/license/retrieval metadata and stable provenance references. Biomedical evidence ingestion is a proposed extension, not an existing claimed capability. | An oracle that treats an indexed record as true merely because it was indexed. |
  
The Sol-Atlas integration seam is tracked separately in [ATLAS-DSCI-001](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/99), building on its existing evidence-native terminal and EPI projection work.
| **Symthaea** | Generate and rank research hypotheses; compare models and baselines; expose uncertainty, applicability limits, counter-evidence, and distribution shift; run reproducible analyses. | A prescriber, regulator, lab, court, insurer, or autonomous fraud adjudicator. |
| **Mycelix** | Provide bounded collaboration, identity/credential evidence, consent and capability boundaries, signed decisions, provenance, and auditable correction/appeal history. | A universal trust score that decides clinical truth or guilt. |
| **Mycelix-Health** | Own healthcare-specific contracts for medications, trials, laboratory evidence, pharmacovigilance, claims, patient consent, and review workflows. | A replacement for licensed clinical practice, ethics review, regulators, or qualified quality systems. |
| **Nix / CI** | Freeze toolchains, datasets, manifests, configuration, and exact source subjects so analyses can be reproduced and compared. | Treating a queued workflow as a pass or inferring scientific validity from a green build. |

Keep clinical PHI out of public research artifacts. Publish minimized metadata and non-sensitive evidence references; handle any authorized patient-level computation within a separately reviewed, access-controlled environment.

## 3. Workstream A — Open pharmaceutical research

### 3.1 Start with evidence quality and repurposing

The first milestone should be a transparent **drug–target–disease evidence graph**, not a model that proposes novel molecules and not a claim that a molecule works.

An evidence record should preserve, at minimum:

- stable source identifier and publisher;
- source/version and retrieval timestamp;
- applicable license and permitted reuse;
- exact dataset or document digest;
- entity identifiers and mapping provenance;
- reported result, units, assay/context, comparator, and uncertainty where available;
- study design, population, limitations, and potential conflicts;
- correction, retraction, or supersession state.

Conflicting results must remain visible. Missing evidence is not negative evidence, and an absence of reported toxicity is not proof of safety.

Potential public starting sources include the [Open Targets Platform](https://platform.opentargets.org/), [PubChem](https://pubchem.ncbi.nlm.nih.gov/), [ChEMBL](https://www.ebi.ac.uk/chembl/), published literature, public trial registries, WHO essential-medicines material, and relevant [SAHPRA pharmacovigilance guidance](https://www.sahpra.org.za/vigilance-guidelines/). Source admission must be recordable and licence-aware:

- The [Open Targets Platform licence documentation](https://platform-docs.opentargets.org/licence) states that its platform data are CC0 1.0 and its codebases are Apache 2.0; preserve the distinction between the data and code licences.
- [PubChem's download documentation](https://pubchem.ncbi.nlm.nih.gov/docs/downloads), updated 26 March 2026, says most data are downloadable but contributor licensing exceptions may apply. Capture source-level terms instead of assigning a blanket licence to all PubChem content.
- [EMBL-EBI licensing guidance](https://www.ebi.ac.uk/licencing/) explains that data resources can have different governance and terms; do not assume that every hosted resource shares the same licence.
- SAHPRA's [current pharmacovigilance guidance index](https://www.sahpra.org.za/vigilance-guidelines/) lists, among others, the human-medicine post-marketing adverse-reaction guideline updated 27 August 2026 (version 11) and the industry e-reporting guideline updated 19 August 2026 (version 1). Ingest only the applicable current version and preserve the regulatory jurisdiction and effective document identity.

The [Open Source Pharma Foundation](https://www.ospfound.org/about.html) is a relevant ecosystem reference because it explicitly works toward open, affordable pharmaceutical research and development, with attention to unmet needs in the Global South.

### 3.2 Proposed pipeline

1. **Need definition.** Select a disease or medicine-access problem with clinicians, pharmacists, affected communities, and public-health researchers. Publish the selection criteria and conflicts of interest.
2. **Data admission.** Import only lawfully usable data. Preserve source-level provenance and freeze an immutable input manifest.
3. **Baseline replication.** Reproduce a published result or compare a proposed method with a straightforward statistical baseline before using a complex model.
4. **Evidence synthesis.** Connect disease/target, compound, assay, trial, safety, and access facts while preserving each relationship's source and limitations.
5. **Hypothesis prioritization.** Rank evidence gaps or repurposing candidates using preregistered criteria. Separate observed evidence from model-derived inference. Show what would change the ranking.
6. **Independent computational checks.** Use time-aware and source-aware holdouts, leakage checks, negative controls, calibration, sensitivity analyses, and a distinct evaluator. Molecular docking or free-energy estimates, if later used, remain computational evidence and not proof of clinical benefit.
7. **External validation boundary.** Any wet-lab work needs qualified facilities, appropriate biosafety, assay controls, documented methods, and independent replication. Human research needs independent ethics review, informed consent where applicable, and appropriate regulatory authorization. Manufacturing requires validated quality systems and applicable good-manufacturing-practice controls.
8. **Open release.** Release code, manifests, analysis methods, eligible data or links, negative results, model cards, uncertainty, and reproducible reports under licenses that actually permit the release. Do not publish patient records or misrepresent a computational ranking as a treatment recommendation.

### 3.3 Evidence gates

Every stage returns one of **SupportedForNextStage**, **Indeterminate**, or **RejectedForThisUse**, plus machine-readable reasons and the exact artifacts examined. These are workflow statuses, not claims of clinical safety.

A stage cannot pass merely because the model returned a score. Required evidence includes input completeness, applicability to the intended question, uncertainty handling, baseline comparison, reproducibility, independent checks, and any stage-specific external verification.

### 3.4 Pharmacogenomics is research-only until repaired and validated

Symthaea's current [pharmacogenomics validation note](https://github.com/Luminous-Dynamics/symthaea/blob/main/papers/applications/pgx-health-equity/HWE_VALIDATION.md) documents important limitations in the current Hardy–Weinberg-based phenotype approximation, including missing alleles, loss of diplotype-specific information, and substantial discrepancies for some population/phenotype combinations.

Until that model has been corrected and independently validated for a defined context, it must not drive patient-specific dosing, medicine selection, or medication clearance. In particular:

- don't convert coarse population/ancestry averages into individual prescribing advice;
- represent unknown, missing, under-covered, and conflicting genotypes explicitly;
- preserve the exact allele definitions, phenotype translation rules, guideline version, source population, and validation corpus;
- validate against appropriate diplotype/activity-score standards and independent clinical references;
- prohibit a **Clear** or **Safe** conclusion when required genotype, drug, guideline, or context coverage is missing;
- report subgroup uncertainty and poor coverage as a reason to abstain, not to fabricate confidence.

This is a validation and safety boundary, not a claim that pharmacogenomics is inherently unsuitable.

## 4. Workstream B — Medication safety and pharmacovigilance

Build on the existing Mycelix-Health work rather than duplicating it:

- [Issue #46: coverage-aware medication-safety assessment](https://github.com/Luminous-Dynamics/mycelix-health/issues/46) calls out the unsafe equivalence between “no finding” and “sufficient evidence of safety.”
- [Issue #47: trusted evaluator and knowledge admission](https://github.com/Luminous-Dynamics/mycelix-health/issues/47) requires an admitted evaluator and versioned knowledge source rather than accepting caller-declared evidence.
- [SAHPRA pharmacovigilance guidance](https://www.sahpra.org.za/vigilance-guidelines/) is a jurisdiction-relevant regulatory reference.

The shared contract should bind the exact medication artifact, relevant patient-context snapshot, policy, admitted knowledge artifact/version, coverage, evaluator identity/version, time window, and review result. Missing or stale data should result in **Indeterminate**, not a default clearance. A model should surface possible interactions or adverse-event signals for qualified review, never silently amend a prescription.

Pharmacovigilance signals must preserve the difference between an event report, a suspected association, an assessed causal relationship, and a confirmed finding. Deduplication and clustering may help researchers but must not erase the original reports or make causality claims without the required evidence.

## 5. Workstream C — Claims integrity without automated punishment

### 5.1 Scope and terminology

The first claims-integrity system should help authorized reviewers prioritize records for examination. It should distinguish:

- **data/processing error** — malformed, duplicated, misrouted, or inconsistent records;
- **possible billing anomaly** — a pattern requiring explanation;
- **clinical/coding disagreement** — a question that may need professional interpretation;
- **possible waste or abuse** — a review hypothesis under applicable policy;
- **suspected fraud** — a serious allegation that requires evidence and due process;
- **substantiated finding** — an outcome established by an authorized process, not by a model score.

Do not collapse these categories. “Outlier,” “expensive,” “unusual,” or “unexplained” does not mean fraudulent.

### 5.2 Signals to test first

Use a synthetic or expressly authorized, appropriately de-identified claims corpus to test interpretable signals such as:

- exact and near-duplicate claims, including reversal and resubmission relationships;
- inconsistent dates, impossible sequence relationships, or mismatched code/version references;
- conflicting authorization, eligibility, dispensing, and claim-line evidence;
- unusual billing combinations relative to an appropriate specialty, setting, case mix, and time period;
- abrupt shifts following a coding-policy or payment-rule change;
- suspicious network patterns that are independently explainable from the underlying records.

These patterns are leads, not findings. Avoid naive outlier scoring that punishes rural clinics, small practices, specialist referral centers, high-acuity providers, or populations with different access needs. A policy or coding change must not be mistaken for a sudden increase in wrongdoing.

### 5.3 Non-negotiable enforcement boundary

An **AnomalySignal** is non-authoritative. A model output alone must never deny care, block access to a medicine, terminate coverage, freeze payment, blacklist a patient/provider, or establish misconduct.

Before an adverse action, the relevant authorized organization must use its own applicable rules, review the underlying evidence, give affected parties a meaningful explanation and opportunity to correct or appeal, and record the reasoned decision. The system must preserve contrary evidence and reviewer disagreement. Emergency and medically necessary care must not be held hostage to an anomaly score.

Do not use broad Mycelix reputation or MATL scores as a shortcut for clinical quality, fraud, or culpability. Provider identity and licensing evidence can establish scoped facts such as credential status; it does not prove that a particular claim is valid or invalid.

### 5.4 Proposed evidence contracts

The first implementation should be a deterministic, offline Rust engine with versioned inputs and outputs. Suggested contracts:

- **ClaimEvidenceBundleV1:** canonical claim/line identifiers, source records, source timestamps, code-set versions, applicable policy digest, and completeness state;
- **AnomalySignalV1:** rule/model version, exact input digest, signal type, reason codes, baseline/cohort definition, uncertainty, missingness, and non-authoritative status;
- **ReviewCaseV1:** linked signals and evidence, counter-evidence, assigned reviewer scope, and deadlines;
- **ReviewDecisionV1:** the authorized human review outcome, evidence references, rationale, and signer identity;
- **CorrectionAppealReceiptV1:** correction request, response, appeal, resolution, and complete append-only history.

These names are proposals, not existing implemented contracts. Do not store unnecessary patient identifiers in public or cross-organization receipts. Public receipts should commit to the evidence needed for audit without exposing raw PHI.

### 5.5 Privacy, fairness, and governance

Begin with synthetic records and generated edge cases. Any later real-data evaluation requires a lawful basis, purpose limitation, minimum necessary fields, access logging, retention controls, security review, and the appropriate agreements. South African deployments must assess the [Protection of Personal Information Act (POPIA)](https://www.gov.za/documents/protection-personal-information-act) and any other applicable jurisdictional requirements. Health information is especially sensitive; hashing identifiers alone does not make the underlying record anonymous.

Measure performance beyond aggregate accuracy: precision and recall against independently adjudicated labels, false-positive burden, reviewer overturn/appeal rates, time spent per case, calibration, data-missingness sensitivity, and disparities across legitimate provider and care settings where lawful evaluation data exists. A result is not acceptable if fewer false negatives are purchased through an excessive number of false allegations or delayed care.

## 6. Initial implementation plan

### Phase 0 — contracts and evidence, before model sophistication

- Write versioned schemas and explicit evidence states.
- Create synthetic claims and medication-safety fixtures, with both ordinary cases and adversarial edge cases.
- Create a source/license registry for open research inputs.
- Freeze a deterministic baseline and reproducible dataset manifest.
- Add tests for missing data, stale knowledge, duplicates, policy changes, conflicting evidence, and unsupported assumptions.

### Phase 1 — offline evaluation

- Build deterministic claim-integrity rules and a transparent case queue.
- Generate an evaluation report from seeded synthetic ground truth; clearly label synthetic results as such.
- Replay the same manifest and verify byte-stable or explicitly normalized outputs.
- Add separate evaluator logic so the component that produces a signal is not its sole authority to accept that signal.
- For pharmacogenomics and medication safety, maintain fail-closed **Indeterminate** behavior wherever known coverage gaps remain.

### Phase 2 — externally reviewed retrospective study

Only after the offline harness is stable, seek a qualified partner for retrospective evaluation using authorized data and appropriately governed review. Keep the system in shadow mode: it proposes cases, while existing authorized processes remain responsible for decisions. Publish aggregate methods and limitations without exposing patient/provider data.

### Phase 3 — open pharmaceutical research pilot

Choose one bounded question with clear public-interest value and accessible data—preferably a reproducible evidence synthesis or approved-medicine repurposing assessment. Pre-register the analysis plan, freeze data and code versions, evaluate against a conventional baseline, publish failed hypotheses, and seek a qualified lab/research partner before making claims that require experiments.

These phases are sequenced by evidence, not by calendar promises. A phase advances only when its acceptance criteria are met.

## 7. Acceptance criteria

The first milestone is successful only when the program can demonstrate all of the following on its declared test corpus:

- every signal links to exact inputs, code/model version, policy, and source provenance;
- missing, stale, or incompatible inputs fail closed or return **Indeterminate**;
- a signal cannot be converted into an enforcement decision by the signal producer;
- a reviewer can inspect supporting and contradicting evidence and record a correction or appeal;
- deterministic reruns identify the same source subject and disclose any non-determinism;
- the evaluation report includes false positives, false negatives, subgroup/setting caveats, and limitations;
- no claim is made about clinical effectiveness, drug safety, fraud reduction, or production readiness beyond the evidence actually obtained.

For pharmaceutical research, add independent reproduction of the computational baseline and explicit gates for laboratory, clinical, manufacturing, and regulatory evidence before any stronger claim is made.

## 8. Governance and external references

Clinical scientists, pharmacists, biostatisticians, public-health experts, patient advocates, and privacy/security reviewers should help define the intended use and review process. This does not mean offline engineering must wait for a clinical trial; it means clinical or enforcement use is held behind the evidence and governance appropriate to that use.

References to start from:

- [Open Targets Platform](https://platform.opentargets.org/) — open target/disease prioritization data and tooling.
- [PubChem](https://pubchem.ncbi.nlm.nih.gov/) — public chemical, bioactivity, safety, and literature-linked records.
- [ChEMBL](https://www.ebi.ac.uk/chembl/) — curated bioactivity data.
- [Open Source Pharma Foundation](https://www.ospfound.org/about.html) — open pharma ecosystem and unmet health needs.
- [WHO: Ethics and governance of AI for health](https://www.who.int/publications/i/item/9789240029200) — human rights, autonomy, transparency, and accountability principles.
- [FDA/EMA: Guiding Principles of Good AI Practice in Drug Development (January 2026)](https://www.fda.gov/media/189581/download) — joint lifecycle-oriented principles. Check the legal status of each related document; principles and draft guidance must not be described as binding requirements unless the applicable authority says so.
- [SAHPRA: pharmacovigilance guidance](https://www.sahpra.org.za/vigilance-guidelines/) — South African medicine-safety obligations and guidance.
- [South Africa: POPIA](https://www.gov.za/documents/protection-personal-information-act) — privacy law reference.

## 9. Explicit non-claims

This proposal does not claim that Symthaea has discovered a drug, that its pharmacogenomics model can guide individual treatment, that Mycelix-Health is approved for production clinical use, that a claims anomaly proves fraud, or that the system can eliminate healthcare fraud. It defines a path toward testable, reproducible, safety-bounded work.

The central design principle is simple: **make useful evidence easier to produce, inspect, challenge, and reuse—while keeping clinical decisions and adverse enforcement behind qualified, accountable processes.**
