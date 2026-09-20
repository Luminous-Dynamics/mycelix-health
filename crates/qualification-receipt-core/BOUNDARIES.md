# Qualification receipt adapter boundaries

This reference core consumes already-verified adapter facts. Those facts are not self-authenticating booleans.

Production integration must independently qualify at least:

1. canonical transcript hashing and receipt-digest computation;
2. signature-suite implementation and domain separation;
3. signer public-key parsing and key-ID binding;
4. trust-store identity, active/revoked/compromised state and historical verification time;
5. governance-policy identity and threshold/role/organization evaluation;
6. trusted time;
7. durable nonce consumption;
8. durable per-lineage sequence/predecessor state;
9. distributed checkpoint/reconciliation semantics where multiple authorities participate;
10. creation of product-facing `VerifiedQualification<T>` adapters for #204/#206/#208.

The raw lower-level `ApproverVerification` and `verify_qualification_bundle` functions live in the private `model` module and are intentionally not re-exported from the crate.

`BoundApproverVerification::from_verifier_adapter(...)` is still an adapter seam for reference testing. It must not be exposed directly as a remote/UI/zome authority API in production.

A model, MATL score, Git branch, PR merge, GitHub status check, or UI toggle cannot create a qualification receipt.
