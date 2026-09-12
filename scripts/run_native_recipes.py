#!/usr/bin/env python3
"""Invoke every frozen oracle recipe, recording actual shape without inventing a verdict."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--codex',required=True,type=pathlib.Path)
    parser.add_argument('--grok',required=True,type=pathlib.Path)
    parser.add_argument('--output',required=True,type=pathlib.Path)
    args=parser.parse_args();rows=[]
    for family,filename,executable in [('codex','codex__debug-prompt-input.jsonl',args.codex.resolve()),('grok-build','grok-build__inspect-json.jsonl',args.grok.resolve())]:
        version=subprocess.run([str(executable),'--version'],capture_output=True,text=True,timeout=10).stdout.strip()
        for row in map(json.loads,(ROOT/'acceptance/corpus/development/oracle'/filename).read_text().splitlines()):
            with tempfile.TemporaryDirectory(prefix='ctxpect-oracle-recipe-') as td:
                temp=pathlib.Path(td);project=temp/'project';project.mkdir();profile=temp/'profile';profile.mkdir();(profile/'codex').mkdir()
                for item in row['input_files']:
                    source=(ROOT/item['path']).resolve();source.relative_to(ROOT/'acceptance/corpus/development/oracle/inputs')
                    data=source.read_bytes();assert hashlib.sha256(data).hexdigest()==item['digest_sha256']
                    relative=source.relative_to((ROOT/row['input_path']).resolve());dest=project/relative;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
                env={'PATH':'/usr/bin:/bin','HOME':str(profile),'CODEX_HOME':str(profile/'codex'),'GROK_HOME':str(profile/'grok'),'XDG_CONFIG_HOME':str(profile/'config'),'LANG':'en_US.UTF-8'}
                record={'id':row['id'],'input_digest_sha256':row['input_digest_sha256'],'family':family,'version':version,'required_version_matches':version.split()[1]==row['version'],'executed':True,'model_executed':False,'semantic_verdict':'unvalidated','synthetic_input':True}
                try:
                    p=subprocess.run([str(executable),*row['command'][1:]],cwd=project,env=env,input=b'',capture_output=True,timeout=25)
                    record.update(exit_code=p.returncode,stdout_digest=hashlib.sha256(p.stdout).hexdigest(),stderr_digest=hashlib.sha256(p.stderr).hexdigest())
                    try:
                        actual=json.loads(p.stdout)
                        shape='array' if isinstance(actual,list) else 'object' if isinstance(actual,dict) else 'scalar'
                        expected=row['expected_native_shape']['required_keys']
                        matches=isinstance(actual,dict) and all(k in actual for k in expected)
                        record.update(actual_shape=shape,expected_keys=expected,shape_matches=matches,
                                      reason='recipe shape matches; semantic assertions still require a real parser' if matches else 'synthetic expected shape differs from actual pinned native output')
                    except (ValueError,UnicodeDecodeError):record.update(actual_shape='invalid-json',shape_matches=False)
                except subprocess.TimeoutExpired:record.update(exit_code=124,shape_matches=False,reason='timeout')
                rows.append(record)
    report={'schema':'ctxpect-native-recipe-execution-v1','cases':rows,'invoked':len(rows),'process_success':sum(r.get('exit_code')==0 for r in rows),'shape_matches':sum(r['shape_matches'] for r in rows),'semantic_pass':0,'full_native_acceptance':False}
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k!='cases'}))
    return 0 if all(r.get('exit_code')==0 for r in rows) else 1

if __name__=='__main__':raise SystemExit(main())
