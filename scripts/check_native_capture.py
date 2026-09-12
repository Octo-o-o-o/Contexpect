#!/usr/bin/env python3
"""Check the product's native capture adapters with actual pinned executables."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import tempfile


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--bin',default='target/debug/ctxpect',type=pathlib.Path)
    parser.add_argument('--codex',required=True,type=pathlib.Path);parser.add_argument('--grok',required=True,type=pathlib.Path);args=parser.parse_args()
    checks=[]
    with tempfile.TemporaryDirectory(prefix='ctxpect-native-capture-') as td:
        root=pathlib.Path(td).resolve();project=root/'project';project.mkdir();(project/'AGENTS.md').write_text('NATIVE_CAPTURE_PRIVATE_BODY_81\n')
        store=root/'store';(store/'policies').mkdir(parents=True);(store/'exceptions').mkdir()
        (store/'policies/active.json').write_text(json.dumps([dict(layer=l,mode='detect-only' if l in ['user','session'] else 'enforceable',rules=[]) for l in ['organization','team','project','user','session']]))
        (store/'exceptions/native.json').write_text(json.dumps(dict(exception_id='native',action='native.capture',project_digest=hashlib.sha256(f'project:{project}'.encode()).hexdigest(),target='*',state='approved',created_at=1,expires_at=4102444800,requester='test',approver='test-only',reason='disposable native test')))
        for family,version,adapter,tool in [('codex','0.147.0','codex-prompt-input-v1',args.codex),('grok-build','1.0.13','grok-inspect-v1',args.grok)]:
            tool=tool.resolve();profile=root/f'{family}.json';profile.write_text(json.dumps(dict(schema='ctxpect-native-tool-profile-v1',tool=dict(path=str(tool),sha256=hashlib.sha256(tool.read_bytes()).hexdigest()))))
            base=[str(args.bin.resolve()),'collect','--execute','--adapter',adapter,'--profile',str(profile),'--project',str(project),'--store',str(store),'--harness',family,'--version',version,'--json']
            p=subprocess.run(base,capture_output=True,timeout=40);out=json.loads(p.stdout)
            assert p.returncode==0,(p.returncode,out)
            observation=out['native_observation'];assert observation['native_executed'] is True and observation['model_executed'] is False
            assert observation['claims_upgraded'] is False and observation['raw_body_saved'] is False
            assert observation['digest_basis']=='local-keyed-not-cross-device-comparable'
            persisted=(store/f"nativeobservations/{out['observation_id']}.json").read_text();assert 'NATIVE_CAPTURE_PRIVATE_BODY_81' not in persisted and str(project) not in persisted
            if family=='codex':assert observation['observation']['normalization']=='codex-prompt-array-v1'
            else:assert observation['observation']['instruction_body_observed'] is False
            checks.append(dict(family=family,result='pass',actual_os_build=observation['actual_os_build'],frozen_os_build_matches=observation['frozen_os_build_matches']))
            profile.write_text(json.dumps(dict(schema='ctxpect-native-tool-profile-v1',tool=dict(path=str(tool),sha256='0'*64))))
            p=subprocess.run(base,capture_output=True,timeout=40);out=json.loads(p.stdout);assert p.returncode!=0 and out['error']['code']=='native.tool_drift';checks.append(dict(family=family,result='tool-drift-refused'))
        print(json.dumps(dict(scope='actual-product-native-capture',checks=checks,model_calls=0,full_frozen_matrix=False)))

if __name__=='__main__':main()
