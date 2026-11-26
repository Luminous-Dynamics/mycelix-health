# Mycelix Health

**A Holochain-based research application for patient-controlled health information, consent-aware sharing, and health-data workflows.**

Mycelix Health explores an architecture in which patients control authorization to share health information with providers and other permitted participants. The repository contains Rust integrity/coordinator zomes, shared validation and cryptography crates, and application code. It remains a research project and is **not a clinically validated or regulatory-certified health-record system**.

> **Important:** No HIPAA, GDPR, 42 CFR Part 2, FHIR, or other compliance certification is claimed. The internal [compliance status tracker](COMPLIANCE_STATUS.md) is a development gap register, not an audit, legal opinion, or evidence of compliance. Independent security, privacy, clinical, operational, and legal assessment would be required before handling real patient data.

## Current repository scope

The active root Cargo workspace currently includes 17 zome domains, each with integrity and coordinator crates:

- Patient, provider, records, prescriptions, and consent;
- bridge and cross-application integration;
- trials and insurance;
- FHIR mapping and FHIR bridge;
- clinical decision support (CDS) and provider directory;
- telehealth and nutrition;
- mental health, dividends, and health credentials.

It also includes shared zome types and helper crates for HDC experiments, health validation, federated-learning support, cryptography, and zero-knowledge proof experiments.

**Scope warning:** additional feature areas described in older sections of this README or in historical plans have been moved under \`_archive-2026-02-15/\` and are not active root workspace members. Check the current root \`Cargo.toml\` before assuming an archived zome is buildable or part of the active DNA.

## Architecture

- **Integrity zomes** define entries, links, and validation rules.
- **Coordinator zomes** expose application workflows and compose integrity operations.
- **Shared types and validation** provide common structures and checks.
- **Crypto and proof crates** explore encryption, credential, and predicate-proof mechanisms.
- **Applications and services** provide user-interface and integration surfaces whose maturity varies independently from the zome workspace.

The application is designed around explicit authorization and consent boundaries. A data-sharing feature should not be considered complete merely because a data structure or coordinator function exists; verify the relevant integrity validation, authorization path, revocation behavior, audit behavior, and integration tests.

## Build and test

The project uses the Rust workspace and includes Nix-based development support. See [RUNNING.md](RUNNING.md) and [CONTRACT.md](CONTRACT.md) for environment setup and the standalone-checkout dependency contract.

\`\`\`sh
# Install the WASM target if it is not provided by your environment
rustup target add wasm32-unknown-unknown

# Check and test active Cargo workspace members
cargo check --workspace
cargo test --workspace

# Build WASM zomes
cargo build --workspace --target wasm32-unknown-unknown --release
\`\`\`

The repository's \`tests/\` directory is excluded from the root Cargo workspace and has its own manifest. It is a separate test package; root \`cargo test --workspace\` does not automatically include it. Consult \`RUNNING.md\` for the supported invocation and conductor setup.

### Standalone checkout dependency layout

Some path dependencies intentionally resolve to sibling directories from the parent \`mycelix\` repository, including shared bridge, Leptos, ZKP, and federated-learning crates. A bare clone of this repository may therefore not contain every path dependency. The CI workflow reconstructs the required sibling layout from the public parent repository; see [CONTRACT.md](CONTRACT.md) before attempting a standalone build.

## Security and maturity

The project contains useful implementation work, but code presence is not end-to-end assurance. In particular:

- the active workspace is smaller than the historical feature catalogue;
- the internal compliance document is dated and requires re-verification against current law and implementation;
- no independent cryptographic or application security audit is claimed;
- no clinical validation or production readiness is claimed;
- any use involving real health information requires an independently reviewed deployment, consent, key-management, incident-response, and data-governance model.

## Useful documents

- [Running and development guide](RUNNING.md)
- [Standalone build contract](CONTRACT.md)
- [Compliance gap tracker](COMPLIANCE_STATUS.md)
- [Scope audit](SCOPE_AUDIT.md)
- [Cryptographic audit guide](CRYPTO_AUDIT_GUIDE.md)
- [Contributing](CONTRIBUTING.md)

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
