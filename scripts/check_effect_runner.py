#!/usr/bin/env python3
"""Real local command adapter tests. No provider calls and no model effect claim."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import tempfile
import time


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()


def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--bin',default='target/debug/ctxpect',type=pathlib.Path);args=parser.parse_args()
    binary=args.bin.resolve(); checks=[]
    with tempfile.TemporaryDirectory(prefix='ctxpect-runner-') as td:
        root=pathlib.Path(td).resolve(); project=root/'project';project.mkdir();store=root/'store'
        (store/'policies').mkdir(parents=True);(store/'exceptions').mkdir()
        def write(path,value): path.write_text(json.dumps(value));return path
        write(store/'policies/active.json',[dict(layer=l,mode='detect-only' if l in ['user','session'] else 'enforceable',rules=[]) for l in ['organization','team','project','user','session']])
        for action in ['experiment.execute','experiment.persist']:
            id=action.replace('.','-');write(store/f'exceptions/{id}.json',dict(exception_id=id,requester='test',action=action,project_digest=hashlib.sha256(f'project:{project}'.encode()).hexdigest(),target='*',state='approved',created_at=1,expires_at=4102444800,reason='disposable grant',approver='test-only'))
        runner=root/'runner.py'
        runner.write_text('''#!/usr/bin/python3
import hashlib,json,pathlib,sys,time
r=json.load(sys.stdin)
if r['input']=='timeout':time.sleep(4)
if r['input']=='crash':sys.exit(8)
if r['input']=='malformed':print('{}');sys.exit(0)
# A real deterministic gate executes against the copied source file.
actual=pathlib.Path('value.txt').read_text()
outcome='pass' if actual==r['input'] else 'fail'
files={'value.txt':actual}
locked=dict(r['contract']['locked'])
locked['code_digest']=hashlib.sha256(json.dumps(files,sort_keys=True,separators=(',',':')).encode()).hexdigest()
locked['tool_availability_digest']=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest()
if r['input']=='drift':locked['model']='changed'
print(json.dumps({'schema':'ctxpect-runner-result-v1','run_id':r['run_id'],'request_digest':r['request_digest'],'outcome':outcome,'observed':locked,'sandbox':'none-explicit','gate_evidence_digest':hashlib.sha256((actual+':'+outcome).encode()).hexdigest()}))
''');runner.chmod(0o700)
        pin=hashlib.sha256(runner.read_bytes()).hexdigest(); files={'value.txt':'42'}
        def request(id,control='wrong',treatment='42',n=8,timeout=2,total=30):
            contract=dict(schema='experiment-contract-v1',experiment_id=id,primary_outcome='task-pass',margin_pp=10,pairing='paired-by-task',n_planned=n,alpha='0.05',power='0.80',multiplicity='none',itt='count-as-fail',locked=dict(code_digest=digest(files),model='local-deterministic-test',harness='python-gate-fixture-v1',tool_availability_digest=pin),frozen_at=f'{int(time.time())-1}.000Z',invalidation=['runner drift'])
            return dict(schema='ctxpect-command-runner-v1',contract=contract,runner=dict(path=str(runner),sha256=pin,version='ctxpect-command-runner-v1',sandbox='none-explicit'),files=dict(files),budget=dict(max_runs=2*n,timeout_seconds=timeout,total_seconds=total),tasks=[dict(id=f't{i}',control=control,treatment=treatment) for i in range(n)])
        def invoke(req):
            path=write(root/'request.json',req)
            p=subprocess.run([str(binary),'experiment','--execute','--adapter','command-v1','--from',str(path),'--project',str(project),'--store',str(store),'--json'],capture_output=True,timeout=45)
            return p.returncode,json.loads(p.stdout)
        r=request('beneficial');code,out=invoke(r)
        assert code==0 and out['decision']=='supported-beneficial',(code,out)
        assert out['process_attempts']==16 and len(out['executor_receipts'])==16
        assert out['sandbox_host_verified'] is False
        assert all(e['stdout_digest'] and e['gate_evidence_digest'] for e in out['executor_receipts'])
        checks.append('16 real child processes and gate evidence support fixture-only beneficial')
        before=(store/'runnerjobs/beneficial.json').read_bytes()
        code,out=invoke(r);assert code!=0 and out['error']['code']=='effect.execution_exists'
        assert (store/'runnerjobs/beneficial.json').read_bytes()==before
        checks.append('same experiment never launches twice')
        for id,control,treatment,n,decision in [('harmful','42','wrong',8,'supported-harmful'),('equivalent','42','42',40,'supported-equivalent-within-margin'),('small','wrong','42',1,'inconclusive')]:
            code,out=invoke(request(id,control,treatment,n));assert out['decision']==decision,(code,out);checks.append(decision)
        code,out=invoke(request('protocol','malformed','42',8));assert out['decision']=='inconclusive' and out['reason_code']=='effect.runner_protocol_invalid';checks.append('malformed response cannot become beneficial via ITT')
        code,out=invoke(request('drift','drift','42',2));assert out['decision']=='inconclusive' and out['reason_code']=='effect.confounder_drift';checks.append('observed confounder drift')
        code,out=invoke(request('timeout','timeout','42',1,1));assert any(r['outcome']=='timeout' for r in out['runs']) and out['decision']=='inconclusive';checks.append('timeout retained under ITT')
        code,out=invoke(request('crash','crash','42',1));assert any(r['outcome']=='crash' for r in out['runs']);checks.append('process crash retained under ITT')
        code,out=invoke(request('budget','timeout','42',4,1,1));assert out['reason_code']=='effect.budget_exhausted' and out['process_attempts']<8;checks.append('total budget stops new processes and forces inconclusive')
        for id,mutate,reason in [
            ('bad-code',lambda r:r['files'].update({'value.txt':'43'}),'effect.confounder_drift'),
            ('bad-tool',lambda r:r['runner'].update(sha256='0'*64),'effect.runner_drift'),
            ('bad-path',lambda r:r['files'].update({'../escape':'x'}),'effect.runner_path_invalid'),
            ('bad-budget',lambda r:r['budget'].update(max_runs=1),'effect.budget_invalid'),
        ]:
            r=request(id);mutate(r);code,out=invoke(r);assert code!=0 and out['error']['code']==reason,(reason,code,out);assert not (store/f'runnerjobs/{id}.json').exists();checks.append(reason)
        print(json.dumps({'scope':'real-local-command-adapter-with-deterministic-fixture-gate','passed':len(checks),'checks':checks,'model_calls':0,'human_acceptance':False}))

if __name__=='__main__':main()
