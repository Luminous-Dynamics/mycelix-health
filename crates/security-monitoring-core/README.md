# Mycelix Security Monitoring Core

Reference semantics for one narrow security-monitoring theorem:

> A clean monitoring assessment is a positive, evidence-bearing result. It is not the absence of an error and it is not an empty anomaly vector produced after telemetry failure.

## State model

The public result distinguishes:

- `Clean(MonitoringEvidenceReceipt)` — complete evidence was retrieved and evaluated and no rule fired;
- `Anomalies { .. }` — complete evidence was retrieved and one or more rules fired;
- `Unavailable(..)` — the detector could not obtain the evidence required to decide;
- `InvalidEvidence(..)` — evidence was returned but could not be trusted/evaluated.

Only the first two states carry a positive `MonitoringEvidenceReceipt`.

## Evidence receipt

A successful assessment binds:

- opaque subject context;
- observation window;
- source-record count;
- canonical source-set digest;
- exact rule-set identifier;
- evaluation time;
- contract version.

A legitimate empty source set can still produce `Clean`, but only when the empty set was actually retrieved and represented by a non-zero canonical digest.

## Deliberate boundaries

This crate does **not**:

- fetch Holochain access logs;
- define the production anomaly algorithms;
- persist security events;
- claim the source audit trail is complete or tamper-evident;
- expose clear PHI in monitoring identifiers;
- make HIPAA, 42 CFR Part 2, GDPR, POPIA, breach-detection or regulatory-compliance claims.

Those remain separate proof lines, especially #164, #176 and #178.

## Integration direction

The production `records::detect_access_anomalies` adapter should eventually convert transport/decode/authentication outcomes into this typed state rather than collapsing failures into `Vec::new()`.

SDK and Leptos consumers must preserve the distinction. A UI must never render `Unavailable` or `InvalidEvidence` as `No anomalies`.

Tracks #182.
