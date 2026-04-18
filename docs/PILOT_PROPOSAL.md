# Mycelix Health: Hospital Pilot Proposal

## One-Paragraph Summary

Mycelix Health is a patient-controlled health record system where patients own their encryption keys, earn dividends from research use of their data, and providers access records only with granular, revocable consent. Built on Holochain (peer-to-peer, no central server), it implements HIPAA audit controls, 42 CFR Part 2 substance abuse protections, and GDPR crypto-erasure. The system includes federated learning that enables privacy-preserving research without exposing individual records.

## What We're Asking For

A **6-month pilot with 50-100 patients** in one clinical setting (primary care, behavioral health, or clinical trials). We need:

1. **FHIR API access** to your Epic/Cerner endpoint (read-only initially)
2. **50-100 volunteer patients** who want to control their own health data
3. **1-2 clinical champions** (physicians who believe in patient data sovereignty)
4. **IT liaison** for FHIR OAuth setup and firewall configuration
5. **IRB review** for patient consent to the pilot study

## What Patients Get

- **Their own health vault** — records encrypted with keys only they hold
- **Plain-language consent** — "Dr. Chen can see your lab results for treatment until December 2026"
- **Access timeline** — see exactly who viewed what, when
- **Data dividends** — earn from research use of their de-identified data
- **Portability** — export their complete record in FHIR R4 format

## What The Hospital Gets

- **Research pipeline** — federated learning on patient data without data warehousing
- **Compliance documentation** — SHA-256 chained audit trail exceeds HIPAA minimum
- **Patient engagement** — patients who control their data engage more with their care
- **Innovation narrative** — first hospital to deploy patient-sovereign health records

## Technical Requirements

| Requirement | Detail |
|-------------|--------|
| FHIR endpoint | R4, SMART on FHIR OAuth (Epic Open or Cerner Code Console) |
| Network | WebSocket connectivity on port 8888 (Holochain conductor) |
| Patient devices | Any modern browser (Chrome, Firefox, Safari, Edge) |
| Mobile | Android/iOS via Capacitor wrapper (optional) |
| Data residency | All data on patient devices + Holochain DHT (no cloud dependency) |

## Security Guarantees

- **Encryption**: XChaCha20-Poly1305 (256-bit AEAD) with HMAC-HKDF key derivation
- **Audit**: SHA-256 chained, tamper-evident, content-hashed
- **Consent**: 15 data categories, 7 grantee types, re-disclosure prevention
- **Emergency**: Break-glass access with automatic patient notification
- **Erasure**: Patient can crypto-erase all data by destroying their key

## Regulatory Alignment

- HIPAA: encryption at rest, audit controls, right to amend, minimum necessary
- 42 CFR Part 2: segregated substance abuse access, re-disclosure prevention
- GDPR: cryptographic erasure, data portability, consent management
- 21st Century Cures Act: FHIR R4 interoperability, no information blocking

## Timeline

| Month | Milestone |
|-------|-----------|
| 1 | FHIR OAuth setup, IRB submission, patient recruitment |
| 2 | First patient onboarded, records flowing from EHR to vault |
| 3 | Consent management active, providers accessing via consent |
| 4 | Federated learning round with 10+ patients |
| 5 | Data dividend distribution, patient satisfaction survey |
| 6 | Pilot report, decision on expansion |

## Team

Luminous Dynamics — Richardson, TX
- Tristan Stoltz — Founder, systems architect
- Contact: tristan.stoltz@evolvingresonantcocreationism.com

## Open Source

All code is AGPL-3.0. Hospitals can audit every line. No vendor lock-in.
Repository: github.com/luminous-dynamics (public repos for standalone crates)
