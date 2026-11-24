# Mycelix Health × Symthaea decision-value bridge v1

**Status:** interface proposal grounded in existing public evaluators; no clinical execution integration or model result is claimed.  
**Date:** 2026-11-24

## 1. Why compose rather than duplicate

Mycelix Health and Symthaea have different responsibilities.

- **Mycelix Health** owns patient/tenant/source identity, evidence custody, FHIR revision and time semantics, issuer and lineage verification, consent/application authority, conflict state, and the final decision/evidence receipt.
- **Symthaea** proposes candidate decisions or information-gathering actions and estimates their decision value under a declared information set, uncertainty model, risk gate, cost budget and deadline.
- **An independent evaluator** owns hidden truth, priors, likelihoods, loss tables, held-out labels and grading. It must not share those assets with the candidate process.

This uses rather than recreates the existing Symthaea `research/decision-predictability-benchmark` finite-world evaluator. Its API already separates candidate-visible `CandidateEpisode`, `ObservedSignal`, `PublicProbeOption`, and `CandidateSubmission` from evaluator-side `EvaluatorWorld`, `StateModel`, `EvidenceChannelModel` and `ProbeModel`. It implements both a Bayesian EVSI route and a distinct prior-free robust minimax route. The upstream README records a 16/16 focused exact-head test pass for those evaluator primitives and explicitly says that its Rust type boundary is not a process-security boundary.

**Integration rule:** do not duplicate an EVSI or robust decision-value engine in Mycelix Health. The Health evaluator remains responsible for evidence sufficiency and hard authority gates; Symthaea owns the decision-value calculation. A typed adapter should bind the two.

## 2. Trust boundary and data flow

```text
External EHR / laboratory / patient-authorized source
                 |
                 v
Mycelix identity + source custody + revision / temporal reconciliation
                 |
                 v
Independent issuer, lineage, patient-binding and authority checks
                 |
        +--------+---------+
        |                  |
        v                  v
Evidence packet       Hard-gate report
(source-bound,        (identity, issuer,
versioned, timed)      freshness, lineage,
        |              authority, conflict)
        +--------+---------+
                 |
       candidate-visible projection
       (minimum necessary disclosure)
                 |
                 v
Symthaea candidate policy
(decision / probe proposal, uncertainty)
                 |
                 v
Mycelix re-checks authority, purpose, scope,
recipient, consent, budget, deadline and expiry
                 |
         authorized request only
                 |
                 v
Source / reviewer executes and returns
a source-bound observation receipt
                 |
                 v
Mycelix revalidates receipt and updates evidence view
                 |
                 v
Independent evaluator replays the episode
```

Symthaea never grants itself patient access, consent, signing authority, or permission to execute. A positive EVSI, mutual-information score, covariance separation score or model confidence is not an authorization.

## 3. Candidate/evaluator mapping

| Health-side fact or operation | Candidate-visible projection | Evaluator-only / authority side |
|---|---|---|
| Pseudonymous episode and target subject | opaque `episode_id` and minimum-necessary subject binding | re-identification mapping and true patient identity |
| Verified evidence | bounded `ObservedSignal { channel_id, outcome_id }` plus explicitly released evidence claims | original source payload, signature/issuer proof, full lineage, hidden labels |
| Missing/stale/contested evidence | explicit states; never infer negative truth from missingness | exact gold reason and hidden correct disposition |
| Available decisions | allowlisted `allowed_decision_ids` | policy and loss table authoring |
| Additional information | only currently permitted `PublicProbeOption` values | real consent, recipient, source and purpose authorizations |
| Candidate response | `CandidateSubmission { decision_id, requested_probe_id }` | independently graded decision, request legality, regret and loss |
| Observation result | a new verified signal after source receipt | hidden world state and outcome likelihoods |

The finite-world API can accept public evidence channels, allowed decisions and permitted probes without exposing priors, likelihood magnitudes or task-loss tables. The bridge must preserve that separation. Do not place PHI or raw clinical documents in the benchmark's finite-world hidden model, and do not treat the public Rust types as a security sandbox.

## 4. Evidence state is not clinical truth

The Health evidence gate should keep these dimensions separate:

1. subject identity and patient binding;
2. issuer authenticity;
3. source/resource version and payload digest;
4. clinical event time;
5. specimen collection, analysis performed, result issued, source `lastUpdated`, and local ingestion times;
6. source/transform lineage;
7. application authority, purpose, recipient, expiry and revocation;
8. unresolved conflict and source-order uncertainty;
9. clinical semantic support.

A record can be authentic but unauthorized for this application; current but from an incompetent source; validly signed but wrong-patient; or correctly versioned but clinically conflicted. These are different states and must not collapse to a single `verified=true` bit.

The candidate should receive only the checks and evidence claims permitted for the task. The evaluator can test whether the candidate correctly uses those checks, but the candidate must not be able to forge or upgrade the check results.

## 5. Bayesian and prior-free decision value

Use the Symthaea Bayesian evaluator only where the benchmark has defensible prior probabilities and observation likelihoods. It computes information-set Bayes risk, candidate loss/regret, probe outcome value, acquisition cost, and EVSI. The model must not invent probabilities to make an experiment rankable.

Use the separate prior-free robust evaluator when the defensible input is a set of possible states and outcome-support sets. It reports worst-case minimax loss reduction, not expected value. Preserve the output type and label; never translate robust loss reduction or Mahalanobis distance into a probability or EVSI.

Keep these metrics separate:

- information gain / mutual information;
- Bayesian EVSI and net EVSI;
- robust worst-case decision-loss reduction;
- candidate decision regret;
- risk-gate violations;
- acquisition and disclosure cost;
- deadline feasibility.

High information gain with zero decision value is a required negative control. A lower-information observation that changes the optimal action can have positive decision value.

## 6. Hard gates before candidate ranking

Before Symthaea sees an evidence item or an observation option, Mycelix must enforce the applicable hard gates:

- exact tenant and subject binding;
- authenticated source and issuer;
- valid source/resource version and digest;
- explicit as-of time semantics;
- complete required lineage;
- current purpose/action/recipient authority and consent;
- no silent conflict resolution;
- no revoked, expired or replayed capability.

A failure is not merely a penalty inside the Symthaea loss function. It blocks release or execution. No weighted score or EVSI can compensate for a wrong-patient disclosure or unauthorized action.

## 7. Benchmark integration protocol

Use the Health temporal evidence corpus as the evidence/identity/revision layer, and the Symthaea finite-world evaluator as the decision-value layer.

Matched conditions:

1. naive latest-ingestion baseline;
2. source-sequence-only evidence projection;
3. FHIR/RAG timeline baseline;
4. strongest general agent with identical visible evidence and tool permissions;
5. Mycelix evidence gate without Symthaea decision-value ranking;
6. Mycelix evidence gate plus Symthaea Bayesian EVSI where assumptions are licensed;
7. Mycelix evidence gate plus Symthaea prior-free robust ranking where priors are not justified.

Freeze agent/version, prompts, visible packet, permitted tools, budget, seeds, loss rules, likelihoods, evaluation process and retry policy. Use synthetic data only until a separately approved study protocol exists. Keep candidate, evaluator and execution credentials in separate processes/environments.

Report separate vectors:
- evidence exact-match and DTEC;
- current timeline precision/recall and temporal order;
- source/version/digest/time/semantic citation fidelity;
- history, quarantine and conflict precision/recall;
- wrong-subject and unauthorized release counts;
- decision regret and worst-regime loss;
- EVSI or robust improvement according to the declared mode;
- decision coverage, risk at common coverage, and abstention correctness;
- data over-collection/disclosure;
- p50/p95/p99 latency, tokens/compute, source acquisition cost, and reviewer time.

No single weighted leaderboard score is the primary result.

## 8. Research targets (not results)

- Zero wrong-subject releases and zero unauthorized observations/actions in the frozen qualification corpus; publish exact confidence bounds.
- 100% exact source, version, digest and temporal-field matching for accepted evidence.
- At least +10 percentage points in exact evidence-view accuracy versus the strongest matched evidence baseline at equal tool and compute budget.
- Lower selective decision risk at the same coverage, or higher coverage at the same selective risk.
- Decision regret no more than 10% of the oracle-achievable loss reduction only in tasks where the information-set oracle and loss model are independently justified.
- No increase in disclosure burden, unauthorized action, or critical false acceptance while reducing review time.

These are falsifiable research targets, not clinical claims or production acceptance criteria.

## 9. Existing owners and implementation boundaries

- Mycelix Health #288 / #317 own the synthetic evidence benchmark and matched health-agent matrix.
- Mycelix Health #299 and PR #304 own health-specific evidence sufficiency, risk/coverage and review-cost evaluation.
- Symthaea #12546 and `research/decision-predictability-benchmark` own finite-world Bayesian and robust decision-value evaluation.
- Symthaea #9817 / PR #9821 own active sensing and resource-budgeted probe planning.
- Symthaea #12407 owns observation event, time, mask and modality semantics.

Next code increment should be a small adapter contract and replay test, not a new clinical agent, scheduler, probability model or duplicated EVSI implementation.
