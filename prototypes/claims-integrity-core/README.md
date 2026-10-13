# Claims Integrity Core (Prototype)

A small, dependency-free Rust prototype for deterministic, offline, non-authoritative claims-integrity signals.

## Safety boundary

This package is an engineering prototype. It is not a fraud detector, clinical decision system, payer enforcement engine, or production-ready claims validator. A signal means only that a record pattern merits review. It does not establish fraud, abuse, intent, overpayment, or medical necessity.

The prototype intentionally makes no automated payment, coverage, care, sanctions, or provider-ranking decisions. Do not connect it to real claims or patient data. Use only synthetic fixtures until a separately reviewed lawful data path, privacy controls, threat model, governance process, and evaluation protocol exist.

## Implemented rules

- PotentialDuplicateServiceLines: groups submitted lines that have the same beneficiary reference, provider reference, service code, service date, and amount. This is a review lead, not a finding. Legitimate repeated services can match these fields.
- ReversalTargetNotFound: flags a reversal record whose referenced line is absent from the supplied evidence bundle.
- ReversalTargetNotSubmitted: flags a reversal whose target exists but is itself not a submitted line.
- ReversalTargetMismatch: flags a reversal that references a submitted line but disagrees on beneficiary, provider, service code, service date, currency, or full amount.
- A referenced submitted line is excluded from duplicate grouping only when those service fields and the full amount match the reversal. An inconsistent reversal cannot hide a potential duplicate; ordinary matching reversal-plus-replacement flows still avoid an automatic duplicate signal.
- Incomplete input still permits observed-record analysis, but the overall result is Indeterminate and scope is ObservedRecordsOnly; absence of a signal must not be interpreted as a clean result.

The core deliberately avoids opaque risk scores and provider reputation scores.

## Evidence contract

ClaimEvidenceBundleV1 requires:
- schema version;
- a lowercase 64-hex source-manifest digest;
- a lowercase 64-hex policy digest;
- a declared code-set version and completeness state;
- unique, bounded synthetic/opaque identifiers and a currency-code field whose alpha-3 format is checked (not membership in an authoritative currency registry);
- valid ISO calendar service dates and lowercase 64-hex source-record digests.

The core checks digest syntax, but does not authenticate signatures or recompute/verify that a digest matches source bytes. A future integration must perform source admission and digest verification in a separately trusted layer. Identifiers are schema-checked, not proven de-identified; do not send PHI.

AnomalySignalV1 carries input-manifest and policy digests, exact line references, source-record digests, a stable key, a reason code, and an unconditionally non-authoritative authority marker.

## Run

    cargo fmt --manifest-path prototypes/claims-integrity-core/Cargo.toml -- --check
    cargo test --manifest-path prototypes/claims-integrity-core/Cargo.toml --locked
    cargo clippy --manifest-path prototypes/claims-integrity-core/Cargo.toml -- -D warnings

The crate has its own Cargo workspace on purpose so it can be tested without compiling the full Holochain healthcare workspace or resolving sibling repository dependencies. The synthetic test suite covers 14 behavior and fail-closed cases.

## Out of scope

No fraud conclusions, payment decisions, real-data ingestion, ML detector, legal adjudication, automated provider rankings, or guarantees of privacy/de-identification. Real-world use requires independent evaluation against appropriately governed and adjudicated data, fairness analysis, human review, explanations, and a correction/appeal pathway.
