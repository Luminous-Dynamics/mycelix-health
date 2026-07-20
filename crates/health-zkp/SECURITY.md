# Health ZKP Security Status

The crate is **fail-closed by default**. No current proof system in this crate
is suitable for clinical authorization, insurance eligibility, trial admission,
employment screening, or compliance decisions.

## Default behavior

- Range-proof generation returns a domain-separated SHA-256 commitment
  placeholder.
- Commitment placeholders carry `security_bits = 0` and are always rejected by
  `verify_proof`.
- RISC0 receipts are always rejected because no health guest image ID and
  journal-to-claim binding are implemented.
- Proof timestamps, expiry, byte length, and data timestamps are checked against
  a trusted verifier time.

## Experimental Winterfell range proof

The `experimental-unbound-range-proofs` feature enables generation and
cryptographic verification of the existing Winterfell AIR. This proves that the
trace satisfies its current constraints, but the AIR does not yet bind the
hidden value to `HealthPublicInputs::data_commitment`. A prover can therefore
construct a valid trace unrelated to the committed health record.

The feature must never be enabled in a production policy path.

## Required work before production

1. Define a canonical commitment over the source observation, units, timestamp,
   patient pseudonym, attestor, and proof type.
2. Bind that commitment to the AIR or zkVM journal as a public input.
3. Constrain the hidden value reconstruction and range comparison completely.
4. Bind proof-type-specific semantics, including units and threshold direction.
5. Add replay protection and an explicit maximum attestation lifetime.
6. Add adversarial tests for forged traces, substituted commitments, changed
   public inputs, stale proofs, and cross-patient replay.
7. Obtain independent cryptographic review before enabling policy enforcement.
