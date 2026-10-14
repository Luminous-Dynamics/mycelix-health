// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic walkthrough only. No real plant, source, licence, lab, or product data.

use medicine_bio_domain::{
    BotanicalBatchRecord, EligibilityTarget, EvidenceRef, GateAssessment, GateKey,
};
use std::collections::BTreeMap;

fn main() {
    let gates = [
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

    let assessments = gates
        .into_iter()
        .map(|gate| {
            let evidence = EvidenceRef {
                evidence_id: format!("SYNTHETIC-ONLY-{gate:?}"),
                sha256: "a".repeat(64),
                reviewer_id: "synthetic-fixture-reviewer".to_owned(),
                reviewed_at: "2026-01-02T03:04:05Z".to_owned(),
                scope: format!("SYNTHETIC-ONLY:BATCH-DEMO:{gate:?}:ZA"),
            };
            (gate, GateAssessment::Pass { evidence })
        })
        .collect();

    let batch = BotanicalBatchRecord {
        batch_id: "SYNTHETIC-ONLY-BATCH-DEMO".to_owned(),
        assessments,
        finished_product_batch_id: Some("SYNTHETIC-ONLY-PRODUCT-DEMO".to_owned()),
    };

    for target in [
        EligibilityTarget::TraceableBotanicalSupply,
        EligibilityTarget::AuthorizedManufacturingIntake,
        EligibilityTarget::FinishedProductRelease,
    ] {
        let decision = batch.evaluate(target);
        println!(
            "{target:?}: checklist_satisfied={}, blockers={}",
            decision.requirement_set_satisfied,
            decision.blockers.len()
        );
    }

    // Negative control: a successful-looking fixture cannot override a failed gate.
    let mut negative_control = batch.clone();
    negative_control.assessments.insert(
        GateKey::BiodiversityProtection,
        GateAssessment::Fail {
            evidence: EvidenceRef {
                evidence_id: "SYNTHETIC-ONLY-FAILED-BIODIVERSITY-REVIEW".to_owned(),
                sha256: "b".repeat(64),
                reviewer_id: "synthetic-fixture-reviewer".to_owned(),
                reviewed_at: "2026-01-02T03:04:05Z".to_owned(),
                scope: "SYNTHETIC-ONLY:BATCH-DEMO:biodiversity:ZA".to_owned(),
            },
        },
    );
    let blocked = negative_control.evaluate(EligibilityTarget::TraceableBotanicalSupply);
    println!(
        "negative_control: checklist_satisfied={}, blockers={}",
        blocked.requirement_set_satisfied,
        blocked.blockers.len()
    );
}
