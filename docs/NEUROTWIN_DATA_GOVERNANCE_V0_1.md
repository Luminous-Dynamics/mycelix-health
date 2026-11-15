# NeuroTwin Data Governance Contract v0.1

**Status:** research architecture specification. Not a compliance certification or permission to process real patient data.
**System role:** Mycelix Health governs patient-controlled consent, data access, provenance and accountability. It does not perform biological inference or authorize clinical treatment.

## 1. Current-state boundary

The active Mycelix Health repository contains patient, provider, records, consent and behavioral-health functionality. The mental-health zome documents screening instruments, mood entries, safety plans, crisis-event records and encrypted therapy-note workflows.

The digital-twin zome appears in the repository's historical architecture and archived material, but there is no active twin implementation in the current workspace. The current roadmap treats digital-twin restoration as deferred research work. Do not describe the health twin as deployed or operational.

The parent Mycelix project identifies the health cluster as pre-alpha / research-stage and notes that integration tests, external threat modelling, audits and clinical workflow validation remain incomplete. Repository status and unit tests are not independent clinical, security or regulatory assurance.

## 2. Data architecture

- Keep raw MRI, fMRI, diffusion MRI, EEG/MEG, DICOM/BIDS files, direct identifiers, full FHIR payloads and sensitive clinical notes in controlled encrypted storage operated under an approved data-governance arrangement.
- Do not place identifiable health data or raw neuroimaging in a public or broadly replicated DHT.
- Use Mycelix entries for consent state, permitted purposes, dataset/resource references, provenance digests, access decisions, attestations and audit events. A digest proves identity of bytes, not clinical truth.
- Do not assume embeddings, derived features, model gradients or federated updates are anonymous. Treat them as potentially re-identifiable sensitive data until a threat model and empirical privacy assessment establish otherwise.
- Separate care records from research copies and training pipelines. Use purpose-limited identifiers and keep re-identification keys outside analytical environments.
- Apply least privilege, encryption in transit and at rest, key rotation, compartmentalized access and independent security review. Emergency access must be narrowly scoped, time-limited, justified and audited.

## 3. Granular consent

Consent must be specific, understandable, time-bounded where appropriate and independently revocable for each purpose:

1. Direct care and care-team access.
2. Secondary research use of existing records.
3. Neuroimaging / electrophysiology analysis.
4. Model development and model evaluation.
5. Federated learning participation.
6. Sharing with named institutions or collaborators.
7. Commercial research / licensing.
8. Recontact and future-study invitations.
9. Cross-border transfer and hosting.

A consent to receive care is not consent to research, model training, commercial use or data dividends. Declining secondary research must not reduce care access or provider priority.

Revocation should stop future access and future processing covered by the revocation, propagate to downstream consumers, and create an auditable receipt. Explain honestly that revocation cannot necessarily retract analyses already lawfully completed, published aggregate results or model parameters already trained; the data controller must define whether deletion, cryptographic erasure, retraining or unlearning is technically and legally possible for each use.

## 4. Evidence-carrying data-use receipt

Every research job should emit a signed/attested receipt with at least:

- Study and protocol identifier/version.
- Ethics / IRB determination and validity period.
- Dataset identifier, immutable release/version and source digest manifest.
- Data-use agreement / licence identifier and permitted purposes.
- Participant inclusion manifest commitment (never publish a re-identifiable subject list).
- Consent policy version and purpose authorization decision.
- Data transformation, preprocessing pipeline and code version.
- Model name/version, parameter digest and training configuration.
- Compute environment and random seed.
- Output artifact digest, metrics and report version.
- Operators, institution, timestamps and independent reviewer status.
- Data retention / deletion deadline and incident contact.

A receipt records what computation was performed under which authority. It does not assert that a model is accurate, a clinical claim is true, or a trial is ethically valid.

## 5. Research federation and privacy

Federated learning is not a substitute for governance. Before multi-institution training:

- Establish a data-controller / processor allocation and institutional agreements.
- Threat-model gradient leakage, membership inference, model inversion, poisoning, collusion and compromised clients.
- Specify secure aggregation, minimum cohort sizes, clipping, differential privacy accounting and attack-response thresholds.
- Evaluate privacy–utility tradeoffs on the actual target task.
- Prevent one institution or provider reputation score from silently controlling patient eligibility, triage or research access.
- Preserve per-site provenance and report site-level performance; a federated aggregate score can hide a failing site.
- Keep model release and export separately authorized; model updates can carry sensitive information.

## 6. Sensitive mental-health workflows

Treat psychiatric, trauma, intimate-partner violence, substance-use, suicide-related and therapy-note data as high sensitivity. Use segmented consent and narrowly scoped access.

The system must not:
- Autonomously diagnose or prescribe.
- Start, stop or change medication.
- Infer imminent suicide risk from a single score or opaque model.
- Replace clinical crisis assessment or local emergency services.
- Rank patients' worthiness for care using provider, patient or model reputation.
- Use data-dividend incentives to pressure participants into wider consent.

Any screening instrument, including PHQ-9, GAD-7 or C-SSRS, is a structured measurement with limitations. A screening score is not a diagnosis. Crisis-related workflows require qualified clinical ownership, defined response time, staffed escalation and jurisdiction-appropriate emergency resources.

## 7. Mycelix responsibilities versus Symthaea / Symtropy

| Component | Responsibility | Explicit non-responsibility |
|---|---|---|
| Mycelix Health | Consent, authorization, data provenance, audit, patient agency, controlled federation | Clinical diagnosis, treatment selection, biological truth |
| Symthaea | Research models, multimodal analysis, uncertainty, reproducible predictions | Self-authorized care or claims of consciousness as clinical evidence |
| Symtropy | Calibrated simulation only when equations, units and parameters are defensible | Treating generic physics or Φ coupling as a validated brain/body simulator |
| Clinical partners | Protocol, interpretation, clinical oversight, care decisions, adverse-event response | Delegating professional responsibility to an algorithm |

## 8. Security and compliance qualification

Before any real patient use, obtain:
- Independent cryptographic and application security review.
- Penetration testing and abuse-case review.
- Privacy impact assessment and jurisdiction-specific legal review.
- Clinical workflow validation and usability assessment.
- Backup, recovery, breach-response and business-continuity exercises.
- Independent review of consent revocation, deletion and data-export claims.
- A documented regulatory intended-use assessment.

Do not advertise “HIPAA compliant”, “GDPR compliant”, “Part 2 compliant”, “FDA cleared” or equivalent based solely on an implementation checklist. Legal compliance depends on the organization, contracts, jurisdiction, operations and independent assessment—not only code.

## 9. Acceptance gates

- No research processing without a verifiable current purpose authorization.
- No access decision from an absent, expired, malformed or ambiguous consent.
- No public-DHT PHI or raw neuroimaging.
- No secondary purpose inferred from a broad care consent.
- Every research output links to a versioned data-use receipt.
- Revocation creates a verifiable downstream invalidation / restriction event.
- Privacy threat model includes embeddings and gradients.
- Clinical and emergency workflows have named, accountable human owners.
- All critical controls have negative tests, not only happy-path tests.

**v0.1 outcome:** Mycelix becomes the authority and evidence ledger around research data use, while clinical and biological claims remain outside its authority.
