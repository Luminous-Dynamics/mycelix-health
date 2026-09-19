# Emergency Medication Override v1

## Purpose

Emergency care must not depend on pretending incomplete safety evidence is `Safe`, and it must not be stopped merely because a CDS/knowledge service cannot complete every normal check in time.

This tranche introduces a **separate emergency uncertainty pathway**. It never returns or aliases ordinary `MedicationActivationCapability`.

The v1 path is intended for:

- `Indeterminate` medication-safety assessments caused by missing/stale/partial/unavailable evidence; and
- optionally, deployment-policy-approved `RequiresReview` assessments when delay itself would create serious clinical risk.

A `Blocked` assessment is not accepted by v1. Known contraindicating evidence requires a future, separately designed break-glass pathway with stronger authority/review requirements if such a workflow is clinically justified.

A fully `Cleared` assessment is also rejected here; it belongs on the ordinary qualified activation path.

## Required inputs

`authorize_emergency_medication_override(...)` requires all of the following:

1. an activation-valid, typed `MedicationRequestArtifact`;
2. fresh requester -> authenticated-principal resolution;
3. `VerifiedMedicationSafetyAssessment` for the exact medication artifact;
4. an exact expected medication-safety policy digest;
5. a professional `AuthorityPermit` whose purpose is `Prescribe` and whose target is the exact medication artifact;
6. an exact expected authority-policy digest;
7. a versioned `EmergencyMedicationOverridePolicyV1`;
8. the authenticated principal and execution jurisdiction;
9. a typed emergency reason;
10. a non-zero commitment to the sensitive human rationale;
11. an execution time against which all freshness limits are rechecked.

Emergency status therefore relaxes **only the normal full-safety-clearance requirement**. It does not relax medication semantics, requester identity, prescribing authority, exact-target binding, policy binding, jurisdiction, or evidence freshness.

## Output

The result is a non-cloneable/non-serializable `EmergencyMedicationOverrideCapability`.

Consuming it produces `EmergencyMedicationOverrideReceiptV1`, binding:

- exact medication artifact digest;
- requester-resolution evidence;
- exact safety-assessment digest;
- safety context and safety policy identities;
- the non-cleared safety decision and typed reasons;
- professional authority policy/evidence;
- emergency override policy;
- jurisdiction;
- typed emergency reason;
- rationale commitment;
- authorization time.

The serialized receipt is evidence, not authority. Deserialization cannot recreate the consumed capability.

## Safety principles

### Unknown is not safe

`Indeterminate` remains `Indeterminate` in the receipt. No emergency path converts it to `Cleared`.

### Emergency is not an identity bypass

The medication requester must still resolve to the authenticated principal, and the requester resolution must still be fresh.

### Emergency is not a prescribing-authority bypass

The clinician must still hold professional authority for `Prescribe`, for the exact medication artifact, exact authority policy, and execution jurisdiction.

### Emergency is not a contraindication bypass

`Blocked` is rejected by v1. A future known-danger break-glass design, if required, must be a separate capability with stronger controls rather than a flag on this one.

### Sensitive rationale stays protected

The portable receipt carries a commitment to the rationale rather than requiring raw PHI/free text. The readable rationale belongs in the appropriate protected clinical record.

## Remaining trust boundary

The pure crate cannot establish which emergency policy a deployment is institutionally allowed to use. A DHT/runtime layer must pin the accepted emergency-policy identity independently of request-controlled input before an emergency override becomes a distributed clinical state.

That DHT path should also remain **separate from `QualifiedMedicationActivation`**, so ordinary-cleared and emergency-override states are unambiguous in every read model and audit export.
