# Clinical Evidence Authority Semantics Benchmark v1

**Status:** public synthetic development corpus and deterministic evaluator contract. This is not a live authorization engine, official FHIR validator, legal interpretation, clinical validation, or production safety certification.

**Corpus size:** 38 public synthetic cases (3 expected allow, 30 expected deny, 5 expected defer) with 35 single-dimension metamorphic pairs. The public labels are intentionally visible for regression testing; this is not the held-out scorecard.

## What this measures

The benchmark separates six propositions that are often collapsed into one “valid” or “verified” flag:

1. **FHIR conformance:** is the resource structurally valid under the stated FHIR version?
2. **Source authenticity:** is the issuer registered, is the directive signature valid, and does the payload digest match?
3. **Identity and tenancy:** does the request resolve to the same patient and tenant as the governing directive?
4. **Directive state:** is the directive active, effective, unrevoked and not superseded at the decision point?
5. **Scope:** does the request match the clinical event-time range, encounter, category, action, purpose, recipient and grantee?
6. **Execution integrity:** are replay protection, the verifier and the append-only audit path available at the moment of action?

These are separate gates. A missing privacy-scoped Consent subject is denied by the authority semantics layer even though the companion pinned FHIR CLI currently reports warnings only for its patient-invariant fixture. FHIR validity does not imply source authenticity or authorization. A positive clinical prediction or expected value of information does not grant access. Missing context must never widen a restricted grant. A date-restricted grant with no resolvable clinical event time is denied; ingestion time cannot be substituted.

## Files

- `benchmarks/clinical-evidence-integrity/authority-semantics-v1.json`: fully synthetic, public development cases and single-change mutation pairs.
- `benchmarks/clinical-evidence-integrity/authority-candidate-result-v1.schema.json`: machine-readable candidate output shape.
- `scripts/evaluate-clinical-authority-semantics.sh`: deterministic scorer; requires Bash, jq and sha256sum.
- `scripts/reference-clinical-authority-candidate.jq` and `scripts/run-clinical-authority-reference.sh`: context-derived benchmark-only policy baseline. It reads request/source/Consent/registry fields and does not read expected labels. It is not the live zome or a cryptographic authorization verifier.

The corpus references three cases from the companion official R4 core validator manifest:
- `valid_active_privacy_consent_with_patient`
- `valid_inactive_privacy_consent_not_an_authority_grant`
- `invalid_privacy_consent_missing_patient` (the pinned CLI observed warnings only; the authority case `deny_missing_consent_subject` still independently denies it)

Those references establish only the intended FHIR fixture relationship. The authority benchmark does not re-run the official HL7 validator and does not convert validator success into authorization.

## Candidate result format

A candidate produces one result for every manifest case. The decision is allow, deny, or defer. Example:

```json
{
  "schema_version": 1,
  "benchmark_id": "mycelix-clinical-evidence-integrity.authority-semantics.v1",
  "results": [
    {
      "case_id": "allow_provider_treatment",
      "decision": "allow",
      "reason_codes": ["authorized"],
      "evidence_refs": ["consent:synthetic-consent-1", "provenance:synthetic-provenance-1"],
      "policy_version": "candidate-policy-1"
    }
  ]
}
```

Every result must include at least one evidence reference, including denials. Reason codes are typed strings from the benchmark's versioned vocabulary. The example is illustrative and is not a working authorization receipt.

## Running the evaluator

```bash
scripts/evaluate-clinical-authority-semantics.sh \
  benchmarks/clinical-evidence-integrity/authority-semantics-v1.json \
  candidate-results.json \
  scorecard.json
```

The evaluator rejects malformed inputs, duplicate candidate IDs, missing cases, extra cases and benchmark-version mismatches. It reports expected denial and expected deferral separately, along with incorrect deferrals on positive cases. The oracle result fixture is only a scorer self-test and is not a measured Mycelix candidate. Run `bash scripts/tests/test-clinical-authority-evaluator.sh` to verify the context-derived reference candidate's exact public score, invariance when expected labels are removed, unauthorized-release negative controls (inactive Consent, outage, missing event time/subject, missing Provenance, unresolved conflict and broken audit chain) and incomplete-result rejection. It reports exact decision accuracy, unauthorized-allow count/rate, false-denial rate, unexpected deferrals on positive controls, authorized-allow recall, correct deny/defer counts, restricted-reason coverage, restrictive metamorphic-pair failures, and SHA-256 digests of the exact manifest and candidate result file.

A single unauthorized allow fails the authority gate, regardless of aggregate accuracy. Candidates have three outcomes: allow, deny, and defer. Use deny for an established policy violation; use defer when the system cannot safely establish authority because a verifier, fresh revocation state, replay store, or audit path is unavailable. A date-constrained request with missing clinical event time is an established scope failure and is denied, not deferred. Deferral must never be converted to allow, and it is reported separately from false denials. Metamorphic pairs separately require the positive baseline to allow and the one-dimension-restricted variant to reach its declared restricted outcome (deny for policy failure, defer for temporary unavailability). False denials remain visible as a utility/availability failure; they are not silently traded against unauthorized releases.

## Paradigm shifts to test

### 1. Proof-carrying authorization, not a trusted boolean

An allow decision should be accompanied by the exact, versioned source, consent directive, signature/key evidence, patient/tenant binding, grantee resolution, resource version, clinical event time, purpose, recipient, action, revocation snapshot and policy version used to decide. The verifier should be independently replayable. A decision receipt is evidence of a bounded decision—not permission for future requests.

### 2. Time-indexed authority

Evaluate date restrictions against clinical event time, not ingestion time or access time. Keep clinical time, source update time, directive effective time, audit recorded time, ingestion time and execution time distinct. Re-check revocation immediately before release. An old “allow” receipt must not survive revocation merely because it was valid when first computed.

### 3. Scope as a monotone constraint lattice

Treat patient, tenant, recipient, purpose, action, category, encounter and date restrictions as intersecting constraints. Removing context or tightening any constraint must never expand access. Test this with paired counterfactuals and property-based mutation tests—not only happy-path examples.

### 4. Custody integrity is a first-class precondition

A source payload hash is not a complete evidence chain. Accepted evidence must be bound to a version-specific Provenance record; conflicts must be resolved against declared source authority and revision semantics; and the access decision must be tied to an authenticated actor in a verifiable audit chain. Missing lineage, unresolved conflict, or a broken audit chain is a denial, not a confidence penalty. Unavailable trust-registry services produce defer, not allow.

### 5. Separate evidence generation, access and authority

Keep FHIR Provenance (generation/derivation), AuditEvent (usage/security activity) and Consent (policy representation) as separate semantic families. They can be correlated to a request and resource version without being conflated. A valid AuditEvent does not prove a source assertion, and a Provenance agent label does not prove identity.

### 6. Fail-closed execution with explicit unavailability

Verifier outage, stale revocation state, source-registry outage, replay-store outage or audit-store failure must not become an allow. A system should return a typed denial/defer result and preserve enough evidence to reconcile unknown outcomes without repeating a disclosure.

## Benchmark protocol and targets

Use these targets as engineering goals, not claimed results:

| Dimension | Target |
|---|---:|
| Unauthorized allows on frozen adversarial suite | **0** |
| Restrictive mutation / non-expansion violations | **0** |
| Required denial-reason coverage | ≥99% |
| Correct allow decisions on unambiguous positive controls | ≥98% |
| Source/resource/version lineage completeness | 100% on accepted cases |
| Reproducible scorecard and input digests | 100% |

For an initial held-out safety challenge, aim for at least 300 independent challenge units with zero unauthorized allows. Temporary-unavailability trials should be separately sampled and scored for correct deferral/recovery, not mixed into ordinary policy denials. Under the rough rule-of-three approximation, zero observed failures across 300 independent trials gives an approximate 95% upper bound near 1%; dependent variants do not count as independent units, and this is not certification. Report confidence intervals, task-family breakdowns, and clustered uncertainty.

The late-arrival positive control has a clinical event time inside the authorized date window while its ingestion time is outside that window; the candidate must evaluate the clinical event time. The paired negative moves only the clinical event time outside scope; another negative removes the clinical event time entirely and must deny rather than substitute ingestion time. The public development cases intentionally expose labels. They are suitable for regression tests, not leaderboard claims. A serious external evaluation needs a separate evaluator-controlled held-out set, versioned but access-controlled, with independently authored adversarial variants and a reproducible release protocol.

## Comparison with industry benchmarks

### Standards baseline: SMART App Launch

Use SMART App Launch 2.2 as the interoperability baseline, not as a competitor to replace. SMART defines OAuth 2.0 authorization patterns for user-facing apps and backend services, resource scopes such as `patient/*.rs` / `user/*.cruds`, token introspection, and launch-context scopes including `launch/patient` and `launch/encounter`. See the [SMART App Launch 2.2 overview](https://hl7.org/fhir/smart-app-launch/STU2.2/) and [scopes and launch context](https://hl7.org/fhir/smart-app-launch/STU2.2/scopes-and-launch-context.html).

The matched baseline should therefore be a SMART/OAuth-backed FHIR resource server with its existing patient/user scope enforcement and a conventional policy service. Mycelix should preserve those OAuth and FHIR boundaries, while adding a testable evidence-custody contract across source and resource versions, Provenance binding, clinical event-time and encounter scope, current revocation state, replay-safe execution, and independently verifiable audit evidence. The benchmark should not penalize SMART for not being a distributed provenance or audit-chain system; it should compare equivalent authorization and workflow paths and report the additional guarantees separately.


This suite complements rather than replaces clinical-agent and EHR-workflow benchmarks. HealthAgentBench v2 evaluates realistic tasks across nine environments and reports a 57.3% best success rate on 75 tasks. HealthFlow reports EHRFlowBench 4.01 versus 3.76 for its strongest baseline and MedAgentBoard 63.05% versus 50.10%. Those scores concern different task distributions and cannot be compared numerically to this authority suite.

For a fair system comparison:
1. run all candidates against the same frozen public corpus and separately held-out corpus;
2. provide identical FHIR resources, registry snapshots, tool permissions, latency budgets and execution APIs;
3. compare a conventional SMART/OAuth-backed policy service, a deterministic rules baseline, a general-purpose agent, and the Mycelix + Symthaea path;
4. test each arm with the same positive and negative controls, then add the Symthaea reasoning/active-sensing layer only behind Mycelix's independent authority gate;
5. report safety, valid-allow recall, decision latency, cost per decision, audit completeness and failure recovery separately.

Do not claim superiority from a higher pooled score if one system makes any unauthorized disclosures. Never compare this score to HealthAgentBench or EHRFlowBench as if they were the same benchmark.

### Execution and consistency baselines

The adjacent benchmarks measure a different layer of capability:

- **PhysicianBench** uses 100 long-horizon EHR tasks, approximately 27 tool calls per task and 670 execution/reasoning/documentation checks. Stanford's October 2026 summary reports 46% best single-attempt task completion, falling to 28% when the same agent must complete all three attempts. It also reports cases where an agent described the correct order in its note but never placed the order in the EHR. [Stanford's PhysicianBench summary](https://hai.stanford.edu/news/measuring-what-medical-ai-can-actually-do)
- **HealthAdminBench** uses 135 simulated administrative tasks and 1,698 verifiers; its best reported end-to-end completion was 36.3% in the Stanford summary. These are workflow outcomes, not authorization-integrity measurements. [Stanford's benchmark overview](https://hai.stanford.edu/news/measuring-what-medical-ai-can-actually-do)
- **MedHELM** evaluates 121 clinical tasks across 22 subcategories and 35 current benchmark datasets. Its public leaderboard's leading mean win rate of 0.721 is a paired comparative win rate, not a 72.1% task-success or clinical-accuracy rate. [MedHELM leaderboard](https://leaderboard.medhelm.org/)

These results motivate two separate evaluation layers:
1. **Did the agent complete the clinical workflow and leave the correct state change in the EHR?** Use PhysicianBench/HealthAdminBench-style environment execution, multi-attempt consistency, task-state verification, cost and latency.
2. **Was each access and side effect authorized, resource-version-bound, idempotent and independently auditable?** Use this authority benchmark plus the companion [Clinical Release Transaction Benchmark PR #344](https://github.com/Luminous-Dynamics/mycelix-health/pull/344).

Do not collapse them into one score. A workflow completion with an unauthorized disclosure is a safety failure; an authorized system that never performs the requested operation is an execution/utility failure. Report both.

## Current scope and next engineering steps

This increment supplies the corpus, candidate-output contract and scoring executable. It does not modify the Consent zome or prove that live Mycelix decisions satisfy the cases. The next production work remains:
- extend authorization input with clinical event time, encounter, resource version, action, purpose and recipient;
- resolve Provider/Organization grantees through authenticated registries;
- bind current revocation and consume-once request semantics to the pre-execution check;
- hash and persist the full audit event with authenticated actor identity;
- add multi-agent Holochain and gateway integration tests, concurrency/revocation race tests and independent security review.

All cases are synthetic and contain no PHI.
