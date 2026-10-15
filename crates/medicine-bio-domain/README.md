# medicine-bio-domain

A small Rust domain contract for the first software slice of MED-BIO-001:
evidence checklists for community-grown botanical lots and their controlled
handoff into qualified supply and regulated manufacturing systems.

## Scope

This crate models three distinct checklists:

1. **Traceable botanical supply** — species identity review, lawful propagation,
   biodiversity protection, knowledge rights/benefit sharing, site hazards,
   harvest provenance, storage and handling.
2. **Authorized manufacturing intake** — the botanical checklist plus a
   qualified-buyer authorization assessment and incoming-quality requirements.
3. **Finished-product release review** — intake requirements plus manufacturing
   authorization, validated process, finished-product quality tests, product
   authorization, stability/packaging, qualified release decision, and
   adverse-event/recall readiness.

A garden lot is not a medicine. Passing an earlier checklist never implies that
a later stage passes.

## Safety and trust boundaries

- Missing, unassessed, failed, or malformed required evidence blocks the target.
- Only the knowledge-rights/benefit-sharing gate may be marked not applicable;
  even that requires scoped evidence and a non-empty rationale.
- Every evidence scope must match the exact gate, jurisdiction, relevant batch,
  and (for manufacturing/product gates) stable product-master id. The linked
  finished-product batch id and product id are separate identifiers, neither
  interchangeable with the raw botanical lot.
- Evidence references are shape-checked; hashes are **not** recomputed and do
  not authenticate bytes, reviewer identity, legal status, or regulatory data.
- A satisfied checklist is not a clinical opinion, species identification,
  permit verification, manufacturing permission, efficacy claim, or product
  release authorization.
- No real medicinal species catalogue, therapeutic use, doses, extraction
  methods, patient records, or live price/supply data are included.

The enum list is intentionally small and versioned by this crate's API. Changes
to gates should be reviewed as safety-contract changes, not casual UI edits.

## Run

From the workspace root:

```sh
cargo test -p medicine-bio-domain --target "$(rustc -vV | awk '/^host:/{print $2}')"
cargo run -p medicine-bio-domain --example synthetic_gate_walkthrough --target \
  "$(rustc -vV | awk '/^host:/{print $2}')"
```

The example uses fake evidence identifiers and repeated dummy digests. It is a
mechanical walkthrough, not a successful real-world validation.

## Next implementation boundaries

This crate is intentionally dependency-light and offline-testable. It does not
perform authentication, cryptography, taxonomy lookup, risk scoring, authorization
discovery, lab integration, or Holochain writes. Before integration, add
independently tested evidence resolution and signer/capability verification,
jurisdiction profiles with versioned criteria, append-only corrections and
revocation semantics, duplicate/replay handling, and an independent verifier.
Never let an LLM produce its own approval evidence.
