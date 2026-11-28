# Consent Temporal and Encounter Scope Enforcement v1

**Status:** source implementation on a dedicated draft branch, with a first set of resource-write call sites migrated. This is not a complete migration of every Mycelix Health read/export path or a production compliance assertion.

## Problem

`ConsentScope` already contains:
- `date_range: Option<DateRange>`;
- `encounter_hashes: Option<Vec<ActionHash>>`.

The existing `AuthorizationCheckInput` did not carry clinical event time or encounter identity, and `check_authorization` did not evaluate either field. A date/encounter restriction could therefore exist in stored Consent without being applied by the authorization function.

## Changes in this increment

### Typed request context

The shared `AuthorizationInput` and Consent zome `AuthorizationCheckInput` now carry optional:
- `clinical_event_time: Option<Timestamp>`;
- `encounter_hash: Option<ActionHash>`.

Both are serde-defaulted so older cross-zome payloads deserialize as `None`. The field is explicitly the resource's clinical event time, never ingestion time or access time.

### Fail-closed scope evaluator

`consent_scope_context_result` evaluates the date and encounter dimensions:

- Date boundaries are inclusive: event time must be greater than or equal to start, and less than or equal to a configured end.
- An open-ended range has no upper bound.
- If `date_range` is present and clinical event time is missing, the consent does not match.
- If `encounter_hashes` is present and encounter is missing, the consent does not match.
- A supplied encounter must be an exact member of the configured encounter list. An empty configured list authorizes no encounter.
- Unrestricted scopes do not require these context values.

The coordinator preserves a temporal/encounter denial reason when no other eligible consent authorizes the request.

### Additive shared API and migrated callers

- Existing `require_authorization(...)` remains available and sends `None` for both optional context fields. It can still satisfy unrestricted consents, but it cannot satisfy a restricted date or encounter scope without context.
- New `require_authorization_for_resource(...)` accepts clinical event time and encounter.
- `create_procedure`: `ProcedurePerformed.performed_at` + required encounter.
- `create_lab_result`, `create_encrypted_lab_result`, `update_lab_result`, and `acknowledge_critical_result`: `LabResult.collection_time` + optional encounter.
- `create_imaging_study`: `ImagingStudy.study_date` + optional encounter.
- `record_vital_signs`: `VitalSigns.recorded_at` + optional encounter.
- `get_encounter`: the encounter's `start_time` + exact encounter hash.
- `create_diagnosis`: exact encounter when supplied. Diagnosis currently has no typed clinical-event timestamp, so a date-restricted grant still denies until one is defined; the outer and embedded patient hashes must match before authorization.

This is intentionally a bounded initial migration: nine operation paths now use the resource-scoped API. Existing generic resource/list/export functions still call the legacy API. They will fail closed for date- or encounter-restricted grants until they are converted to evaluate each returned resource against its own event time and encounter. We do not treat a single patient-level authorization as proof that every resource in a list is in scope.

## Tests added

Pure coordinator tests cover:
- missing event-time denial for a date-restricted scope;
- event before start / after end denial;
- inclusive start and end boundaries;
- missing encounter denial;
- wrong encounter denial;
- matching encounter acceptance;
- unrestricted scope acceptance;
- backwards deserialization when older AuthorizationCheckInput payloads omit the two new optional fields.

The tests validate the pure scope predicate and serialization contract. Exact-head Rust, Clippy, format and Holochain integration qualification remains pending until the PR workflow completes.

## Deliberate non-scope

This PR does not yet:
- migrate every active `require_authorization` call site;
- filter patient-wide result lists per resource or prevent export of out-of-scope entries;
- bind purpose, recipient, grantee provider/organization resolution or OAuth client identity;
- enforce current revocation freshness, signed source Provenance or request consume-once semantics;
- convert emergency access into a general bypass; break-glass remains its separate patient-authored capability path;
- make clinical event-time semantics authoritative for resources whose source schema has not defined the event clock.

The next migration should prioritize list/read/export functions and should authorize each resource before returning its payload. A patient-level check alone is not sufficient when a consent grant is scoped to particular dates or encounters.

See [issue #333](https://github.com/Luminous-Dynamics/mycelix-health/issues/333) for the full authorization context and grantee-resolution acceptance criteria.
