# Medication Dispense Equivocation Alignment

Status: P0 follow-up required before qualification

The dispense-state reducer and the medication-dispense integrity zome must use the same semantic definition of allocator equivocation.

## Required definition

For one exact medication artifact + activation semantic receipt + refill slot, two allocator authorizations are contradictory only when they authorize different release outcomes:

- different final `MedicationDispenseReceipt` digest; or
- different pharmacist/grantee.

Differences limited to authorization identifier or validity window may represent retry/renewal of the same release authority and must not by themselves become equivocation evidence.

## Current safety posture

The read-side reducer already rechecks this semantic and rejects an `AllocatorEquivocationEvidence` record that points only to retry-equivalent authorizations. The DHT validator still needs to be narrowed from generic payload inequality to the same release-outcome rule.

Until that change is qualified, any application consuming equivocation evidence should pass it through `mycelix-medication-dispense-state` rather than treating the raw evidence entry as a final semantic conclusion.
