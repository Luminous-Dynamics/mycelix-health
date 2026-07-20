#!/usr/bin/env python3
"""Gate the complete clinical promotion contract without requiring live release tools."""
from __future__ import annotations
import hashlib, importlib.util, json, os, pathlib, py_compile, subprocess, tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
def fail(m:str): raise SystemExit(f"clinical promotion contract failed: {m}")
def run(*args:str,expected:int=0):
    r=subprocess.run(args,cwd=ROOT,text=True,capture_output=True)
    if r.returncode!=expected: fail(f"{' '.join(args)} returned {r.returncode}: {r.stderr or r.stdout}")
    return r
def write(path:pathlib.Path,value:dict): path.parent.mkdir(parents=True,exist_ok=True); path.write_text(json.dumps(value,indent=2,sort_keys=True)+"\n"); os.chmod(path,0o600)
def sha(path:pathlib.Path): return hashlib.sha256(path.read_bytes()).hexdigest()
def self_test():
    spec=importlib.util.spec_from_file_location("ev",ROOT/"scripts/release-evidence.py"); assert spec and spec.loader; ev=importlib.util.module_from_spec(spec); spec.loader.exec_module(ev)
    repro_policy=json.loads((ROOT/"release/reproducibility-policy.json").read_text())
    with tempfile.TemporaryDirectory() as raw:
        root=pathlib.Path(raw); first=root/"a"; second=root/"b"; source="1"*40
        for d in (first,second):
            for rel in repro_policy["required_artifacts"]: p=d/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(("artifact:"+rel).encode())
        for d in (root/"ca",root/"cb"): d.mkdir(); write(d/"build-context.json",{"source_revision":source}); write(d/"nix-closure.json",{"store_paths":[]})
        repro=root/"repro.json"; run("python3","scripts/compare-release-builds.py","--first",str(first),"--second",str(second),"--first-context",str(root/"ca"),"--second-context",str(root/"cb"),"--output",str(repro))
        repro_prov=root/"repro-prov.json"; write(repro_prov,{"schema_version":2,"status":"verified","source_revision":source})
        evidence={"evidence_version":ev.EVIDENCE_VERSION,"release_id":"health-v1","wire_schema_version":ev.WIRE_SCHEMA_VERSION,"source_manifest_sha256":list(bytes.fromhex(ev.EXPECTED_SOURCE_MANIFEST_SHA256)),"supply_chain_policy_sha256":list(bytes.fromhex(ev.EXPECTED_SUPPLY_CHAIN_POLICY_SHA256)),"sbom_sha256":list(bytes.fromhex(ev.EXPECTED_SBOM_SHA256)),"schema_migration_epoch":ev.SCHEMA_MIGRATION_EPOCH,"source_revision":source,"built_at_utc":"2026-07-20T00:00:00Z","dna_hash":"uhC0k-self-test","dna_bundle_sha256":list(bytes.fromhex(sha(first/"health.dna"))),"zomes":[]}
        for name in ev.EXPECTED_COORDINATORS:
            file="health_bridge" if name=="bridge" else name
            evidence["zomes"].append({"coordinator_name":name,"integrity_name":name+"_integrity","coordinator_wasm_sha256":list(bytes.fromhex(sha(first/f"wasm/{file}.wasm"))),"integrity_wasm_sha256":list(bytes.fromhex(sha(first/f"wasm/{name}_integrity.wasm")))})
        payload=ev.canonical_bytes(evidence); private=root/"private.pem"; public=root/"public.der"; msg=root/"msg"; sig=root/"sig"; msg.write_bytes(payload)
        run("openssl","genpkey","-algorithm","ED25519","-out",str(private)); run("openssl","pkey","-in",str(private),"-pubout","-outform","DER","-out",str(public)); run("openssl","pkeyutl","-sign","-rawin","-inkey",str(private),"-in",str(msg),"-out",str(sig))
        bundle=root/"bundle"; write(bundle/"health-v1.signed-evidence.json",{"evidence":evidence,"signer_key_id":"test","signature":list(sig.read_bytes())}); write(bundle/"trusted-release-signers.json",{"signers":[{"signer_key_id":"test","status":"active","ed25519_public_key":list(public.read_bytes()[-32:])}]})
        compat=root/"compat.json"; report=json.loads(subprocess.run(["python3","scripts/check-release-compatibility.py"],cwd=ROOT,text=True,capture_output=True,check=True).stdout); report["signed_evidence_sha256"]=sha(bundle/"health-v1.signed-evidence.json"); write(compat,report)
        supply=root/"supply.json"; policy=json.loads((ROOT/"release/supply-chain-policy.json").read_text()); materials={name:{"sha256":sha(path)} for name,path in {"supply_chain_policy":ROOT/"release/supply-chain-policy.json","sbom":ROOT/"release/health-v1.sbom.cdx.json","github_actions_lock":ROOT/"release/github-actions-lock.json"}.items()}; write(supply,{"status":"verified","source_revision":source,"materials":materials,"reviewed_exceptions":policy["exceptions"]})
        sdk=root/"sdk.tgz"; sdk.write_bytes(b"sdk"); subjects=root/"subjects.json"; subject_value={"source_revision":source,"subjects":[{"name":"health-happ","filename":"mycelix-health.happ","sha256":sha(first/"mycelix-health.happ"),"size_bytes":(first/"mycelix-health.happ").stat().st_size},{"name":"health-sdk","filename":"sdk.tgz","sha256":sha(sdk),"size_bytes":3}]}; write(subjects,subject_value)
        attest=root/"attest.json"; write(attest,{"status":"verified","source_revision":source,"subjects":[dict(x,verification_sha256="2"*64) for x in subject_value["subjects"]]})
        suite=root/"suite"; write(suite/"SUITE-LEDGER.json",{"source_revision":source,"clinical_release_eligible":True,"fault_matrix_complete":False}); write(suite/"SUITE-MANIFEST.json",{"clinical_release_eligible":True})
        out=root/"out"; cmd=["python3","scripts/promote-clinical-release.py","--release-bundle",str(bundle),"--compatibility-report",str(compat),"--supply-chain-report",str(supply),"--reproducibility-report",str(repro),"--reproducibility-provenance",str(repro_prov),"--attestation-subjects",str(subjects),"--attestation-report",str(attest),"--suite-root",str(suite),"--output-dir",str(out)]; run(*cmd)
        if json.loads((out/"CLINICAL-PROMOTION-DECISION.json").read_text()).get("status")!="promoted": fail("verified self-test was not promoted")
        bad=json.loads(supply.read_text()); bad["materials"]["sbom"]["sha256"]="0"*64; badpath=root/"bad.json"; write(badpath,bad); refused=root/"refused"; badcmd=cmd.copy(); badcmd[badcmd.index(str(supply))]=str(badpath); badcmd[-1]=str(refused); run(*badcmd,expected=1)
        if refused.exists(): fail("refused promotion created partial output")
    print("clinical promotion integration self-test: ok")
def main():
    policy=json.loads((ROOT/"release/clinical-promotion-policy.json").read_text())
    if policy.get("schema_version")!=2 or policy.get("requirements",{}).get("require_single_source_revision") is not True: fail("clinical promotion policy is not fail-closed v2")
    for rel in ("scripts/build-clinical-supply-chain-report.py","scripts/verify-release-attestations.py","scripts/promote-clinical-release.py","scripts/check-clinical-promotion.py"):
        py_compile.compile(str(ROOT/rel),doraise=True)
    run("python3","scripts/build-clinical-supply-chain-report.py","--self-test"); run("python3","scripts/verify-release-attestations.py","--self-test"); run("python3","scripts/check-release-compatibility.py"); self_test(); print("clinical promotion contract: ok")
if __name__=="__main__": main()
