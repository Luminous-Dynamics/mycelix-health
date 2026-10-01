# Clinical Causality Qualification v1

Status: experimental. This contract is not clinical qualification and requires executable CI plus runtime/DHT qualification before promotion.

## Purpose

A valid causal assessment is not automatically a trusted clinical/research result. Qualification composes four distinct objects:

1. the exact causal assessment under an exact causal-assessment policy;
2. a trust receipt admitting the exact observed event, exposure association, and every evidence artifact for its exact use;
3. policy-bound professional `ClinicalReview` authority targeting the exact assessment;
4. one qualification policy that pins all subordinate policy digests and freshness bounds.

## Exact policy stack

`CausalQualificationPolicyV1` commits to:

- exact causal-assessment policy digest;
- exact causal-evidence trust-policy digest;
- exact assessor-authority policy digest;
- maximum assessment age;
- maximum authority age;
- maximum evidence-trust age;
- maximum tolerated future clock skew.

Callers cannot independently substitute weaker expected policies at qualification time.

## Authority binding

The qualifier accepts only `PolicyBoundAuthorityPermit`, produced by `mycelix-clinical-authority-policy`. The bound authority policy hashes the exact purpose, jurisdiction, credential requirements, and scope requirements, then constructs the underlying legacy authority request internally.

The public causal-qualification API does not accept the older unbound `AuthorityPermit`.

## Evidence trust

The trust receipt must match the exact:

- assessment;
- observed event;
- exposure association;
- causal-assessment policy;
- deployment causal-evidence trust policy.

Trust admission is use-scoped. An artifact admitted as an aggregate signal cannot silently substitute for patient-specific objective confirmation.

## Positive conclusions

A positive conclusion (`EvidenceSuggestsRelationship` or `EvidenceSupportsRelationship`) additionally requires separately evidenced `TemporalPlausibility` supporting the relationship. Mere chronology is insufficient.

## Output

Successful composition yields a non-cloneable, non-serializable `QualifiedCausalAssessmentCapability`. Consuming it produces a serializable audit receipt carrying policy/evidence identities and assessor provenance. Deserializing that receipt does not recreate authority.

## Non-claims

Qualification does not prove population-level causal effect, regulatory reportability, physical truth of an observation, external legitimacy of unverified evidence, or DHT provenance. Deployment policy selection must itself come from a trusted runtime/DNA configuration boundary. A later tranche must add DNA-rooted attestation and correction/revocation lineage.

## Migration blocker

P0 #75 tracks migration of other high-assurance workflows from caller-supplied `AuthorityRequest` / unbound `AuthorityPermit` to policy-bound authority.
