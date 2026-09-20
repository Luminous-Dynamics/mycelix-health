# Qualification Receipt Ed25519 v1 evidence status

**SOURCE-STAGED / UNEXECUTED in the current authoring environment / CI REQUESTED / NO PASS CLAIM.**

The exact stacked subject must execute its dedicated GitHub Actions workflow before any cryptographic/governance PASS is claimed.

The workflow's scope is limited to:

- Python syntax/compile checks;
- the committed Rust/Python transcript golden vector;
- real OpenSSL Ed25519 signing/verification with ephemeral runner keys;
- governed bundle validation and independent reconstruction;
- exact #208 profile matching while preserving the explicit non-authority boundary.

It does not qualify production private-key custody, durable nonce consumption, durable/current lineage-head checkpoints, trusted clocks, Rust #212 execution, Patient-v2 product integration, clinical validity, confidentiality of unrelated data, or legal/regulatory compliance.
