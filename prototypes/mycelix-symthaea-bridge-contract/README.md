# Mycelix Health × Symthaea Bridge Contract v1

**Status:** dependency-free contract prototype. No live clinical, gateway, Holochain, Symthaea runtime, or cryptographic verifier integration is included.

## Purpose and ownership

This crate defines the narrow handoff from qualified Mycelix evidence to the existing Symthaea finite-world decision-evaluation interface. It intentionally does not implement EVSI, robust minimax, active sensing, a clinical agent, or an authorization engine.

- Mycelix owns source/tenant/subject identity, FHIR revision and temporal semantics, evidence gates, consent/purpose/recipient checks, disclosure, execution and observation receipts.
- Symthaea owns candidate decision/probe proposals and its existing Bayesian or prior-free robust decision-value calculations.
- An independent evaluator owns hidden states, priors, likelihoods, loss tables and grading in a separately permissioned process.

## Candidate projection

HealthDecisionEpisodeV1 is a field-for-field transport analogue of Symthaea's current CandidateEpisode, ObservedSignal and PublicProbeOption candidate types. The mapping is:

| Bridge field | Symthaea field |
|---|---|
| episode_id | CandidateEpisode.episode_id |
| current_step / decision_deadline_step | Same |
| remaining_probe_budget | Same |
| maximum_worst_case_loss | Same |
| allowed_decision_ids | Same |
| evidence[channel_id, outcome_id] | ObservedSignal |
| permitted_probes[probe_id, acquisition_cost, result_delay_steps] | PublicProbeOption |

The candidate episode contains no patient identifier, source principal, FHIR resource, payload, signature, receipt digest, priors, likelihoods, hidden state, loss table, or evaluator label.

## Independent release and lineage

A candidate signal can only be projected from a VerifiedEvidenceSignalV1. Its fields are private; verify_evidence_release_v1 checks structural requirements and calls the injected EvidenceReceiptVerifierV1.

The production verifier must verify the signature over every field, authenticate source/issuer and all nine gate attestations against configured trust roots, bind them to the same event/payload/subject/tenant/episode/purpose/recipient, check current authority and revocation, and atomically consume the nonce.

A true flag, digest string, caller-provided JSON field, Rust private type, or fake verifier is not cryptographic proof. Tests use a fake verifier solely to exercise the adapter contract. No production verifier is supplied.

The operator-side EvidenceReferenceManifestV1 remains separate from HealthDecisionEpisodeV1. It maps each channel/outcome pair to exact receipt IDs, source principal/connection, event ID, FHIR resource identity/version and payload/source-receipt digests. Do not send this manifest to the candidate process.

## Probe and execution boundary

A verified ProbeOfferReceiptV1 allows a probe to appear in the candidate-visible option list. It does not authorize subsequent execution.

The candidate returns CandidateSubmissionV1. validate_candidate_submission_v1 checks only that decision/probe identifiers were allow-listed. Immediately before acquisition, Mycelix must re-authenticate the principal and re-check subject/tenant, source connection, purpose, recipient, action, resource type, budget, expiry, revocation and one-time-use semantics.

Symthaea cannot grant itself access by returning a positive EVSI, high mutual information, covariance distance or confident decision. Unknown execution outcome means reconcile request/receipt state before a retry or another mutation.

## Uncertainty and missingness

An absent ObservedSignal is not a negative clinical finding. Missing, stale, conflicted, not-collected and unavailable states must be represented as explicit observation outcomes only when their semantics are defined in the observation model. They must never be silently converted to a normal/negative result.

## Run

    cargo fmt --manifest-path prototypes/mycelix-symthaea-bridge-contract/Cargo.toml -- --check
    cargo clippy --manifest-path prototypes/mycelix-symthaea-bridge-contract/Cargo.toml --locked --all-targets -- -D warnings
    cargo test --manifest-path prototypes/mycelix-symthaea-bridge-contract/Cargo.toml --locked

This is a contract test target only. It does not qualify the upstream Symthaea evaluator or establish process isolation, clinical validity, SOTA, production readiness or safety.
