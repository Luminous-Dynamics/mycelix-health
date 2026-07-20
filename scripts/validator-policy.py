#!/usr/bin/env python3
"""Canonicalize, sign, and verify governed Health validator policy revisions."""
from __future__ import annotations
import argparse, hashlib, json, struct, subprocess, tempfile
from pathlib import Path
from typing import Any

DOMAIN=b"MYCELIX-HEALTH-VALIDATOR-POLICY-V1"
ROLE={"clinical":1,"privacy":2,"compliance":3,"safety":4}
OP={"standard_mutation":1,"sensitive_record_release":2,"provider_correction":3,"break_glass_acknowledgement":4}

class PolicyError(ValueError): pass

def b32(v:Any,label:str)->bytes:
    raw=bytes.fromhex(v) if isinstance(v,str) else bytes(v)
    if len(raw)!=32: raise PolicyError(f"{label} must be 32 bytes")
    return raw

def raw(v:Any,n:int,label:str)->bytes:
    out=bytes.fromhex(v) if isinstance(v,str) else bytes(v)
    if len(out)!=n: raise PolicyError(f"{label} must be {n} bytes")
    return out

def lp(v:bytes)->bytes: return struct.pack(">I",len(v))+v
def text(v:Any,label:str)->bytes:
    if not isinstance(v,str) or not v.strip() or len(v.encode())>128: raise PolicyError(f"invalid {label}")
    return v.encode()
def oi64(v:Any)->bytes: return b"\0" if v is None else b"\1"+struct.pack(">q",int(v))

def canonical(policy:dict[str,Any])->bytes:
    if policy.get("policy_version")!=1 or int(policy.get("revision",0))<1: raise PolicyError("invalid policy version/revision")
    revision=int(policy["revision"]); prev=policy.get("previous_revision_digest")
    if (revision==1)!=(prev is None): raise PolicyError("invalid previous revision binding")
    start=int(policy["effective_from_ms"]); end=int(policy["expires_at_ms"])
    if start<=0 or end<=start or end-start>366*24*60*60*1000: raise PolicyError("invalid policy window")
    members=policy.get("members"); rules=policy.get("rules")
    if not isinstance(members,list) or not 1<=len(members)<=64: raise PolicyError("invalid members")
    if not isinstance(rules,list) or not 1<=len(rules)<=16: raise PolicyError("invalid rules")
    ids=set(); agents=set(); ops=set(); out=bytearray(DOMAIN); out.append(1)
    out+=lp(text(policy["policy_id"],"policy id")); out+=struct.pack(">Q",revision)
    out+=b"\0" if prev is None else b"\1"+b32(prev,"previous digest")
    out+=struct.pack(">q",start)+struct.pack(">q",end)+b32(policy["deployment_evidence_sha256"],"deployment digest")
    out+=struct.pack(">I",len(members))
    for m in members:
        mid=text(m["member_id"],"member id"); org=text(m["organization_id"],"organization id"); agent=text(m["agent_binding"],"agent binding")
        if mid in ids or agent in agents: raise PolicyError("duplicate member or agent")
        ids.add(mid); agents.add(agent)
        roles=sorted(set(m["roles"]),key=ROLE.__getitem__)
        if not roles: raise PolicyError("empty roles")
        active=int(m["active_from_ms"]); until=int(m["active_until_ms"])
        if active<=0 or until<=active: raise PolicyError("invalid member window")
        out+=lp(mid)+lp(org)+lp(agent)+lp(raw(m["agent_bytes"],39,"agent bytes"))+struct.pack(">I",len(roles))+bytes(ROLE[r] for r in roles)
        out+=struct.pack(">q",active)+struct.pack(">q",until)+oi64(m.get("revoked_at_ms"))+oi64(m.get("compromised_at_ms"))
    ordered=sorted(rules,key=lambda r:OP[r["operation_class"]]); out+=struct.pack(">I",len(ordered))
    for r in ordered:
        op=r["operation_class"]
        if op in ops: raise PolicyError("duplicate operation rule")
        ops.add(op); threshold=int(r["threshold"]); orgs=int(r["min_distinct_organizations"]); roles=sorted(set(r["required_roles"]),key=ROLE.__getitem__); age=int(r["max_attestation_age_ms"])
        if threshold<1 or orgs<1 or orgs>threshold or not roles or age<=0 or age>86400000: raise PolicyError("invalid operation rule")
        out+=bytes([OP[op],threshold,orgs])+struct.pack(">I",len(roles))+bytes(ROLE[x] for x in roles)+struct.pack(">q",age)+bytes([1 if r["require_activity_observation"] else 0])
    return bytes(out)

def run(*args:str)->bytes:
    p=subprocess.run(args,check=False,capture_output=True)
    if p.returncode: raise PolicyError((p.stderr or p.stdout).decode().strip())
    return p.stdout

def sign(private:Path,payload:bytes)->bytes:
    with tempfile.TemporaryDirectory() as d:
        p=Path(d); (p/"payload").write_bytes(payload)
        run("openssl","pkeyutl","-sign","-rawin","-inkey",str(private),"-in",str(p/"payload"),"-out",str(p/"sig"))
        return raw((p/"sig").read_bytes(),64,"signature")

def verify(public:Path,payload:bytes,signature:bytes)->None:
    with tempfile.TemporaryDirectory() as d:
        p=Path(d); (p/"payload").write_bytes(payload); (p/"sig").write_bytes(signature)
        run("openssl","pkeyutl","-verify","-rawin","-pubin","-inkey",str(public),"-in",str(p/"payload"),"-sigfile",str(p/"sig"))

def public_raw(public:Path)->bytes:
    der=run("openssl","pkey","-pubin","-in",str(public),"-outform","DER"); prefix=bytes.fromhex("302a300506032b6570032100")
    if not der.startswith(prefix) or len(der)!=len(prefix)+32: raise PolicyError("not canonical Ed25519 public key")
    return der[-32:]

def main()->int:
    ap=argparse.ArgumentParser(); sub=ap.add_subparsers(dest="cmd",required=True)
    s=sub.add_parser("sign"); s.add_argument("policy",type=Path); s.add_argument("--private-key",type=Path,required=True); s.add_argument("--public-key",type=Path,required=True); s.add_argument("--key-id",required=True); s.add_argument("--output",type=Path,required=True)
    v=sub.add_parser("verify"); v.add_argument("signed",type=Path); v.add_argument("--public-key",type=Path,required=True)
    args=ap.parse_args()
    if args.cmd=="sign":
        policy=json.loads(args.policy.read_text()); payload=canonical(policy); sig=sign(args.private_key,payload); verify(args.public_key,payload,sig)
        args.output.write_text(json.dumps({"policy":policy,"signer_key_id":args.key_id,"signer_public_key":list(public_raw(args.public_key)),"signature_bytes":list(sig)},indent=2,sort_keys=True)+"\n")
        print(f"policy_sha256={hashlib.sha256(payload).hexdigest()}")
    else:
        signed=json.loads(args.signed.read_text()); payload=canonical(signed["policy"]); verify(args.public_key,payload,raw(signed["signature_bytes"],64,"signature")); print(f"policy_sha256={hashlib.sha256(payload).hexdigest()}")
    return 0
if __name__=="__main__": raise SystemExit(main())
