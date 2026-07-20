#!/usr/bin/env python3
"""Create one fail-closed, create-only Mycelix Health clinical promotion decision."""
from __future__ import annotations
import argparse, hashlib, importlib.util, json, os, pathlib, subprocess, tempfile
from typing import Any, NoReturn
ROOT=pathlib.Path(__file__).resolve().parents[1]
POLICY=ROOT/"release/clinical-promotion-policy.json"
SPEC=importlib.util.spec_from_file_location("health_release_evidence",ROOT/"scripts/release-evidence.py")
assert SPEC and SPEC.loader
release_evidence=importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(release_evidence)
DIAGNOSTICS_SPEC=importlib.util.spec_from_file_location("health_promotion_diagnostics",ROOT/"scripts/clinical-promotion-diagnostics.py")
assert DIAGNOSTICS_SPEC and DIAGNOSTICS_SPEC.loader
promotion_diagnostics=importlib.util.module_from_spec(DIAGNOSTICS_SPEC); DIAGNOSTICS_SPEC.loader.exec_module(promotion_diagnostics)

class PromotionError(ValueError):
    def __init__(self,message:str,*,code:str|None=None,stage:str|None=None):
        self.code=code
        self.stage=stage
        super().__init__(message)

def classify_failure(message:str)->tuple[str,str]:
    value=message.lower()
    if "overwrite" in value or "output" in value and "exists" in value: return "PROMOTION_OUTPUT_EXISTS","promotion"
    if "placeholder" in value or "unresolved" in value: return "UNRESOLVED_PLACEHOLDER","release_evidence"
    if "release signer" in value or "signature" in value or "release evidence" in value or "release id" in value: return "PROMOTION_POLICY_REFUSAL","release_evidence"
    if "compatibility report" in value or "migration" in value: return "REPORT_NOT_VERIFIED","compatibility"
    if "supply-chain" in value or "sbom" in value or "github_actions_lock" in value or "github actions lock" in value: return "CURRENT_MATERIAL_DIGEST_MISMATCH","supply_chain"
    if "reproducibility" in value or "non-identical" in value or "nix context" in value: return "ARTIFACT_DIGEST_MISMATCH","reproducibility"
    if "attestation" in value or "attested" in value: return "ARTIFACT_DIGEST_MISMATCH","attestation"
    if "empirical" in value or "fault matrix" in value: return "EMPIRICAL_SUITE_INELIGIBLE","empirical_suite"
    if "source revision" in value: return "SOURCE_REVISION_MISMATCH","source_coherence"
    if "digest differs" in value or "wasm differs" in value or "dna digest" in value: return "ARTIFACT_DIGEST_MISMATCH","reproducibility"
    if "not verified" in value: return "REPORT_NOT_VERIFIED","promotion"
    return "PROMOTION_POLICY_REFUSAL","promotion"

def fail(message:str,*,code:str|None=None,stage:str|None=None)->NoReturn:
    classified_code,classified_stage=classify_failure(message)
    raise PromotionError(message,code=code or classified_code,stage=stage or classified_stage)
def load(path:pathlib.Path)->dict[str,Any]:
    value=json.loads(path.read_text())
    if not isinstance(value,dict): fail(f"{path} must contain an object")
    return value
def sha256(path:pathlib.Path)->str: return hashlib.sha256(path.read_bytes()).hexdigest()
def canonical(value:Any)->bytes: return json.dumps(value,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()
def hex32(value:Any,label:str)->str:
    try: raw=bytes(value)
    except (TypeError,ValueError) as e: raise PromotionError(f"{label} must be bytes") from e
    if len(raw)!=32: fail(f"{label} must contain 32 bytes")
    return raw.hex()
def verify_signature(public:bytes,payload:bytes,signature:bytes)->None:
    prefix=bytes.fromhex("302a300506032b6570032100")
    with tempfile.TemporaryDirectory() as raw:
        root=pathlib.Path(raw); (root/"p.der").write_bytes(prefix+public); (root/"m").write_bytes(payload); (root/"s").write_bytes(signature)
        r=subprocess.run(["openssl","pkeyutl","-verify","-rawin","-pubin","-keyform","DER","-inkey",str(root/"p.der"),"-in",str(root/"m"),"-sigfile",str(root/"s")],capture_output=True,text=True)
        if r.returncode: fail("release evidence Ed25519 signature is invalid")
def verify_release_bundle(root:pathlib.Path,policy:dict[str,Any])->tuple[dict[str,Any],dict[str,str]]:
    signed_path=root/"health-v1.signed-evidence.json"; trust_path=root/"trusted-release-signers.json"
    signed=load(signed_path); trust=load(trust_path); evidence=signed.get("evidence")
    if not isinstance(evidence,dict): fail("signed release bundle lacks evidence")
    release_evidence.validate_evidence(evidence)
    if evidence.get("release_id")!=policy.get("release_id"): fail("release ID differs from promotion policy")
    if evidence.get("source_revision") in policy.get("forbidden_source_placeholders",[]) or evidence.get("dna_hash") in policy.get("forbidden_source_placeholders",[]): fail("release bundle contains an unresolved placeholder")
    key_id=signed.get("signer_key_id"); matches=[x for x in trust.get("signers",[]) if isinstance(x,dict) and x.get("signer_key_id")==key_id and x.get("status")=="active"]
    if len(matches)!=1: fail("release signer must appear exactly once and be active")
    public=bytes(matches[0].get("ed25519_public_key",[])); signature=release_evidence.signature64(signed.get("signature")); payload=release_evidence.canonical_bytes(evidence)
    if len(public)!=32 or not any(public): fail("release signer public key is invalid")
    verify_signature(public,payload,signature)
    return evidence,{"signed_evidence_sha256":sha256(signed_path),"trusted_signers_sha256":sha256(trust_path),"evidence_sha256":hashlib.sha256(payload).hexdigest()}
def require_verified(value:dict[str,Any],label:str)->None:
    if value.get("status")!="verified": fail(f"{label} is not verified")
def comparisons(report:dict[str,Any])->dict[str,str]:
    result={}
    for item in report.get("comparisons",[]):
        if item.get("byte_identical") is not True: fail("reproducibility report contains a non-identical artifact")
        result[str(item["artifact"])]=str(item["first_sha256"])
    if not result: fail("reproducibility report has no artifacts")
    if report.get("context_mismatches"): fail("reproducibility context differs")
    return result
def verify_artifacts(evidence:dict[str,Any],repro:dict[str,Any],subjects:dict[str,Any],attestation:dict[str,Any])->None:
    hashes=comparisons(repro)
    if hashes.get("health.dna")!=hex32(evidence.get("dna_bundle_sha256"),"DNA digest"): fail("signed DNA digest differs from reproducible build")
    for zome in evidence.get("zomes",[]):
        coordinator="health_bridge" if zome["coordinator_name"]=="bridge" else zome["coordinator_name"]
        if hashes.get(f"wasm/{coordinator}.wasm")!=hex32(zome["coordinator_wasm_sha256"],"coordinator digest"): fail(f"coordinator WASM differs: {coordinator}")
        if hashes.get(f"wasm/{zome['integrity_name']}.wasm")!=hex32(zome["integrity_wasm_sha256"],"integrity digest"): fail(f"integrity WASM differs: {zome['integrity_name']}")
    subject_map={x["name"]:x for x in subjects.get("subjects",[])}; verified_map={x["name"]:x for x in attestation.get("subjects",[])}
    if set(subject_map)!={"health-happ","health-sdk"} or set(verified_map)!=set(subject_map): fail("attested subject set is incomplete")
    if subject_map["health-happ"]["sha256"]!=hashes.get("mycelix-health.happ"): fail("attested hApp differs from reproducible hApp")
    for name,item in subject_map.items():
        if verified_map[name]["sha256"]!=item["sha256"] or verified_map[name]["size_bytes"]!=item["size_bytes"]: fail(f"attestation report differs for {name}")
def promote(a:argparse.Namespace)->dict[str,Any]:
    policy=load(a.policy.resolve()); evidence,release_hashes=verify_release_bundle(a.release_bundle.resolve(),policy)
    compatibility=load(a.compatibility_report.resolve()); supply=load(a.supply_chain_report.resolve()); repro=load(a.reproducibility_report.resolve()); repro_prov=load(a.reproducibility_provenance.resolve()); subjects=load(a.attestation_subjects.resolve()); attest=load(a.attestation_report.resolve())
    suite=a.suite_root.resolve(); ledger=load(suite/"SUITE-LEDGER.json"); manifest=load(suite/"SUITE-MANIFEST.json")
    for value,label in ((compatibility,"compatibility report"),(supply,"supply-chain report"),(repro,"reproducibility report"),(repro_prov,"reproducibility provenance"),(attest,"GitHub attestation report")): require_verified(value,label)
    if ledger.get("clinical_release_eligible") is not True or manifest.get("clinical_release_eligible") is not True: fail("empirical suite is not clinically release eligible")
    if policy["requirements"].get("require_fault_matrix_complete") and ledger.get("fault_matrix_complete") is not True: fail("fault matrix is incomplete")
    revision=evidence["source_revision"]
    revisions=[supply.get("source_revision"),repro_prov.get("source_revision"),subjects.get("source_revision"),attest.get("source_revision"),ledger.get("source_revision")]
    if any(x!=revision for x in revisions): fail("promotion inputs do not share one source revision")
    if compatibility.get("signed_evidence_sha256")!=release_hashes["signed_evidence_sha256"]: fail("compatibility report targets different signed evidence")
    current={"supply_chain_policy":sha256(ROOT/"release/supply-chain-policy.json"),"sbom":sha256(ROOT/"release/health-v1.sbom.cdx.json"),"github_actions_lock":sha256(ROOT/"release/github-actions-lock.json")}
    for name,digest in current.items():
        material=supply.get("materials",{}).get(name,{})
        if material.get("sha256")!=digest: fail(f"supply-chain report does not bind current {name}")
    verify_artifacts(evidence,repro,subjects,attest)
    inputs={"promotion_policy_sha256":sha256(a.policy.resolve()),"release_bundle":release_hashes,"compatibility_report_sha256":sha256(a.compatibility_report.resolve()),"supply_chain_report_sha256":sha256(a.supply_chain_report.resolve()),"reproducibility_report_sha256":sha256(a.reproducibility_report.resolve()),"reproducibility_provenance_sha256":sha256(a.reproducibility_provenance.resolve()),"attestation_subjects_sha256":sha256(a.attestation_subjects.resolve()),"attestation_report_sha256":sha256(a.attestation_report.resolve()),"suite_ledger_sha256":sha256(suite/"SUITE-LEDGER.json"),"suite_manifest_sha256":sha256(suite/"SUITE-MANIFEST.json")}
    identity={"release_id":"health-v1","source_revision":revision,"dna_hash":evidence["dna_hash"],"inputs":inputs}; digest=hashlib.sha256(canonical(identity)).hexdigest()
    return {"schema_version":2,"decision_id":f"health-clinical-promotion-{digest[:24]}","status":"promoted","release_id":"health-v1","source_revision":revision,"dna_hash":evidence["dna_hash"],"decision_digest_sha256":digest,"inputs":inputs,"claims":{"signed_deployment_verified":True,"schema_and_migration_compatible":True,"supply_chain_policy_verified":True,"reviewed_exceptions_are_exact_and_unexpired":True,"reproducible_artifacts_and_nix_context_verified":True,"github_source_and_workflow_provenance_verified":True,"empirical_clinical_happy_paths_verified":True,"github_attestation_is_not_a_security_guarantee":True}}
def write_output(root:pathlib.Path,decision:dict[str,Any])->None:
    if root.exists(): fail(f"refusing to overwrite {root}")
    root.mkdir(parents=True,mode=0o700)
    decision_path=root/"CLINICAL-PROMOTION-DECISION.json"
    with decision_path.open("x") as h: json.dump(decision,h,indent=2,sort_keys=True); h.write("\n")
    os.chmod(decision_path,0o600)
    manifest={"schema_version":1,"status":"verified","decision_id":decision["decision_id"],"decision_sha256":sha256(decision_path),"source_revision":decision["source_revision"]}
    path=root/"MANIFEST.json"
    with path.open("x") as h: json.dump(manifest,h,indent=2,sort_keys=True); h.write("\n")
    os.chmod(path,0o600)
def refusal_inputs(a:argparse.Namespace)->dict[str,pathlib.Path]:
    return {
        "promotion_policy":a.policy,
        "release_bundle":a.release_bundle,
        "compatibility_report":a.compatibility_report,
        "supply_chain_report":a.supply_chain_report,
        "reproducibility_report":a.reproducibility_report,
        "reproducibility_provenance":a.reproducibility_provenance,
        "attestation_subjects":a.attestation_subjects,
        "attestation_report":a.attestation_report,
        "suite_root":a.suite_root,
    }

def refusal_source_revision(a:argparse.Namespace)->str|None:
    try:
        signed=load(a.release_bundle.resolve()/"health-v1.signed-evidence.json")
        evidence=signed.get("evidence")
        value=evidence.get("source_revision") if isinstance(evidence,dict) else None
        return value if isinstance(value,str) else None
    except (OSError,ValueError,TypeError,KeyError,json.JSONDecodeError):
        return None

def build_refusal_report(a:argparse.Namespace,error:Exception)->dict[str,Any]:
    policy=promotion_diagnostics.load_policy()
    code=getattr(error,"code",None)
    stage=getattr(error,"stage",None)
    if code not in policy.get("reason_codes",{}): code="INTERNAL_ERROR"
    if stage not in policy.get("stage_order",[]): stage="promotion"
    item=promotion_diagnostics.reason(policy,code,str(error),stage=stage)
    identity={
        "policy_sha256":sha256(a.policy.resolve()) if a.policy.exists() else None,
        "source_revision":refusal_source_revision(a),
        "reason":item,
        "inputs":promotion_diagnostics.build_input_manifest(refusal_inputs(a)),
    }
    digest=hashlib.sha256(canonical(identity)).hexdigest()
    return {
        "schema_version":1,
        "report_kind":"promotion-refusal",
        "report_id":f"health-promotion-refusal-{digest[:24]}",
        "status":"refused",
        "release_id":"health-v1",
        "source_revision":identity["source_revision"],
        "reason":item,
        "inputs":identity["inputs"],
        "promotion_output_created":a.output_dir.exists(),
        "report_digest_sha256":digest,
    }

def parse_args()->argparse.Namespace:
    p=argparse.ArgumentParser(); p.add_argument("--policy",type=pathlib.Path,default=POLICY); p.add_argument("--release-bundle",type=pathlib.Path,required=True); p.add_argument("--compatibility-report",type=pathlib.Path,required=True); p.add_argument("--supply-chain-report",type=pathlib.Path,required=True); p.add_argument("--reproducibility-report",type=pathlib.Path,required=True); p.add_argument("--reproducibility-provenance",type=pathlib.Path,required=True); p.add_argument("--attestation-subjects",type=pathlib.Path,required=True); p.add_argument("--attestation-report",type=pathlib.Path,required=True); p.add_argument("--suite-root",type=pathlib.Path,required=True); p.add_argument("--output-dir",type=pathlib.Path,required=True); p.add_argument("--refusal-output",type=pathlib.Path); return p.parse_args()

def refusal_path_is_separate(a:argparse.Namespace)->bool:
    if not a.refusal_output: return False
    refusal=a.refusal_output.resolve(); output=a.output_dir.resolve()
    return refusal!=output and output not in refusal.parents

def main()->int:
    a=parse_args()
    try:
        if a.refusal_output and not refusal_path_is_separate(a):
            fail("refusal output must be outside the promotion output directory",code="INPUT_UNSAFE",stage="promotion")
        decision=promote(a); write_output(a.output_dir.resolve(),decision); print(a.output_dir.resolve()); return 0
    except (PromotionError,OSError,KeyError,ValueError,TypeError,json.JSONDecodeError,release_evidence.EvidenceError,promotion_diagnostics.DiagnosticError) as error:
        normalized=error if isinstance(error,PromotionError) else PromotionError(str(error),code="INTERNAL_ERROR",stage="promotion")
        diagnostic_error=None
        if a.refusal_output and refusal_path_is_separate(a):
            try:
                report=build_refusal_report(a,normalized)
                promotion_diagnostics.write_create_only(a.refusal_output.resolve(),report)
                print(a.refusal_output.resolve())
            except (OSError,ValueError,TypeError,KeyError,json.JSONDecodeError,promotion_diagnostics.DiagnosticError) as report_error:
                diagnostic_error=promotion_diagnostics.sanitize_text(report_error,promotion_diagnostics.load_policy())
        print(f"clinical release promotion failed: {normalized}")
        if diagnostic_error: print(f"clinical refusal evidence was not written: {diagnostic_error}")
        return 1

if __name__=="__main__": raise SystemExit(main())
