# FHIR Claim Adapter (Prototype)

A separate, offline Rust prototype mapping a narrow, explicitly declared subset of synthetic FHIR R4 `Claim` resources to `claims-integrity-core`. It is **not** a production FHIR server integration.

## Status and safety boundary

- Synthetic fixture only; no network calls, EHR connection, real claim data, patient identifiers, or production data path.
- `Claim` is mapped only under the declared synthetic supported-subset profile. `ClaimResponse` becomes a **reference-only** projection and never a submitted service line.
- Output signals remain non-authoritative review leads. They cannot trigger payment, coverage, care, sanction or provider-ranking decisions.
- Source digest strings are computed from the exact bytes supplied to this function, but are not a source signature, authenticity assertion, or lawful-processing authorization.
- `source_resource_ref`, `claim_ref`, `beneficiary_ref` and `provider_ref` must be caller-supplied opaque local tokens. The mapper does not pseudonymize raw identifiers or prove that a token is de-identified.
- Mycelix canonical artifact snapshots/selectors are not minted here. The JSON Pointer-like item locator is a hint tied to the exact payload digest and must be qualified by the canonical evidence layer before it becomes a canonical selector.

## Supported mapping policy

Mapping policy version: `fhir-r4-claim-supported-subset-v1`, scoped to FHIR R4 / 4.0.1.

The local `profiles/synthetic-claim-review-v1.StructureDefinition.json` defines the synthetic Claim subset; `profiles/synthetic-claimresponse-linkage-v1.StructureDefinition.json` defines the reference-only response profile. The mapper accepts only active Claim resources with `use=claim` (not preauthorization or predetermination), an explicit `meta.versionId`, required type/patient/created/provider/priority/insurance context, exactly one focal insurance coverage, unique positive item sequences, one unambiguous product/service coding per item, a `servicedDate` (not `servicedPeriod`), a net Money value/currency, and an explicit currency-to-minor-unit scale supplied by the caller. The scale mapping and code-set version are included in the policy digest. Decimal values are parsed with `serde_json`'s `arbitrary_precision` feature and converted without floating-point arithmetic; values that exceed configured precision are not rounded.

To prevent accidental semantic collisions, the internal service code is a deterministic SHA-256 token over the coding system, optional coding version, and code. The envelope retains those original coding fields for review. The source record digest is the SHA-256 of the exact original payload bytes; each mapped line references that digest and its item array index/sequence.

Unknown extensions, unsupported profiles/statuses, duplicate sequences, nested details, modifiers, service Periods, extra unsupported item fields and missing/invalid values are explicitly rejected or recorded as unmapped. Any loss makes the claim Partial; no mapped items yields Indeterminate. The claims core is separately told whether the input was Complete, Partial or Unknown. It may emit review leads over observed records for partial input, but its overall result remains Indeterminate.

A ClaimResponse preserves its source version, payload digest, status/outcome, whether a request reference exists, and an optional caller-supplied opaque linked-Claim ref. It does not map adjudication amounts into Claim service lines or copy raw FHIR references into output.

## Run

```sh
cargo fmt --manifest-path prototypes/fhir-claim-adapter/Cargo.toml -- --check
cargo test --manifest-path prototypes/fhir-claim-adapter/Cargo.toml --locked
cargo clippy --manifest-path prototypes/fhir-claim-adapter/Cargo.toml --locked -- -D warnings
```


Static source review finds 16 uniquely named synthetic unit tests. This is a test inventory, not a claim they have executed. The branch must pass its own CI and an external FHIR profile validator before this profile is treated as qualified.

The workflow tests Rust 1.96.0 and 1.99.0. CI results, rather than the presence of a workflow or test source, determine qualification. No claim is made that this prototype is FHIR-complete or ready for clinical use.
