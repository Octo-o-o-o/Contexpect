#!/usr/bin/env python3
"""Enumerate the entire frozen release surface and verify submitted evidence identity.

This validates an evidence inventory; it does not authenticate an external
reviewer or substitute a file hash for a semantic/human acceptance verdict.
"""
import argparse
import collections
import csv
import hashlib
import json
import pathlib
import subprocess

ROOT=pathlib.Path(__file__).resolve().parents[1]


def load(path):
    return json.loads(path.read_text())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def requirements(root=ROOT):
    matrix=load(root/'acceptance/compatibility-matrix.yaml')
    capabilities=load(root/'acceptance/context-capability-matrix.yaml')
    projection=load(root/'acceptance/projection-matrix.yaml')
    corpus=load(root/'acceptance/corpus-manifest.json')
    rows=[]
    def add(id,kind,**detail):rows.append(dict(id=id,kind=kind,**detail))
    from contexpect_contract import REQUIRED_GATES
    for name,command in REQUIRED_GATES:add('gate/'+name,'software-gate',command=command)
    for coordinate in matrix['coordinates']:
        for axis in ['static','native']:
            status=coordinate[f'{axis}_support']
            if status!='not-applicable':add(f"coordinate/{axis}/{coordinate['id']}",'coordinate',expectation=status,os_lane=coordinate['os_lane'])
    for cell in capabilities['cells']:
        if cell['status']!='not-applicable':add('capability/'+cell['id'],'capability',expectation=cell['status'],os_lane=cell['os_lane'])
    for cell in projection['cells']:
        if not cell['required_write']:continue
        coordinates=[c for c in matrix['coordinates'] if c['family_id']==cell['family_id'] and c['surface']==cell['native_target']['surface'] and c['static_support']!='not-applicable']
        if not coordinates:raise ValueError('required-write cell has no applicable coordinate: '+cell['id'])
        for coordinate in coordinates:
            for gate in projection['gates_per_required_write_cell']:
                add(f"projection/{cell['id']}/{coordinate['id']}/{gate}",'projection',os_lane=coordinate['os_lane'],authority=cell['authority'])
    for fixture in corpus['fixtures']:
        add('corpus/'+fixture['id'],'corpus',corpus=fixture['corpus'],input_digest=fixture['digest_sha256'])
    with (root/'acceptance/traceability.csv').open() as file:
        for row in csv.DictReader(file):
            add('requirement/'+row['requirement_id'],'normative',source_digest=row['source_digest'],implementation=row['implementation'])
    for lane in matrix['os_lanes']:
        for gate in ['install-cli','install-desktop','upgrade','uninstall-preserves-native','crash-recovery','webview-security','a11y','performance','soak-72h-1000-changes']:
            add(f"lane/{lane['id']}/{gate}",'os-lane',os_lane=lane['id'],build=lane['build'])
        for artifact in ['cli','desktop','sbom-cyclonedx','sbom-spdx','notice-license','signed-adapter-corpus-update']:
            add(f"artifact/{lane['id']}/{artifact}",'release-artifact',os_lane=lane['id'],build=lane['build'])
    for gate in ['cadence-four-cycles','history-50-sessions','encrypted-A-to-B','effect-live-runner','qa-eight-participants-six-tasks','independent-readback','integrated-product-gate']:
        add('integration/'+gate,'integration')
    ids=[r['id'] for r in rows]
    if len(ids)!=len(set(ids)):raise ValueError('duplicate required identifier')
    return rows


def evaluate(required, evidence, artifact_root, candidate):
    if evidence.get('schema')!='ctxpect-release-evidence-v1':raise ValueError('unknown evidence schema')
    if evidence.get('candidate')!=candidate:raise ValueError('candidate mismatch')
    known={r['id'] for r in required};provided={}
    for item in evidence.get('items',[]):
        if item['id'] not in known:raise ValueError('unknown requirement id: '+item['id'])
        if item['id'] in provided:raise ValueError('duplicate evidence id: '+item['id'])
        provided[item['id']]=item
    output=[]
    for requirement in required:
        row=dict(requirement);item=provided.get(row['id'])
        row['evidence_state']='missing'
        if item:
            row['evidence_state']='invalid'
            try:
                artifact=(artifact_root/item['artifact']).resolve(strict=True)
                artifact.relative_to(artifact_root.resolve())
                if not artifact.is_file() or sha(artifact)!=item['sha256']:raise ValueError('artifact digest mismatch')
                if item.get('candidate')!=candidate:raise ValueError('item candidate mismatch')
                if row.get('build') and item.get('os_build')!=row['build']:raise ValueError('OS build mismatch')
                if row.get('os_lane') and item.get('os_lane')!=row['os_lane']:raise ValueError('OS lane mismatch')
                if row.get('input_digest') and item.get('input_digest')!=row['input_digest']:raise ValueError('fixture digest mismatch')
                if item.get('verdict') not in ['pass','fail','unavailable']:raise ValueError('verdict missing')
                row['evidence_state']='verified-file'
                row['reported_verdict']=item['verdict']
                row['artifact']=item['artifact'];row['sha256']=item['sha256']
            except (ValueError,KeyError,OSError) as error:row['reason']=str(error)
        output.append(row)
    counts=dict(collections.Counter(row['evidence_state'] for row in output))
    return dict(schema='ctxpect-release-inventory-v1',candidate=candidate,requirements=output,counts=counts,
                evidence_inventory_complete=all(r['evidence_state']=='verified-file' for r in output),
                semantic_and_reviewer_authentication='not-performed-by-this-inventory',
                full_product_accepted=False)


def candidate_identity(root=ROOT):
    head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    diff=subprocess.check_output(['git','diff','--binary','HEAD'],cwd=root)
    # Include untracked implementation, excluding Git-ignored local evidence.
    paths=subprocess.check_output(['git','ls-files','--others','--exclude-standard','-z'],cwd=root).split(b'\0')
    untracked=[]
    for raw in paths:
        if not raw:continue
        path=raw.decode();file=root/path
        if file.is_symlink():untracked.append([path,'symlink:'+str(file.readlink())])
        else:untracked.append([path,sha(file)])
    return {'head':head,'git_diff_v1':hashlib.sha256(diff).hexdigest(),'untracked_sha256':hashlib.sha256(json.dumps(untracked,sort_keys=True).encode()).hexdigest()}


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--evidence',type=pathlib.Path);parser.add_argument('--artifact-root',type=pathlib.Path,default=ROOT);parser.add_argument('--output',required=True,type=pathlib.Path);args=parser.parse_args()
    candidate=candidate_identity();evidence=load(args.evidence) if args.evidence else dict(schema='ctxpect-release-evidence-v1',candidate=candidate,items=[])
    try:report=evaluate(requirements(),evidence,args.artifact_root,candidate)
    except ValueError as error:print(json.dumps({'error':str(error)}));return 2
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k!='requirements'}))
    return 0 if report['evidence_inventory_complete'] else 3

if __name__=='__main__':raise SystemExit(main())
