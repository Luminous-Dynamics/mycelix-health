# Medication Administration State v1

Status: experimental / draft qualification

## Purpose

This reducer derives a deterministic current interpretation from already DHT-valid `QualifiedMedicationAdministration` publications plus append-only `MedicationAdministrationCorrection` records.

It exists so downstream care, CDS, research, and Symthaea cannot collapse administration provenance into `performed = true`.

## State model

For each v1 logical occurrence key (`administration_id`), current state is exactly one of:

- `None`
- `One { administration_receipt_digest }`
- `Conflict { administration_receipt_digests }`

There is no timestamp winner and no arrival-order winner.

Each semantic receipt lineage is separately classified as:

- `Current`
- `Corrected`
- `PublicationSuppressed`

## Duplicate publication vs semantic correction

Multiple DHT actions carrying the exact same semantic administration receipt are idempotent publication provenance, not multiple clinical administrations.

`DuplicateDocumentation` is publication-scoped: it suppresses only the exact publication action it targets. If an equivalent unsuppressed publication remains, the semantic administration receipt remains current.

All other v1 correction reasons are semantic-receipt-scoped. A wrong-patient, wrong-medication, wrong-dose, wrong-route, entered-in-error, or source-evidence-revoked correction invalidates that semantic administration lineage.

All correction evidence remains visible even when one correction determines lifecycle.

## Conflict resolution

Two distinct current semantic administration receipts for the same logical occurrence produce `Conflict`.

Conflict may become `One` only when explicit correction evidence invalidates/suppresses enough competing lineages. Time, action order, agent trust score, or query order cannot choose a winner.

## Read-set closure

The reducer does not query Holochain and cannot prove global absence. A correction whose target publication is absent from the supplied set fails with `OrphanCorrection`; a mismatched semantic receipt fails closed.

Callers must materialize a complete-for-purpose publication/correction set before interpreting the result.

## Identity limitation — P0 #67

V1 groups by `administration_id`, which is root/verifier-authorized but still a caller-selected logical identifier. It does not prove that two different IDs cannot refer to the same intended scheduled dose.

P0 #67 must replace this grouping boundary with a versioned, domain-separated computable administration-occurrence identity derived from exact clinical intent before the system claims unique scheduled-dose occurrence semantics.

Until then this reducer is suitable for exact receipt/correction interpretation, not proof of global scheduled-dose uniqueness.

## Privacy

The reducer uses the privacy-minimized DHT attestation and does not require patient identity, medication name, dose, route, site, diagnosis, or narrative rationale.

Those details remain in protected event/receipt evidence and may be joined only through an authorized clinical workflow.

## Non-claims

This reducer does not prove:

- physical administration occurred;
- global query completeness;
- `administration_id` uniquely identifies a real-world dose occurrence;
- an omitted correction does not exist;
- later symptoms/labs were caused by the administration;
- legacy `MedicationAdherence` is qualified administration;
- regulatory or clinical qualification.

## Qualification targets

Before promotion:

- exact-head format/clippy/build/test must pass;
- duplicate publication must be idempotent;
- same-receipt changed attestation must fail;
- conflicting current receipts must remain conflict independent of input order;
- explicit semantic correction must resolve only the corrected lineage;
- duplicate-documentation correction must suppress only its target publication;
- mixed duplicate + semantic corrections must preserve both provenance classes;
- correction ID/action reuse attacks must fail closed;
- orphan and wrong-receipt corrections must fail closed;
- emergency activation receipt domain must survive reduction;
- conductor tests must prove only DHT-valid publication/correction records reach the reducer adapter;
- P0 #67 must be satisfied before unique scheduled-dose occurrence claims.
