# Clinical Evidence Authority Semantics Benchmark v1

**Status:** public synthetic development corpus and deterministic evaluator contract. This is not a live authorization engine, official FHIR validator, legal interpretation, clinical validation, or production safety certification.

## What this measures

The benchmark separates six propositions that are often collapsed into one “valid” or “verified” flag:

1. **FHIR conformance:** is the resource structurally valid under the stated FHIR version?
2. **Source authenticity:** is the issuer registered, is the directive signature valid, and does the payload digest match?
3. **Identity and tenancy:** does the request resolve to the same patient and tenant as the governing directive?
4. **Directive state:** is the directive active, effective, unrevoked and not superseded at the decision point?
5. **Scope:** does the request match the clinical event-time range, encounter, category, action, purpose, recipient and grantee?
6. **Execution integrity:** are replay protection, the verifier and the append-only audit path available at the moment of action?

These are separate gates. FHIR validity does not imply source authenticity or authorization. A positive clinical prediction or expected value of information does not grant access. Missing context must never widen a restricted grant. A date-restricted grant with no resolvable clinical event time is denied; ingestion time cannot be substituted.

## Files

- `benchmarks/clinical-evidence-integrity/authority-semantics-v1.json`: fully synthetic, public development cases and single-change mutation pairs.
- `benchmarks/clinical-evidence-integrity/authority-candidate-result-v1.schema.json`: machine-readable candidate output shape.
- `scripts/evaluate-clinical-authority-semantics.sh`: deterministic scorer; requires Bash, jq and sha256sum.

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

The evaluator rejects malformed inputs, duplicate candidate IDs, missing cases, extra cases and benchmark-version mismatches. It reports expected denial and expected deferral separately, along with incorrect deferrals on positive cases. The oracle result fixture is only a scorer self-test and is not a measured Mycelix candidate. Run scripts/tests/test-clinical-authority-evaluator.sh to verify the evaluator's passing control, an unauthorized-release negative control, and incomplete-result rejection. It reports exact decision accuracy, unauthorized-allow count/rate, false-denial rate, unexpected deferrals on positive controls, authorized-allow recall, correct deny/defer counts, restricted-reason coverage, restrictive metamorphic-pair failures, and SHA-256 digests of the exact manifest and candidate result file.

A single unauthorized allow fails the authority gate, regardless of aggregate accuracy. Candidates have three outcomes: allow, deny, and defer. Use deny for an established policy violation; use defer when the system cannot safely establish authority because a verifier, fresh revocation state, replay store, or audit path is unavailable. A date-constrained request with missing clinical event time is an established scope failure and is denied, not deferred. Deferral must never be converted to allow, and it is reported separately from false denials. Metamorphic pairs separately require the positive baseline to allow and the one-dimension-restricted variant to reach its declared restricted outcome (deny for policy failure, defer for temporary unavailability). False denials remain visible as a utility/availability failure; they are not silently traded against unauthorized releases.

## Paradigm shifts to test

### 1. Proof-carrying authorization, not a trusted boolean

An allow decision should be accompanied by the exact, versioned source, consent directive, signature/key evidence, patient/tenant binding, grantee resolution, resource version, clinical event time, purpose, recipient, action, revocation snapshot and policy version used to decide. The verifier should be independently replayable. A decision receipt is evidence of a bounded decision—not permission for future requests.

### 2. Time-indexed authority

Evaluate date restrictions against clinical event time, not ingestion time or access time. Keep clinical time, source update time, directive effective time, audit recorded time, ingestion time and execution time distinct. Re-check revocation immediately before release. An old “allow” receipt must not survive revocation merely because it was valid when first computed.

### 3. Scope as a monotone constraint lattice

Treat patient, tenant, recipient, purpose, action, category, encounter and date restrictions as intersecting constraints. Removing context or tightening any constraint must never expand access. Test this with paired counterfactuals and property-based mutation tests—not only happy-path examples.

### 4. Separate evidence generation, access and authority

Keep FHIR Provenance (generation/derivation), AuditEvent (usage/security activity) and Consent (policy representation) as separate semantic families. They can be correlated to a request and resource version without being conflated. A valid AuditEvent does not prove a source assertion, and a Provenance agent label does not prove identity.

### 5. Fail-closed execution with explicit unavailability

Verifier outage, stale revocation state, replay-store outage or audit-store failure must not become an allow. A system should return a typed denial/defer result and preserve enough evidence to reconcile unknown outcomes without repeating a disclosure.

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

This suite complements rather than replaces clinical-agent and EHR-workflow benchmarks. HealthAgentBench v2 evaluates realistic tasks across nine environments and reports a 57.3% best success rate on 75 tasks. HealthFlow reports EHRFlowBench 4.01 versus 3.76 for its strongest baseline and MedAgentBoard 63.05% versus 50.10%. Those scores concern different task distributions and cannot be compared numerically to this authority suite.

For a fair system comparison:
1. run all candidates against the same frozen public corpus and separately held-out corpus;
2. provide identical FHIR resources, registry snapshots, tool permissions, latency budgets and execution APIs;
3. compare a conventional SMART/OAuth-backed policy service, a deterministic rules baseline, a general-purpose agent, and the Mycelix + Symthaea path;
4. test each arm with the same positive and negative controls, then add the Symthaea reasoning/active-sensing layer only behind Mycelix's independent authority gate;
5. report safety, valid-allow recall, decision latency, cost per decision, audit completeness and failure recovery separately.

Do not claim superiority from a higher pooled score if one system makes any unauthorized disclosures. Never compare this score to HealthAgentBench or EHRFlowBench as if they were the same benchmark.

## Current scope and next engineering steps

This increment supplies the corpus, candidate-output contract and scoring executable. It does not modify the Consent zome or prove that live Mycelix decisions satisfy the cases. The next production work remains:
- extend authorization input with clinical event time, encounter, resource version, action, purpose and recipient;
- resolve Provider/Organization grantees through authenticated registries;
- bind current revocation and consume-once request semantics to the pre-execution check;
- hash and persist the full audit event with authenticated actor identity;
- add multi-agent Holochain and gateway integration tests, concurrency/revocation race tests and independent security review.

All cases are synthetic and contain no PHI.
