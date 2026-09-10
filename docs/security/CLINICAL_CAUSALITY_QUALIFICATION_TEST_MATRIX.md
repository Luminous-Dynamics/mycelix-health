# Clinical Causality Qualification v1 — adversarial matrix

Required executable qualification cases before promotion:

- wrong causal-assessment policy digest -> reject;
- wrong evidence-trust policy digest -> reject;
- wrong assessor-authority policy digest -> reject;
- unbound legacy `AuthorityPermit` cannot call the public qualifier (compile-time API property);
- policy-bound permit for the wrong assessment -> reject;
- policy-bound permit for the wrong purpose -> reject;
- authority policy scope mutation -> different policy digest;
- missing required authority scope -> no policy-bound permit;
- stale assessor authority -> reject;
- stale evidence trust -> reject;
- stale assessment -> reject;
- future-dated assessment/authority/trust beyond policy skew -> reject;
- evidence trust for different observed event/association -> reject;
- positive conclusion without separately admitted temporal-plausibility evidence -> reject;
- `Indeterminate` assessment may still be process-qualified when all policy/evidence/authority requirements are satisfied;
- serialized qualification receipt validates its self-digest but cannot recreate an in-process capability;
- changing any receipt policy/evidence/principal/conclusion field without recomputing trusted lineage -> reject.

Conductor/runtime follow-up:

- DNA/root policy selection cannot be caller-controlled;
- DHT causal attestation references exact qualified receipt;
- correction/revocation lineage is append-only;
- structurally compatible wrong entry type cannot substitute for expected source entry definition.
