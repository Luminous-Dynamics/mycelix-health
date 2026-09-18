# Mycelix Medication Administration State

Conflict-preserving read model for qualified clinician medication-administration attestations and append-only corrections.

This crate deliberately exposes `None`, `One`, or `Conflict` per v1 logical administration occurrence. It never derives a bare `performed: bool`, never chooses a winner by timestamp, and never upgrades legacy `MedicationAdherence` records into qualified administration.

`DuplicateDocumentation` suppresses only the targeted publication action. Other v1 correction reasons invalidate the semantic administration-receipt lineage. All correction provenance remains visible.

The caller must provide a complete-for-purpose set of already DHT-valid records. The reducer does not prove global query completeness. V1 uses `administration_id` as a logical occurrence key; P0 #67 tracks replacement with a computable clinical occurrence identity before scheduled-dose uniqueness may be claimed.
