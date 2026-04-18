# Clinical Deployment Checklist

**Adapted from** `mycelix-workspace/RESILIENCE_DEPLOY_CHECKLIST.md`
**Purpose**: Every item must pass before deploying to a clinical environment.
**Rule**: If ANY item fails, DO NOT DEPLOY. Fix it first. Reputation is everything.

## Pre-Deployment (Before Touching Hospital Network)

### 1. Build Verification
- [ ] All 12 WASM zomes compile: `cargo build --release --target wasm32-unknown-unknown`
- [ ] DNA packs successfully: `hc dna pack dna/ -o dna/health.dna`
- [ ] hApp packs successfully: `hc app pack . -o mycelix-health.happ`
- [ ] FL tests pass (35/35): `cargo test --target x86_64-unknown-linux-gnu -p mycelix-health-fl -p mycelix-health-crypto -p mycelix-health-zkp`
- [ ] Quick validation passes: `bash scripts/quick-test.sh`

### 2. Conductor Verification
- [ ] Conductor starts cleanly: `echo "" | hc sandbox --piped generate -a 9999 mycelix-health.happ --run=8888`
- [ ] Admin interface responds: `curl ws://localhost:33743` (or assigned admin port)
- [ ] App interface responds on 8888
- [ ] Portal detects conductor (badge shows "Live")

### 3. Encryption Verification
- [ ] Key generation produces valid BIP-39 phrase (24 words from 2048 wordlist)
- [ ] Encrypt → store → retrieve → decrypt roundtrip succeeds
- [ ] Wrong key fails decryption (Poly1305 auth tag catches it)
- [ ] Key rotation creates new EncryptedRecords without breaking old ones

### 4. FHIR Connectivity
- [ ] Epic CapabilityStatement returns 200: `bash services/ehr-gateway/test-epic-sandbox.sh`
- [ ] All 7 resource types confirmed (Patient, Observation, Condition, MedicationRequest, AllergyIntolerance, Immunization, Procedure)
- [ ] OAuth token exchange works with hospital's FHIR endpoint
- [ ] Patient record ingest succeeds (at least 1 test patient)
- [ ] Data maps correctly to Mycelix entry types

### 5. Consent System
- [ ] Create consent → verify authorized access
- [ ] Revoke consent → verify access blocked
- [ ] Emergency access creates notification + audit entry
- [ ] 42 CFR Part 2 substance abuse records require specific consent
- [ ] Re-disclosure prevention blocks unauthorized sharing
- [ ] Consent rendering produces readable plain language

### 6. Portal Verification
- [ ] Portal loads in < 3 seconds on hospital WiFi
- [ ] Onboarding flow completes without errors
- [ ] All 7 pages render correctly
- [ ] Consent wizard creates valid consent entries
- [ ] Privacy budget gauge updates on FL contribution
- [ ] Works on Chrome, Firefox, Safari, Edge (latest versions)
- [ ] Works on mobile (responsive layout)

## Hospital-Specific (At The Clinical Site)

### 7. Network Configuration
- [ ] Hospital firewall allows WebSocket on port 8888
- [ ] FHIR OAuth redirect URL registered with Epic/Cerner
- [ ] SSL/TLS certificate installed for portal domain
- [ ] DNS configured for portal URL

### 8. Staff Training
- [ ] Clinical champion understands the consent model
- [ ] IT liaison can restart conductor if needed
- [ ] Help desk has FAQ for patient questions
- [ ] Break-glass procedure documented and understood

### 9. Patient Onboarding
- [ ] IRB approval obtained and documented
- [ ] Patient consent forms signed (for the pilot study, not data consent)
- [ ] First patient completes onboarding without help
- [ ] Second patient completes onboarding without help
- [ ] Tenth patient completes onboarding in < 5 minutes

### 10. Go-Live Monitoring
- [ ] First 24 hours: check conductor logs hourly
- [ ] First week: daily review of access audit trail
- [ ] First month: weekly review of anomaly detection alerts
- [ ] Patient satisfaction survey at 30 days

## Failure Scenarios (Must Have Graceful Handling)

### What if...
- [ ] Conductor goes down? → Portal shows "Local Demo" mode, queues submissions
- [ ] Patient loses seed phrase? → Clear documentation, support workflow documented
- [ ] FHIR connection drops? → Retry with exponential backoff, alert IT
- [ ] Emergency access is misused? → Audit trail, patient notification, admin review
- [ ] Encryption key is compromised? → Key rotation procedure, re-encrypt all records
- [ ] Hospital terminates pilot? → Patient data export (FHIR R4), crypto-erasure option

## Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Technical Lead | | | |
| Clinical Champion | | | |
| Hospital IT | | | |
| Compliance Officer | | | |
| Patient Advocate | | | |
