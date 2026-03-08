# Mycelix Health: Re-enablement Roadmap

## Current State (Mar 2026)

**Tier 1 MVP** (7 active zomes): patient, provider, records, prescriptions, consent, bridge, shared
**Tier 2 Deferred** (8 zomes, commented in Cargo.toml): trials, insurance, fhir_mapping, fhir_bridge, cds, provider_directory, telehealth, nutrition
**Tier 3 Archived** (22 zomes in `_archive-2026-02-15/`): See archive README for full list

## Architecture Notes

Health is currently a standalone DNA. When ready, add as a unified hApp role.
Privacy considerations for production: separate validator sets, isolated DHT gossip.

## Phase 1: Tier 1 Stabilization (Current)

- [x] 7 MVP zomes compile to wasm32-unknown-unknown
- [x] DNA packed: `dna/health.dna`
- [x] Standalone hApp manifest
- [ ] Add sweettest integration tests (currently 25 unit test files, no conductor tests)
- [ ] Wire to unified hApp as optional role (deferred: false but separate network_seed)
- [ ] Security audit: is_finite() guards on all f64/f32 fields (follow commons pattern)

## Phase 2: Tier 2 Re-enablement

Priority order based on clinical value and implementation complexity:

### 2a. FHIR Bridge (High Priority)
```bash
# 1. Uncomment in Cargo.toml
"zomes/fhir_mapping/integrity",
"zomes/fhir_mapping/coordinator",
"zomes/fhir_bridge/integrity",
"zomes/fhir_bridge/coordinator",

# 2. Update dna/dna.yaml
# Add fhir_mapping_integrity, fhir_bridge_integrity to integrity zomes
# Add fhir_mapping, fhir_bridge to coordinator zomes with dependencies

# 3. Verify compilation
cargo check --target wasm32-unknown-unknown
```

**Why first**: FHIR 4.0 (HL7) interop is table stakes for any health platform. Enables data exchange with existing EHR systems.

### 2b. Provider Directory + Telehealth
```bash
# Uncomment in Cargo.toml
"zomes/provider_directory/integrity",
"zomes/provider_directory/coordinator",
"zomes/telehealth/integrity",
"zomes/telehealth/coordinator",
```

**Why second**: Provider discovery + remote care are core workflow enablers.

### 2c. CDS + Nutrition + Insurance
```bash
# Uncomment remaining Tier 2 zomes
"zomes/cds/integrity",
"zomes/cds/coordinator",
"zomes/nutrition/integrity",
"zomes/nutrition/coordinator",
"zomes/insurance/integrity",
"zomes/insurance/coordinator",
```

**Why third**: Clinical Decision Support, dietary tracking, and insurance are value-adds on top of core workflows.

### 2d. Trials
```bash
"zomes/trials/integrity",
"zomes/trials/coordinator",
```

**Why last in Tier 2**: Clinical trials are specialized; most deployments won't need them initially.

## Phase 3: Selective Tier 3 Restoration

Not all 22 archived zomes need restoration. Prioritize based on demand:

### High Value (restore when needed)
- **Mental Health** — Specialized assessments, therapy tracking
- **Chronic Care** — Long-term condition management
- **SDOH** — Social determinants of health (bridges to commons cluster)
- **Digital Twin** — Patient simulation (bridges to Symthaea)

### Medium Value (restore when needed)
- **Federated Learning** — ML on health data (bridges to mycelix-fl-core)
- **Research Commons** — De-identified data sharing
- **Population Health** — Aggregate analytics
- **Immunity** — Vaccination tracking

### Low Priority (archive indefinitely)
- **HDC Genetics** — Specialized, can stay in Symthaea
- **I18N** — Localization (should be cross-cutting, not health-specific)
- **Mobile Support** — Platform concern, not zome concern

### Restoration Process (per zome)
```bash
# 1. Move from archive
mv _archive-2026-02-15/ZOME_NAME zomes/ZOME_NAME

# 2. Add to Cargo.toml workspace members
"zomes/ZOME_NAME/integrity",
"zomes/ZOME_NAME/coordinator",

# 3. Add to dna/dna.yaml
# integrity → name: ZOME_NAME_integrity, path: ../target/...
# coordinator → name: ZOME_NAME, dependencies: [ZOME_NAME_integrity]

# 4. Check compilation
cargo check --target wasm32-unknown-unknown

# 5. Run existing tests (if any)
cargo test --workspace

# 6. Add sweettest coverage
# Follow personal_workflow.rs pattern in mycelix-workspace/tests/sweettest/
```

## Phase 4: Unified hApp Integration

When health is ready to join the unified hApp:

```yaml
# Add to mycelix-unified-happ.yaml
  - name: health
    provisioning:
      strategy: create
      deferred: false
    dna:
      path: ./health/dna/health.dna
      modifiers:
        network_seed: ~
        properties: ~
      clone_limit: 0
```

Cross-cluster integration points:
- **Identity** → Patient/provider DID verification
- **Civic/Emergency** → Emergency medical escalation
- **Commons** → Health-based resource allocation
- **Governance** → Health policy proposals
- **Finance** → Health service payments via TEND

## Dependencies

- Holochain 0.6.0 (current, compatible)
- getrandom 0.3 with custom backend (see CLAUDE.md)
- Separate flake: `holonix/main-0.6` (health submodule has its own nix config)
