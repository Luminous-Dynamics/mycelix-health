// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conductor qualification for DNA-rooted population-accountant state.
//!
//! Unlike older Sweettest files in this repository, this target is intentionally
//! NOT ignored. The dedicated qualification workflow must build/pack `dna/health.dna`
//! first and then execute this test against a real Holochain conductor.

use anyhow::{Context, Result};
use holo_hash::ActionHash;
use holochain::conductor::config::ConductorConfig;
use holochain::conductor::ConductorBuilder;
use holochain::prelude::*;
use mycelix_clinical_integrity::{DigestAlgorithm, DigestDomain, StoredDigest};
use mycelix_clinical_population_accountant_canonical_state::{
    reduce_canonical_population_accountant_snapshot,
    BoundedCanonicalPopulationAccountantStateV1,
};
use mycelix_clinical_population_release::{
    PopulationReleaseArtifactKindV1, PopulationReleaseDigestV1, PrivacyLossV1,
};
use population_accountant_index::CanonicalPopulationAccountantLineageSnapshotV1;
use population_accountant_index_integrity::PopulationAccountantLineageAnchorV1;
use population_accountant_integrity::{
    AccountantCommitmentSchemeV1, OpaqueAccountantReceiptCommitmentV1,
    PopulationAccountantStateProjectionV1, PopulationAccountantVerifierAuthorization,
    TrustedPopulationAccountantState,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn dna_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dna/health.dna")
}

fn stored(seed: u8) -> StoredDigest {
    StoredDigest {
        algorithm: DigestAlgorithm::Blake3_256,
        domain: DigestDomain::ClinicalArtifact,
        value: [seed; 32],
    }
}

fn policy() -> PopulationReleaseDigestV1 {
    PopulationReleaseDigestV1 {
        kind: PopulationReleaseArtifactKindV1::ReleasePolicy,
        value: [9; 32],
    }
}

fn now_micros() -> Result<i64> {
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_micros();
    i64::try_from(micros).context("system time exceeded i64 microseconds")
}

fn genesis_projection(seed: u8) -> PopulationAccountantStateProjectionV1 {
    PopulationAccountantStateProjectionV1 {
        schema_version: 1,
        state_id: format!("qualification-genesis-{seed}"),
        release_policy_digest: policy(),
        accountant_instance_digest: stored(1),
        accountant_method_digest: stored(2),
        sequence: 0,
        query_count: 0,
        cumulative_privacy_loss: PrivacyLossV1::ZERO,
        previous_state_hash: None,
        private_receipt_commitment: OpaqueAccountantReceiptCommitmentV1 {
            scheme: AccountantCommitmentSchemeV1::Blake3Keyed,
            value: [seed; 32],
        },
    }
}

async fn publish_root_authorized_genesis(
    conductor: &holochain::conductor::Conductor,
    root_cell: &CellId,
    root_key: &AgentPubKey,
    seed: u8,
) -> Result<ActionHash> {
    let projection = genesis_projection(seed);
    let now = now_micros()?;
    let authorization = PopulationAccountantVerifierAuthorization {
        authorization_id: format!("qualification-authorization-{seed}"),
        grantee: root_key.clone(),
        public_state_digest: projection.digest()?,
        release_policy_digest: projection.release_policy_digest,
        accountant_instance_digest: projection.accountant_instance_digest,
        accountant_method_digest: projection.accountant_method_digest,
        sequence: projection.sequence,
        previous_state_hash: None,
        private_receipt_commitment: projection.private_receipt_commitment,
        valid_from: Timestamp::from_micros(now - 5_000_000),
        valid_until: Timestamp::from_micros(now + 60_000_000),
    };

    let authorization_record: Record = conductor
        .call_zome(
            root_cell,
            "population_accountant",
            "create_population_accountant_verifier_authorization",
            authorization,
        )
        .await?;

    let state = TrustedPopulationAccountantState {
        projection,
        verifier_authorization_hash: authorization_record.action_address().clone(),
    };
    let state_record: Record = conductor
        .call_zome(
            root_cell,
            "population_accountant",
            "publish_trusted_population_accountant_state",
            state,
        )
        .await?;
    Ok(state_record.action_address().clone())
}

async fn setup() -> Result<(
    holochain::conductor::Conductor,
    CellId,
    CellId,
    PopulationAccountantLineageAnchorV1,
)> {
    let conductor = ConductorBuilder::new()
        .config(ConductorConfig::default())
        .build()
        .await?;

    let root_key = conductor
        .keystore()
        .generate_new_sign_keypair_random()
        .await?;
    let attacker_key = conductor
        .keystore()
        .generate_new_sign_keypair_random()
        .await?;

    let properties = serde_json::json!({
        "population_accountant": {
            "schema_version": 1,
            "root_authorities": [root_key.clone()],
            "max_verifier_authorization_duration_micros": 900000000i64
        }
    });
    let property_bytes = serde_json::to_vec(&properties)?;
    let packed = std::fs::read(dna_path()).context(
        "dna/health.dna is missing; the qualification workflow must build and pack the DNA first",
    )?;
    let dna_file = DnaFile::from_file_content(&packed)
        .await?
        .with_properties(SerializedBytes::from(UnsafeBytes::from(property_bytes)))
        .await;
    let dna_hash = conductor.register_dna(dna_file).await?;

    let root_cell = conductor
        .install_app(
            "population-accountant-root".to_string(),
            vec![InstalledCell::new(
                CellId::new(dna_hash.clone(), root_key.clone()),
                "health".into(),
            )],
        )
        .await?
        .into_iter()
        .next()
        .context("root app installed without a cell")?
        .into_id();

    let attacker_cell = conductor
        .install_app(
            "population-accountant-attacker".to_string(),
            vec![InstalledCell::new(
                CellId::new(dna_hash, attacker_key),
                "health".into(),
            )],
        )
        .await?
        .into_iter()
        .next()
        .context("attacker app installed without a cell")?
        .into_id();

    let lineage = PopulationAccountantLineageAnchorV1::new(policy(), stored(1), stored(2))?;
    Ok((conductor, root_cell, attacker_cell, lineage))
}

#[tokio::test(flavor = "multi_thread")]
async fn canonical_admission_rejects_third_party_and_surfaces_genesis_fork() -> Result<()> {
    let (conductor, root_cell, attacker_cell, lineage) = setup().await?;
    let root_key = root_cell.agent_pubkey().clone();

    let first = publish_root_authorized_genesis(&conductor, &root_cell, &root_key, 11).await?;
    let second = publish_root_authorized_genesis(&conductor, &root_cell, &root_key, 12).await?;

    let _: ActionHash = conductor
        .call_zome(
            &root_cell,
            "population_accountant_index",
            "admit_population_accountant_state",
            first.clone(),
        )
        .await?;

    let unauthorized: Result<ActionHash, _> = conductor
        .call_zome(
            &attacker_cell,
            "population_accountant_index",
            "admit_population_accountant_state",
            first.clone(),
        )
        .await;
    assert!(
        unauthorized.is_err(),
        "third party must not canonically admit another author's accountant state"
    );

    let _: ActionHash = conductor
        .call_zome(
            &root_cell,
            "population_accountant_index",
            "admit_population_accountant_state",
            second,
        )
        .await?;

    let snapshot: CanonicalPopulationAccountantLineageSnapshotV1 = conductor
        .call_zome(
            &root_cell,
            "population_accountant_index",
            "materialize_canonical_population_accountant_lineage",
            lineage,
        )
        .await?;
    assert_eq!(snapshot.observed_state_count, 2);

    let reduced = reduce_canonical_population_accountant_snapshot(&snapshot)?;
    assert!(matches!(
        reduced,
        BoundedCanonicalPopulationAccountantStateV1::ConflictWithinBoundedCanonicalRead { .. }
    ));

    Ok(())
}
