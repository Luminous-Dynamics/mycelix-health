# Temporal Evidence Integrity Benchmark v1

This standalone, dependency-free Rust crate is the first executable slice of [health benchmark programme #288](https://github.com/Luminous-Dynamics/mycelix-health/issues/288). It complements—not replaces—the decision-sufficiency evaluator and FHIR source-revision classifier.

## Research question

Can an agent reconstruct a longitudinal evidence view without allowing arrival order, record-update time, stale revisions, ambiguous identity, missing provenance, or revoked authority to silently become clinical truth?

The benchmark evaluates **evidence operations**, not just answer text. It separates source-level identity from cross-source proposition identity: a FHIR resource ID scopes its own revision history, while an assertion key can connect independently sourced claims that refer to the same concept and effective time.
- which source/version assertions enter the current timeline;
- which valid superseded versions remain available as history;
- which records are quarantined;
- which contradictions remain explicitly unresolved;
- whether citations preserve exact source, version, payload digest, measurement method and unit, terminology system/code/version, clinical event time, specimen collection time, analysis time, result issue time, source `lastUpdated`, and local ingestion time;
- whether timeline order follows event time rather than ingestion order.

## V1 contents

The frozen corpus contains 21 synthetic tasks across 21 scenario families and 34 evidence envelopes. Every event now carries an explicit logical `resource_type` and `resource_id`; resource revisions are therefore grouped by resource identity rather than guessed from event IDs or version strings.. It includes late/backfilled events, source revision ordering, same-version/different-payload collisions, exact replay, wrong-patient references, unauthorized/revoked access, expired evidence, issuer and lineage failures, source conflicts, unknown revision order (including equal source-sequence ties across distinct resource versions), method transitions, terminology-version transitions, and clock skew.

The visible input corpus is `fixtures/v1-task-inputs.tsv` plus `fixtures/v1-events.tsv`; it contains task IDs, target-subject bindings, natural-language task requests and synthetic evidence envelopes only. Method and terminology transitions carry explicit synthetic method, unit, code-system, code and version metadata. Evaluator-only labels are separated into `fixtures/v1-cases.tsv` and `fixtures/v1-answer-key-events.tsv`. The Rust API exposes these as distinct `VisibleTaskInput` and answer-key `Task` types. Never provide either answer-key file to an evaluated agent. These are not real medical records and do not encode clinical thresholds or treatment recommendations.

## Scoring

Per task, the evaluator returns:
- task-identity and disposition correctness;
- current-timeline precision/recall and exact ordered match;
- pairwise temporal-order accuracy;
- retained-history recall and exact match;
- quarantine precision/recall and exact match;
- unresolved-conflict precision/recall and exact match;
- unsafe accepted-event IDs;
- provenance mismatch IDs;
- duplicate accepted-event IDs;
- a hard-gate result.

**Hard gates are not averaged away.** Any acceptance of wrong-patient, unauthenticated, unauthorized, expired, incomplete-lineage, quarantined, or unresolved-conflict evidence fails the task's safety gate. Any source/version/digest/event-time citation mismatch also fails it. All required quarantines and unresolved conflict groups must be surfaced.

The evaluator intentionally reports a set of dimensions rather than one weighted score. A system cannot trade a wrong-patient acceptance for better temporal F1.

## Run

```sh
cargo fmt --locked --manifest-path prototypes/temporal-evidence-integrity-bench/Cargo.toml -- --check
HOST="$(rustc -vV | sed -n 's/^host: //p')"
cargo clippy --manifest-path prototypes/temporal-evidence-integrity-bench/Cargo.toml --target "$HOST" --all-targets -- -D warnings
cargo test --manifest-path prototypes/temporal-evidence-integrity-bench/Cargo.toml --target "$HOST"
```

Or run `sh ./prototypes/temporal-evidence-integrity-bench/run.sh` from the repository root. The script prints SHA-256 digests for both visible input files and both evaluator answer-key files, then executes the deterministic negative-control report through `src/bin/naive_baseline.rs`. The CI job also publishes per-task TSV reports for both baselines in the `temporal-evidence-baseline-matrix-v1` artifact. The CLI receives the answer-free input separately from the evaluator-only truth fixture; candidate agents must never receive the answer-key files.

## Baseline protocol

The crate supplies a typed evaluator, an agent-visible input loader, and an oracle fixture path for contract tests. `Cargo.lock` is checked in and the dedicated CI/local runner uses `--locked`. It also provides a deliberately weak `naive_ingestion_order_baseline` that accepts every visible event as current and sorts by ingestion time. This negative control demonstrates that plausible chronological sorting alone does not preserve revision, identity, authority or conflict semantics. It does **not** contain model outputs or claim a model score. The next experiment must freeze model/agent versions, prompts, tool permissions, task seeds, and resource budgets, then compare:
1. raw ingestion-time sort / accept-all baseline (v1 includes an executable per-task report);
2. source-sequence/resource-identity baseline (v1 includes an executable per-task report, but does not resolve cross-source semantic conflicts);
3. ordinary FHIR/RAG timeline reconstruction;
4. a provenance-aware evidence pipeline;
5. the same pipeline with Symthaea sufficiency and abstention.

All systems must receive identical visible evidence and tool permissions. Report task-level paired results, bootstrap confidence intervals, hard-gate counts, coverage, wall time, tokens/API cost, and reviewer effort. Keep HealthAgentBench, LongMedBench, and EHRFlowBench results as separate external comparators; do not blend their scores into this corpus.


## Statistical interpretation and sample-size discipline

The evaluator exposes `zero_failure_upper_confidence_bound(n, confidence)` and
`minimum_zero_failure_trials(target_rate, confidence)`. For zero observed
critical failures in independent Bernoulli trials, the exact one-sided
Clopper-Pearson upper bound is:

`upper = 1 - (1 - confidence)^(1/n)`

At 95% confidence, zero failures in 21 independent trials still permits an
upper failure probability of about **13.3%**. Approximately **299 independent
trials** with zero failures are required to bound the rate below **1%**;
approximately **2,995** are required to bound it below **0.1%**.

These are illustrative finite-suite statistical bounds, not clinical risk
estimates. V1 has 21 tasks but only **one synthetic subject binding**, so its
tasks are not 21 independent patient-level trials. V1 is an adversarial
coverage and evaluator-contract fixture, not a population reliability sample.
V2 must report patient/subject-clustered bootstrap intervals and use held-out
subject clusters. Do not count repeated tasks, resources, or assertions from
one subject as independent samples. A zero-failure result from synthetic
fixtures does not establish real-world safety, representativeness, or clinical
effectiveness.

## Known scope limits

An earlier v1 predecessor passed its dedicated Rust lane, but the present revision adds a separate candidate contract crate, a second executable baseline, and equal-sequence ambiguity fixtures. Those latest changes require fresh exact-head qualification; no result is claimed for them yet.

This is evaluator infrastructure, not a production FHIR ingestion engine, identity service, consent engine, cryptographic verifier, or clinical model. The evaluator-only role labels are reference answers. Real agent trials must receive `v1-events.tsv` only, not the case manifest or answer-key event labels. No clinical validity, benefit, legal compliance, privacy audit, or deployment claim follows from a passing test.

## External context

- [HealthAgentBench v2](https://microsoft.github.io/HealthAgentBench/v2/): 75 agentic tasks across nine healthcare categories; its published leading baseline is 57.3% under the authors' one-attempt evaluation setup.
- [LongMedBench (MICCAI 2026)](https://papers.miccai.org/miccai-2026/0593-Paper4292.html): 355 longitudinal patients and separate factual, temporal, and long-horizon decision suites. Its paper notes a remaining gap between historical retrieval and long-horizon decision use.
- [EHRFlowBench / HealthFlow](https://www.nature.com/articles/s41746-026-03097-0): 100 expert-curated EHR analysis tasks focused on executable analysis pipelines.

These are context and potential interoperability targets, not directly comparable numerical baselines.


## Candidate/evaluator package boundary (v1)

The candidate contract is a separate Cargo package at
`prototypes/temporal-evidence-agent-contract`. Candidate adapters should depend
on this package only. It exposes:

- `VisibleTaskInput`, with the task request and subject binding;
- `VisibleEvidenceEvent`, with source, resource, revision, temporal, semantic,
  issuer, authority and lineage fields supplied to the candidate;
- `Submission` and `EventCitation`, with a versioned JSON Schema at
  `schema/submission-v1.schema.json`;
- `load_visible_fixture()`, which compiles only the answer-free task manifest
  and visible event envelope.

The evaluator package separately owns `Task`, `EvidenceEvent`, expected
dispositions, expected current/history/quarantine sets, conflict answer keys,
and scoring. The candidate contract has no Cargo dependency on the evaluator.
The dedicated workflow builds/tests the contract by its own manifest and uses
`cargo tree` to fail if the evaluator appears in that dependency graph.

**Operational limitation:** packages in one public repository are not a process
sandbox. For model trials, build the candidate adapter in a separate workspace
and mount only `v1-task-inputs.tsv`, `v1-events.tsv`, and the submission
schema into its runtime. Keep `v1-cases.tsv` and
`v1-answer-key-events.tsv` in a separately permissioned evaluator workspace.
The in-repository package split proves a dependency boundary, not filesystem
isolation against an agent that can inspect the whole checkout.

Local runner also qualifies the candidate contract separately before running
the evaluator and reference baselines.
