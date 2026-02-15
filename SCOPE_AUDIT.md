# Mycelix-Health Scope Audit

**Date**: 2026-02-06
**Author**: Claude Code Audit
**Purpose**: Assess actual scope vs. claimed 40 zomes and create scope reduction plan

---

## Executive Summary

The Health hApp contains **37 zomes** (not 40 as sometimes claimed), each with integrity + coordinator crates (74 Cargo.toml files total, plus 1 shared crate = 75). This is an extremely ambitious scope that would require a dedicated healthcare development team working full-time for 2+ years to properly implement and maintain.

**Key Finding**: The codebase exhibits significant structural duplication with the Identity hApp, particularly in verifiable credentials. Most zomes are partially implemented with comprehensive type definitions but limited coordinator logic.

---

## Zome Inventory

### Actual Count: 37 Zomes

| Phase | Zome | Lines (Integrity) | Lines (Coordinator) | Status |
|-------|------|-------------------|---------------------|--------|
| **Phase 1: Core** | patient | 281 | 548 | Core |
| | provider | 212 | 396 | Core |
| | records | 298 | 1231 | Core |
| | prescriptions | 290 | 1115 | Core |
| | consent | 1032 | 1727 | Core |
| | trials | 283 | 586 | Defer |
| | insurance | 274 | 572 | Defer |
| | bridge | 203 | 401 | Core |
| **Phase 2: Revolutionary** | advocate | 919 | 1136 | Future |
| | zkhealth | 602 | 1078 | Future |
| | twin | 860 | 1439 | Future |
| | dividends | 844 | 1281 | Future |
| **Commons Extension** | commons | 866 | 1017 | Future |
| | immunity | 675 | 852 | Future |
| | moment | 753 | 873 | Future |
| **Phase 3: Clinical** | fhir_mapping | 677 | 842 | Defer |
| | fhir_bridge | 398 | 1280 | Defer |
| | cds | 865 | 983 | Defer |
| | provider_directory | 529 | 578 | Defer |
| | telehealth | 559 | 703 | Defer |
| **Phase 4: Equity** | sdoh | 341 | 589 | Future |
| | mental_health | 389 | 1381 | Future |
| | chronic_care | 361 | 680 | Future |
| | pediatric | 480 | 811 | Future |
| **Phase 5: Research** | research_commons | 298 | 603 | Future |
| | trial_matching | 303 | 521 | Future |
| | irb | 297 | 532 | Future |
| | federated_learning | 381 | 687 | Future |
| | population_health | 441 | 784 | Future |
| **Phase 6: Global** | ips | 537 | 989 | Future |
| | i18n | 294 | 477 | Future |
| | disaster_response | 500 | 712 | Future |
| | verifiable_credentials | 635 | 889 | **DUPLICATE** |
| | mobile_support | 595 | 954 | Future |
| **Additional** | credentials | 281 | 407 | **DUPLICATE** |
| | hdc_genetics | 480 | 663 | Future |
| | nutrition | 398 | 1109 | Defer |
| **Shared** | shared | 1784 (combined) | - | Core |

**Total Integrity Lines**: ~19,225
**Total Coordinator Lines**: ~31,661
**Total Test Lines**: ~10,808

---

## Duplication Analysis

### Critical: Verifiable Credentials Duplication

**Problem**: The Health hApp has TWO verifiable credential implementations:
1. `zomes/credentials/` - Simple health credentials (281 lines integrity)
2. `zomes/verifiable_credentials/` - Full W3C VC implementation (635 lines integrity)

**Additionally**, the Identity hApp has its own VC implementation:
- `mycelix-identity/zomes/verifiable_credential/` - W3C VC 2.0 compliant (469 lines)

**Impact**:
- 3 separate implementations of the same W3C standard
- Maintenance burden multiplied 3x
- Divergent validation logic
- No shared types or traits

### Comparison: Health vs Identity Verifiable Credentials

| Feature | Health `verifiable_credentials` | Health `credentials` | Identity `verifiable_credential` |
|---------|--------------------------------|---------------------|----------------------------------|
| W3C VC 2.0 Compliance | Partial | No | Full |
| Health-specific claims | Yes (vaccination, lab results) | Basic | No |
| Selective disclosure | No | No | Yes (DerivedCredential) |
| Revocation registry | Yes (batch) | Simple | No (uses separate revocation zome) |
| Lines of code | 1524 | 688 | 469 |

**Recommendation**: Delete `credentials` zome entirely. Health-specific claim types should be added to Identity's VC implementation via bridge, not duplicated.

---

## Scope Reduction Plan

### Tier 1: Essential MVP (Keep) - 6 Zomes

These are the core zomes needed for a functional health records application:

| Zome | Purpose | Justification |
|------|---------|---------------|
| `patient` | Patient identity and demographics | Core data model |
| `provider` | Provider credentials and verification | Required for trust |
| `records` | Medical records (encounters, diagnoses, labs) | Primary use case |
| `consent` | Access control and audit logging | HIPAA compliance |
| `prescriptions` | Medication management | High-value feature |
| `bridge` | Mycelix ecosystem integration | Cross-hApp federation |
| `shared` | Common utilities, access control | Required by all |

**Estimated effort**: 3-4 months to production-ready

### Tier 2: Defer to Post-MVP (Remove from DNA) - 9 Zomes

These provide value but are not essential for initial launch:

| Zome | Reason to Defer |
|------|-----------------|
| `trials` | Complex regulatory requirements (21 CFR Part 11) |
| `insurance` | Requires EDI integration, not needed for MVP |
| `fhir_mapping` | Can use external gateway instead |
| `fhir_bridge` | External service, not zome |
| `cds` | Requires drug database, regulatory approval |
| `provider_directory` | NPI lookup can be external service |
| `telehealth` | Video infrastructure external |
| `credentials` | **DELETE** - duplicates identity hApp |
| `nutrition` | Nice-to-have, not core |

### Tier 3: Future Vision (Remove Entirely) - 22 Zomes

These are aspirational features that should not be in the codebase until there is a team and funding to implement them properly:

| Category | Zomes | Reason |
|----------|-------|--------|
| Revolutionary Features | `advocate`, `zkhealth`, `twin`, `dividends` | Require ML/AI, complex cryptography |
| Commons Extension | `commons`, `immunity`, `moment` | Philosophical vision, not practical MVP |
| Equity & Access | `sdoh`, `mental_health`, `chronic_care`, `pediatric` | Each is its own product |
| Research | `research_commons`, `trial_matching`, `irb`, `federated_learning`, `population_health` | Entire research platform |
| Global Scale | `ips`, `i18n`, `disaster_response`, `verifiable_credentials`, `mobile_support`, `hdc_genetics` | Premature optimization |

---

## Identity Integration Points

Instead of duplicating VC infrastructure, Health hApp should use Identity hApp via bridge:

### Current (Problematic)
```
Health hApp
  ├── verifiable_credentials/  (635 lines, partial W3C)
  ├── credentials/             (281 lines, simple)
  └── ...

Identity hApp
  ├── verifiable_credential/   (469 lines, full W3C 2.0)
  ├── credential_schema/
  └── revocation/
```

### Proposed (Clean)
```
Health hApp
  ├── patient/
  ├── records/
  ├── consent/
  ├── prescriptions/
  ├── provider/
  ├── bridge/         <-- Uses Identity for VCs
  └── shared/

Identity hApp (source of truth for VCs)
  ├── verifiable_credential/
  ├── credential_schema/
  └── health_credential_types/  <-- NEW: Health-specific claim types
```

### Bridge Functions Needed

```rust
// In bridge coordinator:

/// Request a health credential from Identity hApp
pub fn request_health_credential(
    input: HealthCredentialRequest,
) -> ExternResult<ActionHash> {
    // Cross-cell call to Identity hApp
    call(
        CallTargetCell::OtherRole("mycelix-identity".into()),
        "verifiable_credential",
        "request_credential".into(),
        None,
        &input.into_vc_request(),
    )
}

/// Verify a health credential via Identity hApp
pub fn verify_health_credential(
    credential_hash: ActionHash,
) -> ExternResult<VerificationResult> {
    call(
        CallTargetCell::OtherRole("mycelix-identity".into()),
        "verifiable_credential",
        "verify_credential".into(),
        None,
        &credential_hash,
    )
}
```

---

## Recommended Actions

### Immediate (Week 1-2)

1. **Delete `credentials` zome** - It's a less capable duplicate
2. **Archive Tier 3 zomes** to `_archive/` directory with explanation
3. **Update Cargo.toml** to only build Tier 1 zomes
4. **Update dna.yaml** to only include Tier 1 zomes

### Short-term (Month 1)

1. **Add health credential types to Identity hApp** (`VaccinationCredential`, `LabResultCredential`, `MedicalLicenseCredential`)
2. **Update bridge zome** to use Identity for VC operations
3. **Write integration tests** for cross-hApp credential flow
4. **Remove `verifiable_credentials` zome** from Health DNA after migration

### Medium-term (Month 2-3)

1. **Production-harden Tier 1 zomes** (error handling, pagination, validation)
2. **Add comprehensive tests** for remaining 6 zomes
3. **Document API** for SDK integration
4. **Security audit** consent and access control

---

## Effort Estimates

| Scope | Zomes | Estimated Effort | Team Size |
|-------|-------|------------------|-----------|
| Current (37 zomes) | All | 24+ months | 5+ developers |
| Tier 1 MVP (6 zomes) | Core only | 3-4 months | 2 developers |
| Tier 1+2 (15 zomes) | Core + deferred | 8-12 months | 3 developers |

---

## Conclusion

The Health hApp is currently scoped as an entire healthcare platform rather than a focused product. By reducing to 6 core zomes:

- **Maintenance burden drops 85%** (6 vs 37 zomes)
- **Code complexity drops 80%** (~8K lines vs ~50K lines)
- **Time to MVP drops from years to months**
- **Duplication with Identity hApp eliminated**

The deferred and future zomes are not deleted permanently - they represent a valid vision. However, they should not be in the active codebase until there is dedicated team capacity to implement them properly.

**Recommended MVP scope**: `patient`, `provider`, `records`, `consent`, `prescriptions`, `bridge`, `shared`

---

*This audit was generated by analyzing the codebase at `/srv/luminous-dynamics/mycelix-health/`*
