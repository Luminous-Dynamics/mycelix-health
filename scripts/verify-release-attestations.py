#!/usr/bin/env python3
"""Verify GitHub artifact attestations and normalize a create-only report."""
from __future__ import annotations
import argparse, hashlib, json, os, pathlib, subprocess, tempfile
from typing import Any
class AttestationError(ValueError): pass

def sha256(path:pathlib.Path)->str: return hashlib.sha256(path.read_bytes()).hexdigest()
def load(path:pathlib.Path)->dict[str,Any]:
    value=json.loads(path.read_text())
    if not isinstance(value,dict): raise AttestationError(f"{path} must contain an object")
    return value

def build(manifest:dict[str,Any], artifact_root:pathlib.Path, repository:str, source_ref:str, signer_workflow:str, raw:dict[str,Any])->dict[str,Any]:
    subjects=[]
    for item in manifest.get("subjects",[]):
        path=artifact_root/item["filename"]
        if not path.is_file() or sha256(path)!=item["sha256"] or path.stat().st_size!=item["size_bytes"]:
            raise AttestationError(f"subject bytes differ: {item.get('name')}")
        verification=raw.get(item["name"])
        if not isinstance(verification,(dict,list)) or not verification: raise AttestationError(f"missing verification result for {item['name']}")
        subjects.append({**item,"verification_sha256":hashlib.sha256(json.dumps(verification,sort_keys=True,separators=(",",":")).encode()).hexdigest()})
    if not subjects: raise AttestationError("no subjects were verified")
    return {"schema_version":1,"status":"verified","release_id":"health-v1","source_revision":manifest["source_revision"],"repository":repository,"source_ref":source_ref,"signer_workflow":signer_workflow,"subjects":subjects,"claims":{"artifact_digest_verified":True,"source_revision_constraint_applied":True,"source_ref_constraint_applied":True,"signer_workflow_constraint_applied":True,"security_properties_beyond_provenance_not_inferred":True}}

def write(path:pathlib.Path,value:dict[str,Any])->None:
    if path.exists(): raise AttestationError(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open("x") as h: json.dump(value,h,indent=2,sort_keys=True); h.write("\n")
    os.chmod(path,0o600)

def self_test()->None:
    with tempfile.TemporaryDirectory() as raw:
        root=pathlib.Path(raw); art=root/"health.happ"; art.write_bytes(b"happ")
        manifest={"source_revision":"1"*40,"subjects":[{"name":"health-happ","filename":art.name,"sha256":sha256(art),"size_bytes":4}]}
        report=build(manifest,root,"owner/repo","refs/heads/main","owner/repo/.github/workflows/ci.yml",{"health-happ":{"ok":True}})
        assert report["status"]=="verified"
        out=root/"report.json"; write(out,report)
    print("release attestation verifier self-test: ok")

def main()->int:
    p=argparse.ArgumentParser(); p.add_argument("--subjects-manifest",type=pathlib.Path); p.add_argument("--artifact-root",type=pathlib.Path); p.add_argument("--repository"); p.add_argument("--source-ref",default="refs/heads/main"); p.add_argument("--signer-workflow"); p.add_argument("--output",type=pathlib.Path); p.add_argument("--self-test",action="store_true"); a=p.parse_args()
    if a.self_test: self_test(); return 0
    if not all((a.subjects_manifest,a.artifact_root,a.repository,a.signer_workflow,a.output)): raise AttestationError("all verification arguments are required")
    manifest=load(a.subjects_manifest.resolve()); raw={}
    for item in manifest.get("subjects",[]):
        path=a.artifact_root.resolve()/item["filename"]
        command=["gh","attestation","verify",str(path),"--repo",a.repository,"--source-digest",manifest["source_revision"],"--source-ref",a.source_ref,"--signer-workflow",a.signer_workflow,"--format","json"]
        result=subprocess.run(command,text=True,capture_output=True,check=False)
        if result.returncode: raise AttestationError(f"GitHub attestation verification failed for {item['name']}: {result.stderr.strip() or result.stdout.strip()}")
        try: raw[item["name"]]=json.loads(result.stdout)
        except json.JSONDecodeError as e: raise AttestationError(f"non-JSON gh output for {item['name']}") from e
    report=build(manifest,a.artifact_root.resolve(),a.repository,a.source_ref,a.signer_workflow,raw); write(a.output.resolve(),report); print(a.output.resolve()); return 0
if __name__=="__main__":
    try: raise SystemExit(main())
    except (AttestationError,OSError,KeyError,ValueError,json.JSONDecodeError) as e: print(f"release attestation error: {e}"); raise SystemExit(1)
