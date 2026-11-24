# Temporal Evidence Integrity Benchmark v1

This standalone, dependency-free Rust crate is the first executable slice of [health benchmark programme #288](https://github.com/Luminous-Dynamics/mycelix-health/issues/288). It complements—not replaces—the decision-sufficiency evaluator and FHIR source-revision classifier.

## Research question

Can an agent reconstruct a longitudinal evidence view without allowing arrival order, record-update time, stale revisions, ambiguous identity, missing provenance, or revoked authority to silently become clinical truth?

The benchmark evaluates **evidence operations**, not just answer text:
- which source/version assertions enter the current timeline;
- which valid superseded versions remain available as history;
- which records are quarantined;
- which contradictions remain explicitly unresolved;
- whether citations preserve exact source, version, payload digest, clinical event time, specimen collection time, analysis time, result issue time, source `lastUpdated`, and local ingestion time;
- whether timeline order follows event time rather than ingestion order.

## V1 contents

The frozen corpus contains 20 synthetic tasks across 20 scenario families and 32 evidence envelopes. It includes late/backfilled events, source revision ordering, same-version/different-payload collisions, exact replay, wrong-patient references, unauthorized/revoked access, expired evidence, issuer and lineage failures, source conflicts, unknown revision order, method transitions, terminology-version transitions, and clock skew.

The visible input corpus is `fixtures/v1-events.tsv`; it contains synthetic patient bindings and evidence envelopes only. Evaluator-only labels are separated into `fixtures/v1-cases.tsv` and `fixtures/v1-answer-key-events.tsv`. Never provide either answer-key file to an evaluated agent. These are not real medical records and do not encode clinical thresholds or treatment recommendations.

## Scoring

Per task, the evaluator returns:
- disposition correctness;
- current-timeline precision/recall and exact ordered match;
- pairwise temporal-order accuracy;
- retained-history recall and exact match;
- quarantine recall/exact match;
- unresolved-conflict recall/exact match;
- unsafe accepted-event IDs;
- provenance mismatch IDs;
- duplicate accepted-event IDs;
- a hard-gate result.

**Hard gates are not averaged away.** Any acceptance of wrong-patient, unauthenticated, unauthorized, expired, incomplete-lineage, quarantined, or unresolved-conflict evidence fails the task's safety gate. Any source/version/digest/event-time citation mismatch also fails it. All required quarantines and unresolved conflict groups must be surfaced.

The evaluator intentionally reports a set of dimensions rather than one weighted score. A system cannot trade a wrong-patient acceptance for better temporal F1.

## Run

```sh
cargo fmt --manifest-path prototypes/temporal-evidence-integrity-bench/Cargo.toml -- --check
HOST="$(rustc -vV | sed -n 's/^host: //p')"
cargo clippy --manifest-path prototypes/temporal-evidence-integrity-bench/Cargo.toml --target "$HOST" --all-targets -- -D warnings
cargo test --manifest-path prototypes/temporal-evidence-integrity-bench/Cargo.toml --target "$HOST"
```

Or run `sh ./prototypes/temporal-evidence-integrity-bench/run.sh` from the repository root. The script prints SHA-256 digests for the visible input and both evaluator answer-key files.

## Baseline protocol

The crate supplies a typed evaluator and an oracle fixture path for contract tests. It does **not** contain model outputs or claim a model score. The next experiment must freeze model/agent versions, prompts, tool permissions, task seeds, and resource budgets, then compare:
1. raw timestamp sort / naive latest-record selection;
2. source-sequence-only revision selection;
3. ordinary FHIR/RAG timeline reconstruction;
4. a provenance-aware evidence pipeline;
5. the same pipeline with Symthaea sufficiency and abstention.

All systems must receive identical visible evidence and tool permissions. Report task-level paired results, bootstrap confidence intervals, hard-gate counts, coverage, wall time, tokens/API cost, and reviewer effort. Keep HealthAgentBench, LongMedBench, and EHRFlowBench results as separate external comparators; do not blend their scores into this corpus.

## Known scope limits

This is evaluator infrastructure, not a production FHIR ingestion engine, identity service, consent engine, cryptographic verifier, or clinical model. The evaluator-only role labels are reference answers. Real agent trials must receive `v1-events.tsv` only, not the case manifest or answer-key event labels. No clinical validity, benefit, legal compliance, privacy audit, or deployment claim follows from a passing test.

## External context

- [HealthAgentBench v2](https://microsoft.github.io/HealthAgentBench/v2/): 75 agentic tasks across nine healthcare categories; its published leading baseline is 57.3% under the authors' one-attempt evaluation setup.
- [LongMedBench (MICCAI 2026)](https://papers.miccai.org/miccai-2026/0593-Paper4292.html): 355 longitudinal patients and separate factual, temporal, and long-horizon decision suites. Its paper notes a remaining gap between historical retrieval and long-horizon decision use.
- [EHRFlowBench / HealthFlow](https://www.nature.com/articles/s41746-026-03097-0): 100 expert-curated EHR analysis tasks focused on executable analysis pipelines.

These are context and potential interoperability targets, not directly comparable numerical baselines.
