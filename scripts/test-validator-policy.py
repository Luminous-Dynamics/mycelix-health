#!/usr/bin/env python3
from __future__ import annotations
import hashlib, importlib.util, json, subprocess, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("validator_policy",ROOT/"scripts"/"validator-policy.py"); assert spec and spec.loader
m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
policy={"policy_version":1,"policy_id":"health-validator-policy","revision":1,"previous_revision_digest":None,"effective_from_ms":1700000000000,"expires_at_ms":1700001000000,"deployment_evidence_sha256":[9]*32,"members":[{"member_id":"v1","organization_id":"org-a","agent_binding":"uhCAk-validator-a","agent_bytes":[1]*39,"roles":["clinical"],"active_from_ms":1700000000000,"active_until_ms":1700001000000,"revoked_at_ms":None,"compromised_at_ms":None},{"member_id":"v2","organization_id":"org-b","agent_binding":"uhCAk-validator-b","agent_bytes":[2]*39,"roles":["privacy"],"active_from_ms":1700000000000,"active_until_ms":1700001000000,"revoked_at_ms":None,"compromised_at_ms":None}],"rules":[{"operation_class":"sensitive_record_release","threshold":2,"min_distinct_organizations":2,"required_roles":["clinical","privacy"],"max_attestation_age_ms":60000,"require_activity_observation":True}]}
payload=m.canonical(policy); assert hashlib.sha256(payload).hexdigest()=="ea73f0226423b564226563857a4a2d37b5ac104d967a890a9bff6b69a49880a3"
with tempfile.TemporaryDirectory() as d:
 p=Path(d); priv=p/"priv.pem"; pub=p/"pub.pem"; subprocess.run(["openssl","genpkey","-algorithm","ED25519","-out",str(priv)],check=True); subprocess.run(["openssl","pkey","-in",str(priv),"-pubout","-out",str(pub)],check=True)
 sig=m.sign(priv,payload); m.verify(pub,payload,sig); assert len(m.public_raw(pub))==32
 bad=bytearray(payload); bad[-1]^=1
 try: m.verify(pub,bytes(bad),sig)
 except m.PolicyError: pass
 else: raise AssertionError("tampered policy verified")
print("validator policy ceremony self-test passed")
