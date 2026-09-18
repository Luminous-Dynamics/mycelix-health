# mycelix-medication-administration

Evidence-bound clinician medication administration for Mycelix-Health.

The crate intentionally separates **dispensed** from **administered**. It composes exact activation provenance, finalized dispense supply, patient-subject binding, typed administration semantics, and `AuthorityPurpose::Administer` into a single-owner in-process capability that can be consumed into an audit receipt.

It does not upgrade legacy adherence booleans, self-report, caregiver report, or device observations into clinician-performed administration.

See `docs/security/MEDICATION_ADMINISTRATION_V1.md` for the security and qualification contract.
