# Provider Integrity Authority v1

## Purpose

This tranche moves provider ownership and cross-entry attachment checks from coordinator assumptions into DHT validation.

The threat model includes a modified coordinator. If a rule matters to integrity, it must not depend only on the normal coordinator calling sequence.

## Invariants

1. A provider profile update must be authored by the original profile author.
2. Provider NPI and legacy `mycelix_identity_hash` cannot be rebound through an update.
3. `created_at` is immutable and `updated_at` is monotonic.
4. Legacy `License` and `BoardCertification` entries may only be created by the referenced provider-record author.
5. Legacy license/certification records are immutable. A changed claim is a new record/evidence event rather than mutation of history.
6. Provider-patient relationship v1 creation requires the author to be the author of either the referenced provider record or referenced patient record.
7. Relationship v1 records are immutable.
8. Provider-to-license and provider-to-certification links must agree with the target entry's `provider_hash`, and provider + attachment + link must share the provider owner author.
9. Provider-to-patient links may only be created by one of the referenced record authors.
10. Provider update links must connect provider records authored by the same provider owner.
11. Discovery/index links may only publish a provider record by that provider record's author.
12. Reserved provider-to-records, provider-to-prescriptions, and provider-to-trials links fail closed until explicit authority semantics exist.
13. Entry deletes and link deletes are original-author-only.

## Non-claims

### Provider ownership is not professional licensure

A provider author can still submit a legacy `License` record about themselves. The integrity zome proves who authored/attached the record; it does not prove the external issuing authority, current registry status, jurisdictional scope, or legal authority.

High-assurance flows must continue through:

`issuer-trust -> resolved credential evidence -> clinical-authority -> exact workflow capability`

### NPI format is not NPI verification

The provider entry only checks the legacy 10-digit shape. NPI existence, ownership, enumeration type, deactivation, and current registry state require authoritative external evidence.

### Relationship v1 is not bilateral consent

The existing relationship schema contains provider/patient references but no consent-evidence digest or consent artifact reference. Integrity therefore cannot reconstruct the coordinator's `require_authorization(...)` decision from the relationship entry alone.

A future relationship v2 should bind:

- exact provider principal/binding;
- exact patient principal/binding;
- relationship purpose and scope;
- consent artifact/evidence digest;
- consent policy/version;
- effective period;
- revocation/termination lineage.

Until then, `ProviderPatientRelationship` must not be used as standalone proof of bilateral patient consent.

## Migration

Existing provider, license, certification, and relationship entries remain readable. This tranche hardens new validation/update/link behavior and does not reinterpret legacy records as verified clinical authority.

## Follow-on

1. Build provider-principal adapters only from hardened provider lineage or stronger evidence.
2. Add consent-evidence-bearing provider-patient relationship v2.
3. Deprecate authority-significant use of `verify_provider_credentials`; its legacy status flags inspect provider-submitted records and do not replace issuer-trust/clinical-authority evaluation.
4. Add external NPI/licensure registry adapters with provenance, freshness, and revocation evidence.
5. Route medication requester resolution through `ResolvedPractitionerPrincipal` and then `AuthorityPermit`.
