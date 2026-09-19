# Medication Administration — Next Boundary

This note records the next lifecycle boundary after qualified dispensing.

Dispensing proves that an authorized pharmacy released a medication. It does **not** prove that the medication was administered to or taken by the patient.

A future administration path should therefore be a separate evidence-bearing transition, not a status mutation on the dispense record.

Minimum v1 inputs should include:

- exact medication artifact identity;
- exact current activation lineage;
- exact finalized dispense provenance when applicable;
- exact patient identity/subject binding;
- administrator identity and professional authority when administration is clinician-performed;
- patient/self-administration provenance when applicable;
- typed dose, route, site, timing, method, and product identity;
- deviation/omission/refusal reason semantics;
- device/lot evidence when clinically required;
- exact administration policy and freshness constraints;
- resulting observation/adverse-event linkage.

The model should distinguish at least `Performed`, `NotDone`, `PartiallyPerformed`, and `Indeterminate` rather than converting missing administration evidence into `Performed`.

Emergency provenance must remain visible from activation through dispensing into administration.
