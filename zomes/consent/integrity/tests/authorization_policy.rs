// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use consent_integrity::{
    consent_matches_authorization, Consent, ConsentGrantee, ConsentPurpose, ConsentScope,
    ConsentStatus, DataCategory, DataPermission,
};
use hdi::prelude::{ActionHash, AgentPubKey, Timestamp};

fn agent(byte: u8) -> AgentPubKey {
    AgentPubKey::from_raw_36(vec![byte; 36])
}

fn consent(
    grantee: ConsentGrantee,
    categories: Vec<DataCategory>,
    permissions: Vec<DataPermission>,
) -> Consent {
    Consent {
        consent_id: "synthetic-consent-001".to_string(),
        patient_hash: ActionHash::from_raw_36(vec![7; 36]),
        grantee,
        scope: ConsentScope {
            data_categories: categories,
            date_range: None,
            encounter_hashes: None,
            exclusions: vec![],
        },
        permissions,
        purpose: ConsentPurpose::Treatment,
        status: ConsentStatus::Active,
        granted_at: Timestamp::from_micros(1),
        expires_at: None,
        revoked_at: None,
        revocation_reason: None,
        document_hash: None,
        witness: None,
        legal_representative: None,
        notes: None,
    }
}

fn allows(
    value: &Consent,
    requestor: &AgentPubKey,
    category: &DataCategory,
    permission: &DataPermission,
    is_emergency: bool,
    now: i64,
) -> bool {
    consent_matches_authorization(
        value,
        requestor,
        category,
        permission,
        is_emergency,
        Timestamp::from_micros(now),
    )
}

#[test]
fn exact_active_agent_scope_and_permission_are_allowed() {
    let requestor = agent(1);
    let value = consent(
        ConsentGrantee::Agent(requestor.clone()),
        vec![DataCategory::Medications],
        vec![DataPermission::Read],
    );

    assert!(allows(
        &value,
        &requestor,
        &DataCategory::Medications,
        &DataPermission::Read,
        false,
        10
    ));
}

#[test]
fn wrong_agent_is_denied() {
    let value = consent(
        ConsentGrantee::Agent(agent(1)),
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );

    assert!(!allows(
        &value,
        &agent(2),
        &DataCategory::Demographics,
        &DataPermission::Read,
        false,
        10
    ));
}

#[test]
fn expiry_boundary_is_strict_and_matches_get_active_consents() {
    let requestor = agent(1);
    let mut value = consent(
        ConsentGrantee::Agent(requestor.clone()),
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );
    value.expires_at = Some(Timestamp::from_micros(100));

    assert!(allows(
        &value,
        &requestor,
        &DataCategory::Demographics,
        &DataPermission::Read,
        false,
        99
    ));
    assert!(!allows(
        &value,
        &requestor,
        &DataCategory::Demographics,
        &DataPermission::Read,
        false,
        100
    ));
    assert!(!allows(
        &value,
        &requestor,
        &DataCategory::Demographics,
        &DataPermission::Read,
        false,
        101
    ));
}

#[test]
fn revoked_and_non_active_consent_are_denied() {
    let requestor = agent(1);
    let mut value = consent(
        ConsentGrantee::Agent(requestor.clone()),
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );
    value.revoked_at = Some(Timestamp::from_micros(9));
    assert!(!allows(
        &value,
        &requestor,
        &DataCategory::All,
        &DataPermission::Read,
        false,
        10
    ));

    value.revoked_at = None;
    value.status = ConsentStatus::Pending;
    assert!(!allows(
        &value,
        &requestor,
        &DataCategory::All,
        &DataPermission::Read,
        false,
        10
    ));
}

#[test]
fn category_mismatch_and_explicit_exclusion_are_denied() {
    let requestor = agent(1);
    let value = consent(
        ConsentGrantee::Agent(requestor.clone()),
        vec![DataCategory::Demographics],
        vec![DataPermission::Read],
    );
    assert!(!allows(
        &value,
        &requestor,
        &DataCategory::Medications,
        &DataPermission::Read,
        false,
        10
    ));

    let mut broad = consent(
        ConsentGrantee::Agent(requestor.clone()),
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );
    broad.scope.exclusions = vec![DataCategory::MentalHealth];
    assert!(!allows(
        &broad,
        &requestor,
        &DataCategory::MentalHealth,
        &DataPermission::Read,
        false,
        10
    ));
}

#[test]
fn permission_mismatch_is_denied() {
    let requestor = agent(1);
    let value = consent(
        ConsentGrantee::Agent(requestor.clone()),
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );

    assert!(!allows(
        &value,
        &requestor,
        &DataCategory::Demographics,
        &DataPermission::Write,
        false,
        10
    ));
}

#[test]
fn public_consent_preserves_existing_production_behavior() {
    let requestor = agent(1);
    let value = consent(
        ConsentGrantee::Public,
        vec![DataCategory::Demographics],
        vec![DataPermission::Read],
    );

    assert!(allows(
        &value,
        &requestor,
        &DataCategory::Demographics,
        &DataPermission::Read,
        false,
        10
    ));
}

#[test]
fn unresolved_provider_and_organization_grantees_fail_closed() {
    let requestor = agent(1);
    let provider = consent(
        ConsentGrantee::Provider(ActionHash::from_raw_36(vec![8; 36])),
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );
    let organization = consent(
        ConsentGrantee::Organization("synthetic-org-01".to_string()),
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );

    for value in [&provider, &organization] {
        assert!(!allows(
            value,
            &requestor,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            10
        ));
    }
}

#[test]
fn legacy_emergency_grantee_never_authorizes_without_capability_lookup() {
    let requestor = agent(1);
    let value = consent(
        ConsentGrantee::EmergencyAccess,
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );

    assert!(!allows(
        &value,
        &requestor,
        &DataCategory::Demographics,
        &DataPermission::Read,
        true,
        10
    ));
}

#[test]
fn legal_representative_restriction_matches_current_sensitive_category_behavior() {
    let requestor = agent(1);
    let mut value = consent(
        ConsentGrantee::Public,
        vec![DataCategory::All],
        vec![DataPermission::Read],
    );
    value.legal_representative = Some(agent(2));

    // Current production coordinator applies this guardian restriction to
    // SubstanceAbuse and SexualHealth when the request is not emergency.
    for category in [DataCategory::SubstanceAbuse, DataCategory::SexualHealth] {
        assert!(!allows(
            &value,
            &requestor,
            &category,
            &DataPermission::Read,
            false,
            10
        ));
        assert!(allows(
            &value,
            &requestor,
            &category,
            &DataPermission::Read,
            true,
            10
        ));
    }

    // Current production code does not apply this guardian gate to MentalHealth.
    assert!(allows(
        &value,
        &requestor,
        &DataCategory::MentalHealth,
        &DataPermission::Read,
        false,
        10
    ));
}
