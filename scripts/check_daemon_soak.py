#!/usr/bin/env python3
"""Owned, resumable-evidence daemon soak on disposable instructions; never a full OS matrix.

The worker checks process lifetime, stable observations and notification de-dup.
It records actual monotonic elapsed time, so a short run cannot become 72 hours.
"""
import argparse
import hashlib
import json
import os
import pathlib
import signal
import subprocess
import tempfile
import time
import urllib.request


def write_atomic(path,value):
    temporary=path.with_suffix('.tmp');temporary.write_text(json.dumps(value,indent=2)+'\n');temporary.replace(path)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--bin',default='target/debug/ctxpect',type=pathlib.Path)
    parser.add_argument('--output',required=True,type=pathlib.Path);parser.add_argument('--seconds',type=int,default=20)
    parser.add_argument('--changes',type=int,default=20);parser.add_argument('--poll-ms',type=int,default=100)
    args=parser.parse_args();args.output=args.output.resolve();args.output.mkdir(parents=True,exist_ok=True)
    if args.seconds<1 or args.changes<1:parser.error('positive duration and changes required')
    binary=args.bin.resolve();identity=hashlib.sha256(binary.read_bytes()).hexdigest()
    started=time.monotonic();stopping=False
    def stop(_sig,_frame):
        nonlocal stopping;stopping=True
    for sig in [signal.SIGTERM,signal.SIGINT]:signal.signal(sig,stop)
    report=dict(schema='ctxpect-soak-v1',binary_sha256=identity,worker_pid=os.getpid(),started_at=time.time(),requested_seconds=args.seconds,
                requested_changes=args.changes,state='running',observations=[],scope='periodic-static-instructions',
                transient_changes_between_polls='not-observed',full_frozen_os_lane=False,soak_72h_complete=False)
    with tempfile.TemporaryDirectory(prefix='ctxpect-soak-') as td:
        root=pathlib.Path(td);project=root/'project';project.mkdir();store=root/'store';source=project/'AGENTS.md';source.write_text('soak baseline\n')
        output=(args.output/'daemon.stderr.log').open('wb')
        process=subprocess.Popen([str(binary),'daemon','start','--execute','--poll-ms',str(args.poll_ms),'--project',str(project),'--store',str(store),'--listen','127.0.0.1:0'],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=output,start_new_session=True)
        report['daemon_pid']=process.pid
        try:
            deadline=time.monotonic()+15
            while not (store/'daemon.addr').exists():
                if process.poll() is not None or time.monotonic()>deadline:raise RuntimeError('daemon startup failed')
                time.sleep(.02)
            address=(store/'daemon.addr').read_text().strip()
            def monitor():
                if process.poll() is not None:raise RuntimeError('daemon died')
                with urllib.request.urlopen(f'http://{address}/api/v1/monitor',timeout=10) as response:data=json.load(response)
                runtime=data.get('continuous',{})
                if runtime.get('error_code'):raise RuntimeError(runtime['error_code'])
                return runtime
            def observe(previous=None):
                deadline=time.monotonic()+15
                while not stopping:
                    state=monitor()
                    if state.get('last_snapshot_digest') and state['last_snapshot_digest']!=previous:return state
                    if time.monotonic()>deadline:raise RuntimeError('stable injected change was not observed')
                    time.sleep(.02)
                raise InterruptedError('stopped')
            baseline=observe();current=baseline['last_snapshot_digest'];report['observations'].append(current)
            control_before=(store/'daemon.control').read_bytes()
            duplicate=subprocess.run([str(binary),'daemon','start','--store',str(store),'--listen','127.0.0.1:0','--json'],stdin=subprocess.DEVNULL,capture_output=True,timeout=10)
            if duplicate.returncode==0 or json.loads(duplicate.stdout).get('error',{}).get('code')!='daemon.already_running':raise RuntimeError('second daemon did not refuse store ownership')
            if (store/'daemon.control').read_bytes()!=control_before:raise RuntimeError('second daemon changed live control record')
            report['duplicate_daemon_refused']=True
            next_change=0
            while time.monotonic()-started<args.seconds or next_change<args.changes:
                if stopping:raise InterruptedError('stopped')
                elapsed=time.monotonic()-started
                due=next_change*args.seconds/args.changes
                if next_change<args.changes and elapsed>=due:
                    content=f'soak instruction {next_change}\n';source.write_text(content)
                    state=observe(current);current=state['last_snapshot_digest'];report['observations'].append(current)
                    # Repeating identical bytes must not create another receipt/notification.
                    source.write_text(content);seen_scans=state['scans'];deadline=time.monotonic()+15
                    while monitor()['scans']<=seen_scans:
                        if time.monotonic()>deadline:raise RuntimeError('poll stalled')
                        time.sleep(.02)
                    if monitor()['last_snapshot_digest']!=current:raise RuntimeError('same bytes changed digest')
                    next_change+=1
                    notifications=list((store/'notifications').glob('*.json'))
                    if len(notifications)!=len(set(report['observations'])):raise RuntimeError('notification duplicate or loss')
                    report['observed_changes']=next_change;report['notification_count']=len(notifications)
                else:monitor();time.sleep(min(1,max(.02,due-elapsed)))
                report['elapsed_seconds']=round(time.monotonic()-started,3)
                write_atomic(args.output/'state.json',report)
            # Restart with identical source/store: no duplicate notification.
            before=len(list((store/'notifications').glob('*.json')))
            stopped=subprocess.run([str(binary),'daemon','stop','--store',str(store),'--json'],stdin=subprocess.DEVNULL,capture_output=True,timeout=10)
            if stopped.returncode!=0 or json.loads(stopped.stdout).get('stopped') is not True:raise RuntimeError('product daemon stop did not confirm exit')
            process.wait(timeout=5);report['product_shutdown_verified']=True
            process=subprocess.Popen([str(binary),'daemon','start','--execute','--poll-ms',str(args.poll_ms),'--project',str(project),'--store',str(store),'--listen','127.0.0.1:0'],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=output,start_new_session=True)
            old_address=address;deadline=time.monotonic()+15
            while address==old_address:
                if time.monotonic()>deadline:raise RuntimeError('restart did not bind')
                if (store/'daemon.addr').exists():address=(store/'daemon.addr').read_text().strip()
                time.sleep(.02)
            observe()
            assert len(list((store/'notifications').glob('*.json')))==before
            report.update(state='completed',restart_no_duplicate=True,elapsed_seconds=round(time.monotonic()-started,3),finished_at=time.time())
            report['soak_72h_complete']=report['elapsed_seconds']>=72*3600 and next_change>=1000
            write_atomic(args.output/'state.json',report)
            print(json.dumps({k:v for k,v in report.items() if k!='observations'}));return 0
        except BaseException as error:
            report.update(state='interrupted' if isinstance(error,InterruptedError) else 'failed',error=str(error),elapsed_seconds=round(time.monotonic()-started,3),finished_at=time.time())
            write_atomic(args.output/'state.json',report);print(json.dumps({k:v for k,v in report.items() if k!='observations'}));return 1
        finally:
            if process.poll() is None:
                process.terminate()
                try:process.wait(timeout=5)
                except subprocess.TimeoutExpired:process.kill();process.wait()
            output.close()

if __name__=='__main__':raise SystemExit(main())
