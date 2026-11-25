# Mycelix Health benchmark positioning against the 2026 field

**Snapshot date:** 2026-11-25  
**Scope:** public healthcare-agent benchmarks, interoperability standards, and the research gap addressed by Mycelix Health.  
**Non-claim:** this document does not report a Mycelix model score, clinical outcome, or SOTA win.

## 1. What the current field measures

### HealthAgentBench v2 — broad agent task success

HealthAgentBench v2 reports 75 tasks across nine categories and uses one scored attempt per task, xhigh reasoning, model API access only, and disabled web tools. Its published overall leader is Claude Code with Opus 5.5 at **57.3%** mean task success, followed by Claude Code Sonnet 5.5 and Codex GPT-6-astra at **50.7%** each. Cost is also reported: $4.7/task for the top result and $3.8/task for GPT-6-astra. The benchmark's detailed EHR compositional QA category is much harder than its pooled result: 50% for the top entry and 37.5% for GPT-6-astra.

Sources:
- https://microsoft.github.io/HealthAgentBench/v2/results/
- https://github.com/microsoft/HealthAgentBench

**Interpretation:** a 57.3% pooled success rate is an external reference for agent task completion under that benchmark's conditions. It is not a score for source provenance, patient identity, consent propagation, or safe clinical authority. A Mycelix score from a different corpus cannot be numerically ranked against it.

### LongMedBench — longitudinal clinical reasoning

LongMedBench (MICCAI 2026) uses MIMIC-IV admission records and clinical notes as longitudinal event streams. It contains **355 patients**, averaging **19.72 inpatient visits per patient** and **44.91 medical events per visit**. The evaluation separates factual question answering, temporal reasoning, and long-horizon decision making. Its authors report that current models can use explicit timestamps more effectively than implicit temporal information, and that longer context, RAG, and agent memory improve factual retrieval more than long-horizon decision making.

Source:
- https://papers.miccai.org/miccai-2026/0593-Paper4292.html

**Interpretation:** this validates the choice to separate timeline reconstruction from decision sufficiency. Simply retrieving more history is not the same as using it correctly.

### HealthFlow / EHRFlowBench — executable analysis workflows

HealthFlow introduces EHRFlowBench with **100 expert-adjudicated EHR analysis tasks**, derived from a screen of 51,280 peer-reviewed papers and 118 seed papers. Tasks span the analysis lifecycle, including cohort definition, feature engineering, statistical modeling, visualization, and reporting. The paper reports HealthFlow outperforming compared frameworks across five benchmarks.

Source:
- https://www.nature.com/articles/s41746-026-03097-0

**Interpretation:** a realistic healthcare-agent evaluation must test executable workflow correctness and artifacts, not only natural-language answers. Mycelix should add reproducible data-handling, transformation, and evidence-replay tasks rather than imitate a medical QA leaderboard.

## 2. The defensible differentiation

Mycelix should not compete by claiming a larger medical knowledge score. The differentiated research target is **evidence-qualified action under explicit authority**:

| Dimension | Common agent benchmark emphasis | Mycelix qualification target |
|---|---|---|
| Patient binding | Task-specific correctness | No cross-subject acceptance or disclosure; test tenant + patient binding independently |
| Source authenticity | Often assumed by environment | Issuer/source principal is authenticated, not caller-labelled |
| Resource identity | Record retrieval or answer citation | Logical resource type/ID + source principal + version + payload digest |
| Time | Timestamp QA or temporal order | Clinical event, specimen collection, analysis, result issue, source update, and local ingestion remain distinct |
| Revisions | "Latest" record or RAG | Exact replay idempotency, stale revision history, source-order ambiguity, same-version/different-digest quarantine |
| Semantic disagreement | Answer consistency | Resource integrity collision separated from cross-source proposition contradiction |
| Provenance | Citation presence | Exact source/version/digest/time/semantic lineage match and replayable transformation receipt |
| Authority | Tool permissions are usually fixed | Actor, tenant, subject, purpose, action, recipient, consent, expiry, and revocation are independently re-checked |
| Uncertainty | Confidence or answer accuracy | Coverage-risk curves, calibrated abstention, decision sufficiency, and value-of-information regret |
| Auditability | Agent trace | Independently replayable, content-addressed evidence and execution receipts |

The key paradigm shift is that clinical evidence is not a list of facts sorted by time. It is a **typed evidence graph** containing resource revision, replay, source-order, derivation, support, refutation, and contradiction relationships. The current v1 candidate contract exposes relation types for resource revision, exact replay, payload collision, explicit source-sequence order/ambiguity, and shared assertion agreement/contradiction. Timestamp-only edges are prohibited.

A second shift is to make the authorization boundary executable: a model proposes; the independent policy gate decides whether minimum-necessary evidence can be released or whether an action can be taken. A favorable model score, EVSI, or confidence estimate never grants authority.

## 3. How to compare fairly with SOTA

Use two lanes and report them separately.

**Lane A — external benchmark comparability.** Run a model/agent adapter on HealthAgentBench or another publicly available benchmark only under its published task, model, prompt, tool, retry, and cost protocol. Publish category-level scores and cost/time. Do not mix those results with the synthetic temporal-evidence evaluator.

**Lane B — matched evidence-integrity evaluation.** Run all candidates on the same frozen visible evidence packets and task requests:
1. accept-all / ingestion-time sort;
2. logical-resource + source-sequence baseline;
3. FHIR/RAG timeline baseline;
4. a capable general agent with the same tools and budgets;
5. Mycelix evidence gate without Symthaea decision ranking;
6. Mycelix evidence gate plus Symthaea decision sufficiency/abstention;
7. graph-aware evidence projection plus authority gate.

Freeze model/version, prompt, context, tool schemas, permissions, retries, seeds, wall-time, and cost budgets. Keep candidate and evaluator in different workspaces. Give candidates only the visible manifest, visible evidence, and candidate schemas; retain the answer key in the evaluator workspace.

Report a metric vector rather than one weighted leaderboard number:
- task/exact evidence-view match;
- current timeline precision/recall and pairwise event-time order;
- history retention and correction fidelity;
- quarantine and conflict precision/recall;
- graph node/edge precision/recall and exact graph match;
- source/resource/version/payload-digest/time/semantic citation fidelity;
- wrong-subject, unauthorized, expired, issuer, lineage, and replay critical failures;
- selective risk at common coverage and coverage at common selective risk;
- decision regret, Bayesian EVSI or prior-free robust loss reduction (kept as distinct estimands);
- disclosure volume, acquisition cost, reviewer time, p50/p95/p99 latency, tokens/compute, and reproducibility.

## 4. Qualification sample sizes

The v1 corpus currently has **21 tasks / 34 evidence envelopes but only one synthetic subject binding**. It is a deterministic adversarial coverage set, not an independent reliability sample.

For zero observed failures in (n) independent Bernoulli trials, the one-sided Clopper-Pearson upper bound is:
[
1 - (1 - c)^{1/n}
]
where (c) is the confidence level. At 95% confidence, zero failures in 21 independent trials still permits about 13.3%; zero failures in 299 independent trials are needed to bound the finite-suite rate below 1%. These bounds require an actual independent trial unit. Repeated tasks, events, or records from one subject must be clustered.

Staged target:
- **V2 engineering corpus:** at least 120 tasks, 20+ attack families, at least 60 synthetic subjects, subject-held-out splits, and institution/source-held-out challenges.
- **Final sub-1% finite-suite qualification:** at least 299 independently generated subject clusters with zero critical failures, plus cluster-aware uncertainty reporting. This remains a synthetic benchmark statement—not a real-world clinical risk estimate.

## 5. Interoperability and security boundary

Use official HL7 FHIR R4 validation for FHIR structural conformance; do not treat it as proof of correct patient binding, source authenticity, or application authority. The current PR #312 validator lane is a core R4 fixture lane only.

For autonomous backend connections, use the SMART App Launch Backend Services profile where applicable. It expects pre-authorized client scopes and asymmetric client authentication, with short-lived access tokens; it is not a substitute for Mycelix's own actor/patient/purpose/recipient authority checks. User-facing launch contexts and backend system credentials must not be conflated.

Source:
- https://hl7.org/fhir/smart-app-launch/STU2.2/backend-services.html

No network-exposed deployment should be represented as qualified while #289's HTTP-principal and source-authority boundary remains open.

## 6. Falsifiable targets (not achieved results)

1. Zero wrong-subject and unauthorized releases/actions across the frozen qualification corpus.
2. 100% exact source/resource/version/digest/time/semantic citation fidelity for accepted evidence.
3. At least +10 percentage points exact evidence-view accuracy over the strongest matched baseline at equal tools and compute.
4. Lower selective risk at equal coverage, or higher coverage at equal selective risk.
5. Graph relation exactness with zero invented timestamp-only causal edges.
6. Lower reviewer effort and end-to-end latency without increasing critical safety failures or disclosure burden.

A passing benchmark establishes only what that benchmark tested. It does not establish clinical effectiveness, regulatory compliance, independent cryptographic assurance, or deployment readiness.
