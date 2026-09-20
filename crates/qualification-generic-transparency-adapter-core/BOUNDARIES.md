# QUAL-EVID-007 Boundaries

## This reference may establish

A successful exact-head qualifier may establish only the semantic interoperability theorem implemented by this crate:

- exact generic protocol/schema checks;
- exact Health generic domain identity;
- exact generic subject ↔ Health lineage binding;
- exact checkpoint epoch mapping;
- canonical Health state transcript framing;
- exact generic state-commitment equality to a separately verified derivation;
- exact anchor sequence/predecessor shape;
- exact anchor/nonce/witness/trust identity mapping into Health types;
- generic freshness/expiry checks;
- separately bound Health security-time-floor evidence;
- output implementing Health `VerifiedExternalAnchorProof` only after all mapping checks succeed;
- fail-closed rejection of generic recovered lineages in V1.

## This reference does not establish

It does **not** establish:

- that draft generic PR #2023 is qualified;
- that generic #2038 accepted-anchor API exists yet;
- generic signature/quorum/trust correctness;
- cryptographic correctness of the state-commitment derivation adapter;
- trusted-time source correctness;
- witness independence or key custody;
- durable network/hardware anti-rollback;
- Health #220 qualification;
- Patient-v2 activation;
- clinical validity or legal/regulatory compliance.

## Generic accepted-anchor seam

`VerifiedGenericAcceptedAnchorV1` is a pinned future compatibility contract, not a claim about the current generic crate's public API. `Luminous-Dynamics/luminous-dynamics#2038` tracks producing one complete typed accepted-anchor identity without pairing a head with an arbitrary second proof object.

## Recovery policy

V1 deliberately rejects generic recovered lineages. Recovery acceptance requires a future explicit policy mapping that binds generic resolved-conflict/recovery identity into Health governance.

## Time theorem

Generic freshness and Health security time are independent. The adapter requires both. A fresh generic anchor does not create a trusted Health security-time floor.

## PHI boundary

The adapter maps only opaque digests, counters, anchor metadata, witness/trust commitments, and time evidence. No patient identity, care payload, clinical assertion, or raw qualification prose is required.
