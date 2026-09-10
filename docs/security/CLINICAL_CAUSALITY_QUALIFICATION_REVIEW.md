# Clinical Causality Qualification v1 — review boundary

This tranche is intentionally limited to pure Rust policy/evidence composition. It adds no DHT attestation and does not change legacy trial storage.

Review should focus on:

- exact authority-policy binding;
- exact subordinate-policy binding in qualification;
- evidence-use-scoped trust admission;
- freshness and future-skew handling;
- positive-causality temporal-plausibility requirement;
- non-cloneable/non-serializable capability surfaces;
- audit receipt self-digest integrity;
- preservation of `Indeterminate` as a valid qualified process outcome.

Promotion remains blocked on executable CI and the runtime attestation work tracked separately.
