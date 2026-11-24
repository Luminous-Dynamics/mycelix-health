# Decision Sufficiency Benchmark v1

This is an offline, dependency-free reference evaluator for the research track in Health issue #299. It is deliberately separate from production clinical records and authorization code.

## Research question

For a bounded decision-support task, can a system establish that all required evidence obligations are closed, support every accepted claim with exact source identifiers, and choose another observation only when that observation is authorized, relevant, useful and within declared resource/privacy budgets?

The benchmark distinguishes three questions that are often collapsed:

1. **Evidence sufficiency:** is the current evidence set adequate for the declared task?
2. **Claim support:** does each claim cite exact verified sources for the obligation it relies on?
3. **Acquisition choice:** which additional observations are authorized and non-dominated across expected decision-loss reduction, execution cost and privacy exposure?

## What the evaluator does

The reference evaluator:

- Rejects vacuous tasks with no required obligations.
- Requires every required obligation to have at least one Verified record.
- Treats an unresolved conflict as a blocker even when another record appears verified.
- Never counts wrong-subject, expired, unauthorized, unauthenticated, unversioned or semantically unsupported records as verified.
- Allows a verified alternative source to close an obligation when unrelated rejected records are present, while still counting those rejected records.
- Requires claim citations to resolve to exact source IDs and the correct obligation.
- Excludes unauthorized, irrelevant, zero-value and over-budget retrieval actions.
- Computes a Pareto frontier across benefit, cost and privacy instead of hiding policy choices in an arbitrary weighted score.
- Uses stable sorting so the same frozen task produces the same frontier.

The word Verified is a benchmark label, not a production token. A fixture may use it only when its reference data says that patient binding, issuer authenticity, resource version, temporal validity, semantic support, complete lineage and purpose-scoped authorization all passed.

## What it does not do

This is not a clinical decision engine, authorization service, FHIR parser, proof verifier or patient data model. It does not establish diagnostic accuracy, clinical benefit, legal compliance or deployment readiness. It does not decide whether an actual medical observation is clinically meaningful. It grades whether a system respects a frozen evidence contract.

The candidate-action benefit values are synthetic expected decision-loss reductions. They are benchmark labels, not causal estimates derived from real patients.

## Run

From the repository root:

    bash ./scripts/run-decision-sufficiency-benchmark.sh

Equivalent commands:

    cargo fmt --manifest-path prototypes/decision-sufficiency-bench/Cargo.toml -- --check
    HOST="$(rustc -vV | sed -n 's/^host: //p')"
    cargo test --manifest-path prototypes/decision-sufficiency-bench/Cargo.toml --target "$HOST" --locked

The crate has its own minimal workspace and no external dependencies. The runner prints a SHA-256 digest of the fixture catalogue. The Rust tests exercise the reference evaluator against adversarial cases.

## Frozen v1 test families

The catalogue in fixtures/v1-cases.tsv enumerates the v1 cases. The unit tests currently cover:

- complete evidence and missing obligations;
- wrong-patient evidence;
- unresolved contradictions;
- expired, unauthorized and unauthenticated evidence;
- valid alternative sources;
- exact claim-source binding;
- authorized multi-objective retrieval selection;
- unauthorized, irrelevant, zero-value and over-budget action rejection;
- no-retrieval behavior when evidence is already sufficient;
- vacuous and ambiguous task rejection.

## Metrics for the full research benchmark

This reference implementation supplies machine-checkable foundations, not model scores. The 15-row TSV is a frozen scenario catalogue; the executable task inputs are currently constructed in the typed Rust tests. This is an evaluator-contract slice, not yet the complete 25–40-task model-comparison corpus proposed by #299.

The full #299 experiment should additionally report:

- sufficiency precision / recall by blocker category;
- risk-versus-coverage curves and AURC;
- Brier score and calibration error where probabilistic labels are justified;
- exact source/version/subject/time lineage support rate;
- unauthorized acquisition and disclosure count;
- Pareto-frontier coverage and chosen-action opportunity gaps by benefit, cost and privacy separately;
- expected value of sample information (EVSI) for each candidate acquisition policy where the synthetic world provides a defensible probability and decision-loss model; use EVPI as an upper bound where meaningful;
- decision-curve analysis and net benefit at predeclared action thresholds when tasks have valid outcome, benefit and harm labels; compare with default policies such as act-on-all and act-on-none where appropriate;
- wall time, p50/p95/p99 latency, compute, bytes accessed and reviewer time.

EVSI is the expected improvement in decision quality from collecting a specific observation or dataset, not a generic information-gain score. Report its assumptions and sensitivity analyses. For clinical decision support, net benefit should complement discrimination and calibration rather than replace them. The operating threshold must be fixed with appropriate clinical and patient input before evaluation; decision curves do not choose the threshold for the intended population. See the BMJ [clinical prediction model guide](https://www.bmj.com/content/386/bmj-2023-078276.long), [net-benefit primer](https://www.bmj.com/content/352/bmj.i6), and the review of [value-of-information analysis in health policy](https://pmc.ncbi.nlm.nih.gov/articles/PMC7612603/).

Never collapse the three acquisition dimensions into one scalar without stating the policy trade-off. An observation that is high-value but unauthorized is not a candidate. A task already sufficient under its declared evidence obligations should not trigger redundant retrieval.

## Target ladder (ambitions, not results)

- Zero wrong-patient accepted claims in the frozen adversarial suite.
- Zero unauthorized reads, exports or disclosures.
- Zero accepted claims without exact supporting source IDs.
- Zero silent conflict resolution.
- 100% correct sufficiency labels on deterministic reference fixtures.
- At least 70% task coverage only where the frozen task distribution contains at least 70% sufficient cases; otherwise publish the full risk-coverage curve.
- No more than 10% of oracle-achievable decision-loss reduction left unrealized on tasks with defensible synthetic oracle labels, while reporting cost and privacy separately.
- At least +10 percentage points over the strongest matched baseline on provenance-aware temporal tasks, with paired confidence intervals and no calibration or abstention regression.

These are research targets. No model has been evaluated by this crate, and no SOTA improvement is claimed.

## Versioning

Any change to case inputs, evidence obligations, action labels, budgets or expected outcomes must create a new corpus version. Record the exact repository commit, fixture digest, Rust version, model and tool versions, prompts, policy, random seeds and test outputs for every model evaluation.
