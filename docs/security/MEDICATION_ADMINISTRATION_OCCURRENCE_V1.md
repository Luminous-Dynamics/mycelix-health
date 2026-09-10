# Medication Administration Occurrence Identity v1

Status: **experimental / qualification-required**.

This tranche addresses P0 #67. Human-readable `administration_id` remains useful audit metadata, but it is not treated as proof of a unique clinical dose occurrence.

## Core invariant

The identity of an intended medication-administration occurrence must come from clinical intent, not record naming or recording time.

V1 derives a domain-separated `MedicationAdministrationOccurrence` digest from:

- exact `MedicationRequestArtifact` digest;
- exact patient-subject binding evidence digest;
- exact dosage index;
- one explicit occurrence kind.

Activation and dispensing provenance are deliberately excluded from occurrence identity. Transitioning from emergency to ordinary activation must not manufacture a second occurrence for the same intended dose.

## One-time

For an order dosage whose typed timing is `OneTime`, the exact medication artifact + subject binding + dosage index defines one occurrence.

## Scheduled

V1 never rounds the wall clock to infer a schedule occurrence.

A schedule resolver produces a bounded semantic `AdministrationSchedulePlanSegmentV1` containing exact UTC occurrence windows and stable ordinals. The semantic plan excludes resolver IDs, versions, generation timestamps, and source-evidence references.

Those operational fields live in a separate `AdministrationScheduleResolutionProvenanceV1` digest. Re-running a resolver with different software/version/time but producing the same semantic windows therefore does not change the clinical occurrence identity.

The occurrence digest itself contains the exact ordinal + intended UTC window, not the schedule-plan/provenance digest. This prevents different bounded segmenting strategies from changing an occurrence that has the same exact order/subject/dosage/ordinal/window meaning.

## PRN / as-needed

PRN occurrences require an explicit `PrnAdministrationIntentV1` with a nonzero nonce. The wall clock alone never defines a PRN occurrence.

The occurrence identity uses the exact order/subject/dosage lineage + nonce. Intent creation time and reason evidence remain provenance of the intent artifact and do not independently manufacture another occurrence.

## Occurrence binding

`mycelix-medication-administration-occurrence-binding` is an additive migration layer. It recomputes and checks:

- exact administration event digest;
- exact qualified administration receipt digest;
- exact occurrence digest;
- medication artifact lineage;
- patient-subject binding lineage;
- dosage index;
- schedule-plan + resolution provenance for scheduled occurrences;
- PRN intent evidence for PRN occurrences.

Existing #64/#66 event/receipt schemas therefore do not need to be rewritten in place.

## Non-claims

This layer does **not** prove:

- that the physical administration occurred;
- that the schedule resolver or its source data is institutionally trusted;
- that a local DHT query is globally complete;
- that a serialized occurrence/binding claim is DHT-validated;
- causal effect of the administration on later observations/outcomes.

A later DHT occurrence-binding attestation must admit the binding under deployment-rooted verifier policy before the canonical network read model uses it as a trusted conflict key.

## Required qualification

Before promotion require tests/evidence for:

- administration-ID substitution producing the same occurrence identity;
- different dosage indexes producing different identities;
- schedule resolver provenance changes not changing semantic occurrence identity;
- schedule window/ordinal changes changing occurrence identity;
- DST/timezone source-resolution cases producing explicit canonical UTC windows;
- no wall-clock rounding path;
- PRN nonce uniqueness and replay behavior;
- event/receipt/occurrence lineage substitution failures;
- schedule plan/provenance mismatch failure;
- exact-head workspace build/test/clippy/fmt;
- conductor-level admission tests once the DHT binding tranche exists.
