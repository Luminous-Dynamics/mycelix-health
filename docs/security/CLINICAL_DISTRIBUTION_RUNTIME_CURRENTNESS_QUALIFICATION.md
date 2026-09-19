# Clinical Distribution Runtime Currentness v1 Qualification Checklist

## Scope

Qualifies only the composition of:

- one bounded evaluator admission/revocation observation; and
- one bounded positive evaluator lease observation.

It does not qualify conductor provenance by itself, detector science, model clinical validity, practitioner authority, or presentation authority.

## Static contract

- [ ] `VerifiedDistributionEvaluatorCurrentnessV1` is non-serializable;
- [ ] currentness requires both revocation and positive-lease observations;
- [ ] both read boundaries must be `NetworkBackedStableDoubleRead`;
- [ ] revocation count must equal materialized revocation count;
- [ ] any observed revocation fails closed;
- [ ] missing positive lease evidence fails closed;
- [ ] admission action hashes must match exactly;
- [ ] admission proposal digest must match exactly;
- [ ] selected lease must target the exact admission;
- [ ] detector identity must match exactly;
- [ ] structural distribution-policy digest must match exactly;
- [ ] evaluator trust-policy digest must match exactly;
- [ ] external admission-evidence digest is preserved;
- [ ] currentness policy identity is serializer-independent;
- [ ] currentness proof identity is serializer-independent.

## Temporal contract

- [ ] observation start <= completion for both inputs;
- [ ] neither observation may complete after composition time;
- [ ] both observations must satisfy policy freshness;
- [ ] completion timestamp gap must satisfy policy;
- [ ] policy hard cap rejects observation age > 60 seconds;
- [ ] policy hard cap rejects inter-observation gap > 30 seconds;
- [ ] evaluator admission must be active at composition time;
- [ ] selected positive lease must be active at composition time;
- [ ] proof expiry is min(lease expiry, admission expiry, both freshness expiries);
- [ ] an already-expired composition fails;
- [ ] proof is inactive at and after `valid_until_micros`.

## Adversarial unit tests

- [ ] exact matching observations compose;
- [ ] observed revocation rejects;
- [ ] admission A revocation observation + admission B lease observation rejects;
- [ ] stale revocation observation rejects;
- [ ] stale lease observation rejects;
- [ ] detector substitution rejects;
- [ ] structural policy substitution rejects;
- [ ] trust policy substitution rejects;
- [ ] proposal digest substitution rejects;
- [ ] lease target substitution rejects;
- [ ] expired lease rejects;
- [ ] expired admission rejects;
- [ ] future observation rejects;
- [ ] excessive observation gap rejects;
- [ ] malformed revocation count rejects.

## Focused Rust gates

```bash
cargo fmt --check -- crates/clinical-distribution-currentness
cargo check -p mycelix-clinical-distribution-currentness --all-targets
cargo clippy -p mycelix-clinical-distribution-currentness --all-targets -- -D warnings
cargo test -p mycelix-clinical-distribution-currentness
```

## Workspace gate

- [ ] `crates/clinical-distribution-currentness` is a workspace member;
- [ ] direct dependencies are explicit;
- [ ] tests do not rely on undeclared transitive HDI/Holochain crates.

## Mandatory runtime/conductor qualification

Pure Rust composition is not enough for production trust.

A production adapter must independently prove that both serializable observation inputs came from the intended runtime path:

- [ ] exact conductor identity;
- [ ] exact cell/DNA identity;
- [ ] exact installed zome identities;
- [ ] exact coordinator function names;
- [ ] exact source app-entry definitions;
- [ ] modified coordinator cannot substitute fabricated snapshot values;
- [ ] conductor restart/replay preserves referenced action hashes and timestamps;
- [ ] runtime clock used for composition is bound to the qualified observation path;
- [ ] observation collection order/gap satisfies the configured currentness policy.

## Downstream rule

Do not treat admission-only `DistributionTrustReceipt` as sufficient for a future model-backed presentation permit.

The next distribution-trust API must bind the exact `VerifiedDistributionEvaluatorCurrentnessV1` identity into a new trust receipt. Existing admission-only behavior may remain for historical/non-presentation callers, but cannot become the model-backed clinician-presentation path.

## Evidence policy

Branch existence, PR mergeability, static review, or queued CI are not qualification. Record only gates that actually execute successfully against the exact head.