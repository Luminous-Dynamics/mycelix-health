// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed evidence gates for community-grown botanical lots.
//!
//! This crate checks whether a documented set of evidence meets a defined
//! checklist for a particular hand-off stage. It is not a plant identifier,
//! clinical decision support system, legal-authority registry, laboratory
//! information system, manufacturing control system, or release authority.
//!
//! A passing decision means only that the locally supplied checklist is
//! structurally satisfied. It does not authenticate evidence bytes, verify a
//! licence with a regulator, establish efficacy, or authorize treatment or
//! product distribution. Independent review and jurisdiction-specific rules
//! remain mandatory.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A gate for which a reviewer must provide an explicit, scoped assessment.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum GateKey {
    SpeciesIdentity,
    LawfulPropagation,
    BiodiversityProtection,
    KnowledgeRightsAndBenefitSharing,
    SiteHazardReview,
    HarvestTraceability,
    StorageAndHandling,
    BuyerAuthorization,
    IncomingQualityRequirements,
    ManufacturingAuthorization,
    ManufacturingProcessValidation,
    FinishedProductQualityTesting,
    ProductAuthorization,
    StabilityAndPackaging,
    QualifiedReleaseDecision,
    AdverseEventAndRecallReadiness,
}

/// Evidence scope must match the gate, jurisdiction, and subject being evaluated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceScope {
    /// Botanical lot for cultivation/supply gates; finished-product batch for
    /// manufacturing and post-manufacture gates.
    pub subject_id: String,
    pub gate: GateKey,
    /// Versioned jurisdiction identifier used by the policy/evidence owner.
    pub jurisdiction: String,
}

/// Hash-linked evidence metadata. The digest is shape-checked only; this crate
/// does not fetch evidence or verify that the digest matches its contents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Stable evidence identifier in the responsible evidence system.
    pub evidence_id: String,
    /// SHA-256 digest in lowercase hexadecimal (64 characters).
    pub sha256: String,
    /// Stable person or organization identifier for the reviewer.
    pub reviewer_id: String,
    /// UTC review timestamp in YYYY-MM-DDTHH:MM:SSZ form.
    pub reviewed_at: String,
    /// Structured scope; evaluated for exact gate, subject, and jurisdiction match.
    pub scope: EvidenceScope,
}

/// An evidence state. Unassessed and failed gates never count as satisfied.
/// Not-applicable is deliberately restricted to the knowledge-rights gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GateAssessment {
    NotAssessed,
    Pass { evidence: EvidenceRef },
    Fail { evidence: EvidenceRef },
    NotApplicable {
        evidence: EvidenceRef,
        rationale: String,
    },
}

/// The stage whose minimum evidence checklist is being evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityTarget {
    /// A traceable, reviewed botanical lot may be presented for evaluation.
    /// This is not an assertion that the lot is medicinally effective.
    TraceableBotanicalSupply,
    /// A qualified buyer's receiving specification and authorization are met.
    /// This is not an authorization to manufacture or dispense a medicine.
    AuthorizedManufacturingIntake,
    /// Evidence for finished-product release review is complete.
    /// This does not replace a qualified person's real release decision.
    FinishedProductRelease,
}

impl GateKey {
    /// Whether evidence for this gate must be scoped to the linked finished-product batch.
    pub const fn uses_finished_product_scope(self) -> bool {
        matches!(
            self,
            Self::ManufacturingAuthorization
                | Self::ManufacturingProcessValidation
                | Self::FinishedProductQualityTesting
                | Self::ProductAuthorization
                | Self::StabilityAndPackaging
                | Self::QualifiedReleaseDecision
                | Self::AdverseEventAndRecallReadiness
        )
    }
}

impl EligibilityTarget {
    /// Required gates for this workflow stage, in stable order.
    pub const fn required_gates(self) -> &'static [GateKey] {
        use GateKey as G;
        const BOTANICAL: &[G] = &[
            G::SpeciesIdentity,
            G::LawfulPropagation,
            G::BiodiversityProtection,
            G::KnowledgeRightsAndBenefitSharing,
            G::SiteHazardReview,
            G::HarvestTraceability,
            G::StorageAndHandling,
        ];
        const INTAKE: &[G] = &[
            G::SpeciesIdentity,
            G::LawfulPropagation,
            G::BiodiversityProtection,
            G::KnowledgeRightsAndBenefitSharing,
            G::SiteHazardReview,
            G::HarvestTraceability,
            G::StorageAndHandling,
            G::BuyerAuthorization,
            G::IncomingQualityRequirements,
        ];
        const FINISHED_PRODUCT: &[G] = &[
            G::SpeciesIdentity,
            G::LawfulPropagation,
            G::BiodiversityProtection,
            G::KnowledgeRightsAndBenefitSharing,
            G::SiteHazardReview,
            G::HarvestTraceability,
            G::StorageAndHandling,
            G::BuyerAuthorization,
            G::IncomingQualityRequirements,
            G::ManufacturingAuthorization,
            G::ManufacturingProcessValidation,
            G::FinishedProductQualityTesting,
            G::ProductAuthorization,
            G::StabilityAndPackaging,
            G::QualifiedReleaseDecision,
            G::AdverseEventAndRecallReadiness,
        ];
        match self {
            Self::TraceableBotanicalSupply => BOTANICAL,
            Self::AuthorizedManufacturingIntake => INTAKE,
            Self::FinishedProductRelease => FINISHED_PRODUCT,
        }
    }
}

/// Machine-readable reason a checklist was not satisfied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerCode {
    EmptyBatchId,
    EmptyJurisdiction,
    MissingFinishedProductBatchId,
    MissingAssessment,
    NotAssessed,
    GateFailed,
    InvalidEvidence,
    NotApplicableNotPermitted,
    EmptyNotApplicableRationale,
}

/// A deterministic explanation; blocker order is stable by required-gate order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateBlocker {
    pub gate: Option<GateKey>,
    pub code: BlockerCode,
    pub detail: String,
}

/// Result of a checklist evaluation. This is deliberately not named
/// "approval" or "release": no external authorization is conferred.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EligibilityDecision {
    pub batch_id: String,
    pub target: EligibilityTarget,
    pub requirement_set_satisfied: bool,
    pub blockers: Vec<GateBlocker>,
}

/// Evidence bundle for one cultivation/harvest lot and, where applicable, a
/// linked finished-product batch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotanicalBatchRecord {
    pub batch_id: String,
    /// Jurisdiction whose versioned criteria apply to this evidence evaluation.
    pub jurisdiction: String,
    pub assessments: BTreeMap<GateKey, GateAssessment>,
    /// Required only for FinishedProductRelease. Must point to an independently
    /// tracked finished-product batch rather than treating raw herb as a medicine.
    pub finished_product_batch_id: Option<String>,
}

impl BotanicalBatchRecord {
    /// Evaluate required gates without making clinical or regulatory decisions.
    ///
    /// Extra assessments do not compensate for missing required gates. Only
    /// KnowledgeRightsAndBenefitSharing may be marked not-applicable, and then
    /// only with valid evidence metadata and a non-empty rationale.
    pub fn evaluate(&self, target: EligibilityTarget) -> EligibilityDecision {
        let mut blockers = Vec::new();

        if self.batch_id.trim().is_empty() {
            blockers.push(GateBlocker {
                gate: None,
                code: BlockerCode::EmptyBatchId,
                detail: "batch_id must be non-empty".to_owned(),
            });
        }

        if self.jurisdiction.trim().is_empty() {
            blockers.push(GateBlocker {
                gate: None,
                code: BlockerCode::EmptyJurisdiction,
                detail: "jurisdiction must be non-empty".to_owned(),
            });
        }

        if target == EligibilityTarget::FinishedProductRelease
            && self
                .finished_product_batch_id
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            blockers.push(GateBlocker {
                gate: None,
                code: BlockerCode::MissingFinishedProductBatchId,
                detail: "finished-product release requires a linked product batch id"
                    .to_owned(),
            });
        }

        for gate in target.required_gates() {
            match self.assessments.get(gate) {
                None => blockers.push(GateBlocker {
                    gate: Some(*gate),
                    code: BlockerCode::MissingAssessment,
                    detail: "required gate has no assessment".to_owned(),
                }),
                Some(GateAssessment::NotAssessed) => blockers.push(GateBlocker {
                    gate: Some(*gate),
                    code: BlockerCode::NotAssessed,
                    detail: "required gate is explicitly unassessed".to_owned(),
                }),
                Some(GateAssessment::Pass { evidence }) => {
                    self.append_evidence_blockers(*gate, evidence, &mut blockers);
                }
                Some(GateAssessment::Fail { evidence }) => {
                    self.append_evidence_blockers(*gate, evidence, &mut blockers);
                    blockers.push(GateBlocker {
                        gate: Some(*gate),
                        code: BlockerCode::GateFailed,
                        detail: "required gate has a negative assessment".to_owned(),
                    });
                }
                Some(GateAssessment::NotApplicable { evidence, rationale }) => {
                    self.append_evidence_blockers(*gate, evidence, &mut blockers);
                    if *gate != GateKey::KnowledgeRightsAndBenefitSharing {
                        blockers.push(GateBlocker {
                            gate: Some(*gate),
                            code: BlockerCode::NotApplicableNotPermitted,
                            detail: "this gate cannot be waived as not-applicable".to_owned(),
                        });
                    }
                    if rationale.trim().is_empty() {
                        blockers.push(GateBlocker {
                            gate: Some(*gate),
                            code: BlockerCode::EmptyNotApplicableRationale,
                            detail: "not-applicable requires a reasoned, reviewable rationale"
                                .to_owned(),
                        });
                    }
                }
            }
        }

        EligibilityDecision {
            batch_id: self.batch_id.clone(),
            target,
            requirement_set_satisfied: blockers.is_empty(),
            blockers,
        }
    }

    fn append_evidence_blockers(
        &self,
        gate: GateKey,
        evidence: &EvidenceRef,
        blockers: &mut Vec<GateBlocker>,
    ) {
        let mut issues = evidence.validation_issues();
        if evidence.scope.gate != gate {
            issues.push("scope gate does not match the assessed gate".to_owned());
        }
        let expected_subject_id = if gate.uses_finished_product_scope() {
            self.finished_product_batch_id.as_deref().unwrap_or_default()
        } else {
            self.batch_id.as_str()
        };
        if evidence.scope.subject_id != expected_subject_id {
            issues.push("scope subject_id does not match the expected batch".to_owned());
        }
        if evidence.scope.jurisdiction != self.jurisdiction {
            issues.push("scope jurisdiction does not match the record jurisdiction".to_owned());
        }
        if !issues.is_empty() {
            blockers.push(GateBlocker {
                gate: Some(gate),
                code: BlockerCode::InvalidEvidence,
                detail: issues.join("; "),
            });
        }
    }
}

impl EvidenceRef {
    /// Return every shape-validation issue. No claim is made about authenticity,
    /// legal permission, signer authority, or the bytes behind this reference.
    pub fn validation_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.evidence_id.trim().is_empty() {
            issues.push("evidence_id is empty".to_owned());
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            issues.push("sha256 must be 64 lowercase hexadecimal characters".to_owned());
        }
        if self.reviewer_id.trim().is_empty() {
            issues.push("reviewer_id is empty".to_owned());
        }
        if !is_valid_utc_timestamp(&self.reviewed_at) {
            issues.push("reviewed_at must be a valid UTC YYYY-MM-DDTHH:MM:SSZ timestamp".to_owned());
        }
        if self.scope.subject_id.trim().is_empty() {
            issues.push("scope subject_id is empty".to_owned());
        }
        if self.scope.jurisdiction.trim().is_empty() {
            issues.push("scope jurisdiction is empty".to_owned());
        }
        issues
    }
}

fn is_valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }

    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }

    let number = |start: usize, end: usize| -> u32 {
        value[start..end].parse::<u32>().unwrap_or(u32::MAX)
    };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let hour = number(11, 13);
    let minute = number(14, 16);
    let second = number(17, 19);

    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_GATES: [GateKey; 16] = [
        GateKey::SpeciesIdentity,
        GateKey::LawfulPropagation,
        GateKey::BiodiversityProtection,
        GateKey::KnowledgeRightsAndBenefitSharing,
        GateKey::SiteHazardReview,
        GateKey::HarvestTraceability,
        GateKey::StorageAndHandling,
        GateKey::BuyerAuthorization,
        GateKey::IncomingQualityRequirements,
        GateKey::ManufacturingAuthorization,
        GateKey::ManufacturingProcessValidation,
        GateKey::FinishedProductQualityTesting,
        GateKey::ProductAuthorization,
        GateKey::StabilityAndPackaging,
        GateKey::QualifiedReleaseDecision,
        GateKey::AdverseEventAndRecallReadiness,
    ];

    fn evidence_for(gate: GateKey) -> EvidenceRef {
        EvidenceRef {
            evidence_id: format!("synthetic-evidence-{gate:?}"),
            sha256: "a".repeat(64),
            reviewer_id: "synthetic-reviewer".to_owned(),
            reviewed_at: "2026-01-02T03:04:05Z".to_owned(),
            scope: EvidenceScope {
                subject_id: if gate.uses_finished_product_scope() {
                    "SYNTHETIC-PRODUCT-001".to_owned()
                } else {
                    "SYNTHETIC-BATCH-001".to_owned()
                },
                gate,
                jurisdiction: "ZA".to_owned(),
            },
        }
    }

    fn complete_record() -> BotanicalBatchRecord {
        let assessments = ALL_GATES
            .into_iter()
            .map(|gate| {
                (
                    gate,
                    GateAssessment::Pass {
                        evidence: evidence_for(gate),
                    },
                )
            })
            .collect();
        BotanicalBatchRecord {
            batch_id: "SYNTHETIC-BATCH-001".to_owned(),
            jurisdiction: "ZA".to_owned(),
            assessments,
            finished_product_batch_id: Some("SYNTHETIC-PRODUCT-001".to_owned()),
        }
    }

    fn decision_has_code(decision: &EligibilityDecision, code: BlockerCode) -> bool {
        decision.blockers.iter().any(|blocker| blocker.code == code)
    }

    #[test]
    fn empty_record_fails_closed_with_missing_assessments() {
        let record = BotanicalBatchRecord {
            batch_id: "SYNTHETIC-BATCH-001".to_owned(),
            jurisdiction: "ZA".to_owned(),
            assessments: BTreeMap::new(),
            finished_product_batch_id: None,
        };
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(!decision.requirement_set_satisfied);
        assert_eq!(decision.blockers.len(), 7);
        assert!(decision_has_code(&decision, BlockerCode::MissingAssessment));
    }

    #[test]
    fn fully_evidenced_synthetic_chain_satisfies_all_three_checklists() {
        let record = complete_record();
        for target in [
            EligibilityTarget::TraceableBotanicalSupply,
            EligibilityTarget::AuthorizedManufacturingIntake,
            EligibilityTarget::FinishedProductRelease,
        ] {
            let decision = record.evaluate(target);
            assert!(decision.requirement_set_satisfied, "{decision:?}");
            assert!(decision.blockers.is_empty());
        }
    }

    #[test]
    fn missing_buyer_gates_do_not_block_the_earlier_botanical_lane() {
        let mut record = complete_record();
        record.assessments.remove(&GateKey::BuyerAuthorization);
        record.assessments.remove(&GateKey::IncomingQualityRequirements);

        assert!(record
            .evaluate(EligibilityTarget::TraceableBotanicalSupply)
            .requirement_set_satisfied);
        let intake = record.evaluate(EligibilityTarget::AuthorizedManufacturingIntake);
        assert!(!intake.requirement_set_satisfied);
        assert_eq!(
            intake
                .blockers
                .iter()
                .filter(|blocker| blocker.code == BlockerCode::MissingAssessment)
                .count(),
            2
        );
    }

    #[test]
    fn a_failed_required_gate_blocks_that_lane() {
        let mut record = complete_record();
        record.assessments.insert(
            GateKey::SpeciesIdentity,
            GateAssessment::Fail {
                evidence: evidence_for(GateKey::SpeciesIdentity),
            },
        );
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(!decision.requirement_set_satisfied);
        assert!(decision_has_code(&decision, BlockerCode::GateFailed));
    }

    #[test]
    fn invalid_digest_shape_blocks_a_nominal_pass() {
        let mut record = complete_record();
        let mut evidence = evidence_for(GateKey::SpeciesIdentity);
        evidence.sha256 = "not-a-digest".to_owned();
        record.assessments.insert(
            GateKey::SpeciesIdentity,
            GateAssessment::Pass { evidence },
        );
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(decision_has_code(&decision, BlockerCode::InvalidEvidence));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn uppercase_digest_is_rejected_to_keep_canonical_form() {
        let mut evidence = evidence_for(GateKey::SpeciesIdentity);
        evidence.sha256 = "A".repeat(64);
        assert!(evidence.validation_issues().iter().any(|issue| issue.contains("sha256")));
    }

    #[test]
    fn invalid_calendar_date_is_rejected() {
        let mut evidence = evidence_for(GateKey::SpeciesIdentity);
        evidence.reviewed_at = "2026-02-29T03:04:05Z".to_owned();
        assert!(evidence.validation_issues().iter().any(|issue| issue.contains("reviewed_at")));
    }

    #[test]
    fn leap_day_is_valid_in_a_leap_year() {
        let mut evidence = evidence_for(GateKey::SpeciesIdentity);
        evidence.reviewed_at = "2024-02-29T03:04:05Z".to_owned();
        assert!(evidence.validation_issues().is_empty());
    }

    #[test]
    fn blank_reviewer_id_is_rejected() {
        let mut evidence = evidence_for(GateKey::SpeciesIdentity);
        evidence.reviewer_id = "   ".to_owned();
        assert!(evidence.validation_issues().iter().any(|issue| issue.contains("reviewer_id")));
    }

    #[test]
    fn blank_scope_is_rejected() {
        let mut evidence = evidence_for(GateKey::SpeciesIdentity);
        evidence.scope.jurisdiction.clear();
        assert!(evidence.validation_issues().iter().any(|issue| issue.contains("scope")));
    }

    #[test]
    fn knowledge_rights_may_be_not_applicable_only_with_reviewable_evidence() {
        let mut record = complete_record();
        record.assessments.insert(
            GateKey::KnowledgeRightsAndBenefitSharing,
            GateAssessment::NotApplicable {
                evidence: evidence_for(GateKey::KnowledgeRightsAndBenefitSharing),
                rationale: "Synthetic fixture: reviewer recorded the basis for non-applicability."
                    .to_owned(),
            },
        );
        assert!(record
            .evaluate(EligibilityTarget::TraceableBotanicalSupply)
            .requirement_set_satisfied);
    }

    #[test]
    fn empty_not_applicable_rationale_blocks() {
        let mut record = complete_record();
        record.assessments.insert(
            GateKey::KnowledgeRightsAndBenefitSharing,
            GateAssessment::NotApplicable {
                evidence: evidence_for(GateKey::KnowledgeRightsAndBenefitSharing),
                rationale: "  ".to_owned(),
            },
        );
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(decision_has_code(&decision, BlockerCode::EmptyNotApplicableRationale));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn species_identity_cannot_be_waived_as_not_applicable() {
        let mut record = complete_record();
        record.assessments.insert(
            GateKey::SpeciesIdentity,
            GateAssessment::NotApplicable {
                evidence: evidence_for(GateKey::SpeciesIdentity),
                rationale: "Synthetic attempted bypass".to_owned(),
            },
        );
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(decision_has_code(&decision, BlockerCode::NotApplicableNotPermitted));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn evidence_for_the_wrong_gate_is_rejected() {
        let mut record = complete_record();
        if let Some(GateAssessment::Pass { evidence }) =
            record.assessments.get_mut(&GateKey::SpeciesIdentity)
        {
            evidence.scope.gate = GateKey::LawfulPropagation;
        }
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(decision_has_code(&decision, BlockerCode::InvalidEvidence));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn evidence_for_another_batch_is_rejected() {
        let mut record = complete_record();
        if let Some(GateAssessment::Pass { evidence }) =
            record.assessments.get_mut(&GateKey::SpeciesIdentity)
        {
            evidence.scope.subject_id = "SYNTHETIC-OTHER-BATCH".to_owned();
        }
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(decision_has_code(&decision, BlockerCode::InvalidEvidence));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn evidence_from_another_jurisdiction_is_rejected() {
        let mut record = complete_record();
        if let Some(GateAssessment::Pass { evidence }) =
            record.assessments.get_mut(&GateKey::SpeciesIdentity)
        {
            evidence.scope.jurisdiction = "XX".to_owned();
        }
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(decision_has_code(&decision, BlockerCode::InvalidEvidence));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn product_gate_evidence_must_reference_product_batch_not_raw_batch() {
        let mut record = complete_record();
        if let Some(GateAssessment::Pass { evidence }) =
            record.assessments.get_mut(&GateKey::ProductAuthorization)
        {
            evidence.scope.subject_id = "SYNTHETIC-BATCH-001".to_owned();
        }
        assert!(!record
            .evaluate(EligibilityTarget::FinishedProductRelease)
            .requirement_set_satisfied);
    }

    #[test]
    fn empty_batch_identifier_blocks_even_with_all_gates_passed() {
        let mut record = complete_record();
        record.batch_id = "  ".to_owned();
        let decision = record.evaluate(EligibilityTarget::TraceableBotanicalSupply);
        assert!(decision_has_code(&decision, BlockerCode::EmptyBatchId));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn finished_product_requires_its_own_batch_identifier() {
        let mut record = complete_record();
        record.finished_product_batch_id = None;
        assert!(record
            .evaluate(EligibilityTarget::TraceableBotanicalSupply)
            .requirement_set_satisfied);
        let decision = record.evaluate(EligibilityTarget::FinishedProductRelease);
        assert!(decision_has_code(&decision, BlockerCode::MissingFinishedProductBatchId));
        assert!(!decision.requirement_set_satisfied);
    }

    #[test]
    fn failed_product_test_blocks_release_but_not_botanical_traceability() {
        let mut record = complete_record();
        record.assessments.insert(
            GateKey::FinishedProductQualityTesting,
            GateAssessment::Fail {
                evidence: evidence_for(GateKey::FinishedProductQualityTesting),
            },
        );
        assert!(record
            .evaluate(EligibilityTarget::TraceableBotanicalSupply)
            .requirement_set_satisfied);
        assert!(!record
            .evaluate(EligibilityTarget::FinishedProductRelease)
            .requirement_set_satisfied);
    }

    #[test]
    fn missing_manufacturing_authorization_blocks_intake_independent_of_garden_gates() {
        let mut record = complete_record();
        record.assessments.remove(&GateKey::ManufacturingAuthorization);
        assert!(record
            .evaluate(EligibilityTarget::AuthorizedManufacturingIntake)
            .requirement_set_satisfied);
        assert!(!record
            .evaluate(EligibilityTarget::FinishedProductRelease)
            .requirement_set_satisfied);
    }

    #[test]
    fn utc_offset_form_is_rejected_until_normalized_to_utc() {
        let mut evidence = evidence_for(GateKey::SpeciesIdentity);
        evidence.reviewed_at = "2026-01-02T05:04:05+02:00".to_owned();
        assert!(evidence.validation_issues().iter().any(|issue| issue.contains("reviewed_at")));
    }

    #[test]
    fn blocker_order_is_deterministic() {
        let record = BotanicalBatchRecord {
            batch_id: "SYNTHETIC-BATCH-001".to_owned(),
            jurisdiction: "ZA".to_owned(),
            assessments: BTreeMap::new(),
            finished_product_batch_id: None,
        };
        let one = record.evaluate(EligibilityTarget::FinishedProductRelease);
        let two = record.evaluate(EligibilityTarget::FinishedProductRelease);
        assert_eq!(one, two);
        assert_eq!(
            one.blockers.first().map(|blocker| blocker.code),
            Some(BlockerCode::MissingFinishedProductBatchId)
        );
    }
}
