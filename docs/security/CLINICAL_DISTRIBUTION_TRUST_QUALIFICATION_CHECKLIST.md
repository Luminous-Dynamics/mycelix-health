# Clinical Distribution Trust V1 — Qualification Checklist

## Pure trust-composition gates

- [ ] unsupported trust-policy version fails closed;
- [ ] missing trust-policy identity fails closed;
- [ ] malformed detector/admission/policy digests fail closed;
- [ ] invalid evaluator-admission validity/revocation windows fail closed;
- [ ] structural preflight must match the exact structural distribution policy;
- [ ] trust policy must bind the exact structural distribution-policy digest;
- [ ] trust policy detector must match the structural policy detector;
- [ ] evaluator admission must bind the exact trust-policy digest;
- [ ] evaluator admission detector must match the trust policy detector;
- [ ] admission must be active when the distribution assessment was produced;
- [ ] admission must still be active when trust is evaluated;
- [ ] future-skew bounds fail closed;
- [ ] trust-receipt digest is deterministic and domain-separated;
- [ ] trust receipt is non-serializable and non-cloneable.

## Adversarial gates

- [ ] replay one evaluator admission under another trust policy -> reject;
- [ ] substitute another detector under the same trust policy -> reject;
- [ ] substitute another structural distribution policy -> reject;
- [ ] qualify an assessment produced before admission became active -> reject;
- [ ] qualify after evaluator admission expired -> reject;
- [ ] qualify after evaluator admission revocation -> reject;
- [ ] alter admission-evidence digest -> receipt identity changes;
- [ ] alter assessment digest -> receipt identity changes;
- [ ] alter trust-policy digest -> receipt identity changes.

## Runtime/configuration promotion blockers

Before this trust receipt can unlock model-backed clinical presentation:

- [ ] deployment trust roots are pinned independently of request input;
- [ ] the adapter creating `VerifiedDistributionEvaluatorAdmission` independently verifies its source record;
- [ ] untrusted callers cannot invoke an equivalent self-admission path with arbitrary evidence;
- [ ] revocation/supersession semantics are qualified under the selected runtime consistency model;
- [ ] root/key rotation semantics are explicit;
- [ ] modified-adapter tests prove admission cannot be bypassed;
- [ ] future clinical presentation permit binds the exact distribution trust-receipt identity;
- [ ] raw structural OOD evidence alone remains incapable of minting a permit.

## Promotion rule

Keep this tranche draft until exact-head CI passes. Even after pure tests pass, model-backed clinician presentation remains blocked until the runtime/configuration trust root is separately qualified.
