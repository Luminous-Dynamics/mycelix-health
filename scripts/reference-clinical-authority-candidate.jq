# Deterministic benchmark-only policy baseline.
# It consumes the context fields only; expected_decision and expected_reason_code are never read.

def deny($reason): {decision: "deny", reason: $reason};
def defer($reason): {decision: "defer", reason: $reason};
def allow: {decision: "allow", reason: "authorized"};

def decide($case):
  ($case.context) as $ctx
  | ($ctx.source) as $source
  | ($ctx.patient) as $patient
  | ($ctx.consent) as $consent
  | ($ctx.request) as $request
  | ($ctx.registry) as $registry
  | if ($source.registered != true) then deny("source_untrusted")
    elif ($source.signature_valid != true) then deny("signature_invalid")
    elif ($source.payload_digest_matches != true) then deny("payload_digest_mismatch")
    elif ($source.target_version_matches != true) then deny("version_mismatch")
    elif ($ctx.lineage.provenance_bound != true) then deny("provenance_missing")
    elif ($ctx.lineage.provenance_signature_valid != true) then deny("provenance_signature_invalid")
    elif ($ctx.conflict.reconciled != true) then deny("conflict_unresolved")
    elif ($patient.consent_patient_id == null or $patient.consent_patient_id == "") then deny("subject_missing")
    elif ($patient.requested_patient_id != $patient.consent_patient_id) then deny("subject_mismatch")
    elif ($patient.request_tenant_id != $patient.resource_tenant_id) then deny("tenant_mismatch")
    elif ($consent.status != "active") then deny("consent_inactive")
    elif ($consent.revoked == true) then deny("consent_revoked")
    elif ($consent.superseded == true) then deny("consent_superseded")
    elif ($consent.effective_from > $request.as_of_time) then deny("not_yet_effective")
    elif ($consent.effective_to <= $request.as_of_time) then deny("consent_expired")
    elif ($request.resource_version == null or $request.resource_version == "") then deny("resource_version_missing")
    elif ($ctx.lineage.provenance_target_reference !=
          ($request.resource_type + "/" + $request.resource_id + "/_history/" + $request.resource_version)) then
      deny("provenance_target_mismatch")
    elif ($ctx.lineage.provenance_agent_reference != ("Organization/" + $source.issuer_id)
          or $ctx.lineage.provenance_signature_who != $ctx.lineage.provenance_agent_reference) then
      deny("provenance_agent_mismatch")
    elif ($ctx.lineage.provenance_signature_type_system != "urn:iso-astm:E1762-95:2013"
          or $ctx.lineage.provenance_signature_type_code != "1.2.840.10065.1.12.1.14") then
      deny("provenance_signature_purpose_mismatch")
    elif ($source.resource_digest_sha256 == null or $source.resource_digest_sha256 == ""
          or $ctx.lineage.provenance_signed_content_digest_sha256 != $source.resource_digest_sha256) then
      deny("payload_digest_mismatch")
    elif ($source.resource_canonicalization_profile != "FHIR-R4-CANONICAL-XML-v1"
          or $ctx.lineage.provenance_signed_content_canonicalization_profile
             != $source.resource_canonicalization_profile) then
      deny("provenance_canonicalization_mismatch")
    elif ($request.clinical_event_time == null or $request.clinical_event_time == "") then deny("clinical_event_time_missing")
    elif ($request.clinical_event_time < $consent.event_time_from or $request.clinical_event_time > $consent.event_time_to) then deny("event_time_out_of_scope")
    elif (($consent.encounter_ids | index($request.encounter_id)) == null) then
      (if ($request.encounter_id == null or $request.encounter_id == "")
       then deny("encounter_context_missing") else deny("encounter_mismatch") end)
    elif (($consent.permitted_purposes | index($request.purpose)) == null) then deny("purpose_mismatch")
    elif (($consent.permitted_recipients | index($request.recipient_id)) == null) then deny("recipient_mismatch")
    elif (($consent.permitted_actions | index($request.action)) == null) then deny("action_not_permitted")
    elif (($consent.included_categories | index($request.category)) == null) then deny("category_not_permitted")
    elif ($consent.grantee_kind == "provider" and $registry.provider_registered != true) then deny("principal_unregistered")
    elif ($consent.grantee_kind == "provider" and
          ($registry.provider_organization_id != $request.recipient_id or $consent.grantee_id != $request.principal_id)) then deny("grantee_unresolved")
    elif ($consent.grantee_kind == "organization" and
          ($registry.organization_registered != true or $consent.grantee_id != $request.recipient_id)) then deny("grantee_unresolved")
    elif ($consent.grantee_kind != "provider" and $consent.grantee_kind != "organization") then deny("grantee_unresolved")
    elif ($ctx.audit.chain_valid != true) then deny("audit_chain_invalid")
    elif ($ctx.audit.actor_authenticated != true) then deny("audit_actor_unbound")
    elif ($request.replayed == true) then deny("request_replayed")
    elif ($registry.source_registry_available != true) then defer("source_registry_unavailable")
    elif ($registry.source_signing_key_state_fresh != true) then defer("authority_state_stale")
    elif ($registry.source_signing_key_registered != true
          or $registry.active_source_signing_key_id != $ctx.lineage.provenance_signing_key_id
          or $registry.source_signing_key_revoked == true) then deny("source_untrusted")
    elif ($registry.provenance_verifier_available != true) then defer("verifier_unavailable")
    elif ($ctx.lineage.provenance_verification_status == "invalid") then
      deny("provenance_signature_invalid")
    elif ($ctx.lineage.provenance_verification_status != "verified") then
      defer("verifier_unavailable")
    elif ($registry.revocation_state_fresh != true) then defer("authority_state_stale")
    elif ($registry.verifier_available != true) then defer("verifier_unavailable")
    elif ($registry.audit_store_available != true) then defer("audit_unavailable")
    elif ($registry.replay_store_available != true) then defer("replay_store_unavailable")
    else allow
    end;

{
  schema_version: 1,
  benchmark_id: .benchmark_id,
  results: [
    .cases[]
    | . as $case
    | decide($case) as $decision
    | {
        case_id: $case.id,
        decision: $decision.decision,
        reason_codes: [$decision.reason],
        evidence_refs: ["synthetic-context:" + $case.id],
        policy_version: "mycelix-clinical-authority-reference-v1"
      }
  ]
}
