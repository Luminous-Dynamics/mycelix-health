# Clinical Causality Attestation v1 — qualification checklist

Promotion requires executable evidence for at least the following:

- empty `clinical_causality.root_authorities` rejects root authorization creation;
- duplicate root keys or invalid authorization-duration config fail closed;
- non-root cannot issue verifier authorization;
- authorization duration above configured or 15-minute hard limit is rejected;
- verifier cannot publish before `valid_from` or at/after `valid_until`;
- verifier cannot substitute a different receipt commitment;
- verifier cannot substitute qualification policy;
- verifier cannot substitute causal conclusion;
- changing only `attestation_id` changes the attestation digest and invalidates exact authorization;
- structurally valid authorization authored by a non-root is rejected when consumed;
- zero opaque receipt commitment is rejected;
- updates/deletes of trust/evidence entries are rejected;
- correction author must be a causal DNA root;
- correction target action and receipt commitment must match;
- `Other` correction requires a nonzero protected rationale commitment;
- correction-link deletion is rejected;
- modified coordinator cannot bypass integrity validation;
- exact source-entry-definition protection required by P0 #72 is exercised before promotion;
- default packaged DNA contains no causal root authorities.

Privacy review must confirm that neither verifier authorization nor public attestation contains raw patient identity, medication identity, observation details, assessor identity, or raw qualified-receipt digest.

CI/build/test success is necessary but not sufficient for clinical or regulatory qualification.
