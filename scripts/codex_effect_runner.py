#!/usr/bin/env python3
"""External command-v1 adapter for a frozen Codex contract-reasoning experiment.

The product owns the attempt budget. This adapter owns the local Codex process,
synthetic prompts, deterministic answer gate, and evidence files. It does not
supply model observations from the requested configuration: it reads only the
newly created run's own turn_context and checks the native completion event.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

LIMIT = 8 * 1024 * 1024


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def read_json(path):
    path = Path(path)
    if path.is_symlink() or not path.is_file() or path.stat().st_size > LIMIT:
        raise ValueError('invalid local JSON input')
    return json.loads(path.read_text())


def write_new(path, value):
    with Path(path).open('x', encoding='utf8') as file:
        os.chmod(path, 0o600)
        file.write(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def prompt_for(task, with_context):
    # Expected answers and experiment arm labels are never given to the model.
    visible = [dict(id=q['id'], question=q['question'], options=q['options']) for q in task['questions']]
    material = task['contract_context'] if with_context else '本题没有额外项目合同摘录。'
    return ('你正在判断 Contexpect 软件的合同场景。只根据题面和提供的合同材料作答。'
            '不调用工具，不读取文件，不启动子代理。每题选择一个选项，'
            '仅返回满足 JSON schema 的 answers 对象，不加解释。\n\n'
            '合同材料：\n' + material + '\n\n题目：\n' + canonical(visible))


def answer_schema(task):
    ids = [q['id'] for q in task['questions']]
    return dict(type='object', additionalProperties=False, required=['answers'], properties={
        'answers': dict(type='object', additionalProperties=False, required=ids,
                        properties={id: dict(type='string', enum=list('ABCD')) for id in ids})})


def score_answers(text, task):
    expected = {q['id']: q['expected'] for q in task['questions']}
    try:
        parsed = json.loads(text)
        answers = parsed['answers']
        valid = (set(parsed) == {'answers'} and isinstance(answers, dict)
                 and set(answers) == set(expected) and all(v in list('ABCD') for v in answers.values()))
    except (ValueError, KeyError, TypeError):
        answers = {}; valid = False
    cases = [dict(id=id, correct=valid and answers.get(id) == answer,
                  expected=answer, answer=answers.get(id) if valid else None) for id, answer in expected.items()]
    return dict(format_valid=valid, cases=cases, correct=sum(c['correct'] for c in cases),
                total=len(cases), passed=valid and all(c['correct'] for c in cases))


def read_events(data):
    events = []
    for line in data.decode('utf8', errors='strict').splitlines():
        events.append(json.loads(line))
    started = [e.get('thread_id') for e in events if e.get('type') == 'thread.started']
    completed = [e for e in events if e.get('type') == 'turn.completed']
    failed = [e for e in events if e.get('type') in ['turn.failed', 'error']]
    messages = [e['item'].get('text', '') for e in events
                if e.get('type') == 'item.completed' and e.get('item', {}).get('type') == 'agent_message']
    tools = [e.get('item', {}).get('type') for e in events
             if e.get('type') == 'item.started' and e.get('item', {}).get('type')
             in ['command_execution', 'mcp_tool_call', 'web_search', 'file_change', 'collab_tool_call']]
    if len(started) != 1 or not re.fullmatch(r'[0-9a-f-]{36}', str(started[0])):
        raise ValueError('native thread identity missing')
    return dict(thread_id=started[0], completed=len(completed) == 1 and not failed,
                text=messages[-1] if messages else '', tool_events=tools,
                usage=completed[-1].get('usage', {}) if completed else {})


def own_turn_context(codex_home, thread_id):
    # Only files whose name contains this invocation's returned UUID are read.
    # No other user session bodies or search index are accessed.
    sessions = Path(codex_home) / 'sessions'
    matches = list(sessions.glob(f'*/*/*/*{thread_id}*.jsonl'))
    if len(matches) != 1:
        raise ValueError('own native rollout not found uniquely')
    result = None
    with matches[0].open() as file:
        for line in file:
            row = json.loads(line)
            if row.get('type') == 'turn_context':
                value = row.get('payload', {})
                result = dict(model=value.get('model'), effort=value.get('effort'),
                              sandbox_policy=value.get('sandbox_policy'), approval_policy=value.get('approval_policy'))
    if result is None:
        raise ValueError('native turn_context missing')
    return result


def execute(profile, request):
    if request.get('schema') != 'ctxpect-runner-input-v1':
        raise ValueError('runner input schema mismatch')
    run_id = request['run_id']
    if not re.fullmatch(r'run-[0-9]+-(control|treatment)', run_id):
        raise ValueError('unsafe run id')
    evidence = Path(profile['evidence_root'])
    evidence.mkdir(parents=True, exist_ok=True)
    run_dir = evidence / run_id
    run_dir.mkdir()  # An existing attempt must never be launched a second time.
    stop_file = evidence / 'adapter-stopped.json'
    if stop_file.exists():
        raise ValueError('earlier native failure stopped new model invocations')
    if file_sha(profile['codex_path']) != profile['codex_sha256']:
        raise ValueError('Codex binary drift')
    if file_sha(profile['suite_path']) != profile['suite_sha256']:
        raise ValueError('suite drift')
    suite = read_json(profile['suite_path'])
    task = next(t for t in suite['tasks'] if t['id'] == request['task_id'])
    selection = json.loads(request['input'])
    if selection != dict(task_id=task['id'], context=request['arm'] == 'treatment'):
        raise ValueError('input/arm mismatch')
    files = {name: Path(name).read_text() for name in profile['input_files']}
    code_digest = sha(canonical(files).encode())
    if code_digest != request['contract']['locked']['code_digest']:
        raise ValueError('initial source drift')
    schema_path = run_dir / 'answer-schema.json'
    write_new(schema_path, answer_schema(task))
    prompt = prompt_for(task, selection['context'])
    write_new(run_dir / 'started.json', dict(run_id=run_id, task_id=task['id'], arm=request['arm'],
              request_digest=request['request_digest'], prompt_sha256=sha(prompt.encode()),
              suite_sha256=profile['suite_sha256'], started_at=time.time(), state='starting',
              model=profile['model'], effort=profile['effort'], actual_provider_requests='not-yet-observed'))
    env = {**profile['environment'], 'PATH': '/usr/bin:/bin', 'LANG': 'C', 'NO_COLOR': '1'}
    version = subprocess.run([profile['codex_path'], '--version'], env=env, capture_output=True, timeout=10)
    actual_version = version.stdout.decode().strip()
    if version.returncode or actual_version != profile['codex_version']:
        raise ValueError('Codex version drift')
    command = [profile['codex_path'], '-a', 'never', 'exec', '--ignore-user-config',
               '-s', 'read-only', '--skip-git-repo-check', '-C', str(Path.cwd()),
               '-m', profile['model'], '-c', f'model_reasoning_effort="{profile["effort"]}"',
               '-c', 'project_doc_max_bytes=0', '-c', 'web_search="disabled"',
               '-c', 'features.multi_agent=false', '--output-schema', str(schema_path),
               '--color', 'never', '--json', prompt]
    started = time.monotonic()
    process = subprocess.Popen(command, env=env, stdin=subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    write_new(run_dir / 'process.json', dict(pid=process.pid, launched_at=time.time(),
              model=profile['model'], effort=profile['effort'], native_cli_invocations=1))
    try:
        stdout, stderr = process.communicate(timeout=profile['call_timeout_seconds'])
        timed_out = False
    except subprocess.TimeoutExpired:
        process.kill(); stdout, stderr = process.communicate(); timed_out = True
    # Raw output is transient. The persisted gate retains only four answer labels,
    # aggregate usage, digests, and this newly created run's native identity.
    native = read_events(stdout) if len(stdout) <= LIMIT else None
    if native is None:
        raise ValueError('native output exceeds bound')
    context = own_turn_context(profile['environment']['CODEX_HOME'], native['thread_id'])
    sandbox = context.get('sandbox_policy')
    sandbox_type = sandbox.get('type') if isinstance(sandbox, dict) else sandbox
    identity_valid = context['model'] == profile['model'] and context['effort'] == profile['effort']
    isolation_valid = sandbox_type == 'read-only' and context.get('approval_policy') == 'never'
    normal = process.returncode == 0 and native['completed'] and not timed_out
    gate = score_answers(native['text'], task)
    immutable_files = {name: Path(name).read_text() for name in profile['input_files']}
    unchanged = sha(canonical(immutable_files).encode()) == code_digest
    protocol_valid = identity_valid and isolation_valid and not native['tool_events'] and unchanged
    outcome = ('timeout' if timed_out else 'crash' if not normal else
               'pass' if gate['passed'] and protocol_valid else 'fail')
    receipt = dict(schema='ctxpect-native-effect-gate-v1', run_id=run_id,
                   task_id=task['id'], arm=request['arm'], request_digest=request['request_digest'],
                   suite_sha256=profile['suite_sha256'], prompt_sha256=sha(prompt.encode()),
                   codex_sha256=profile['codex_sha256'], native= {k:v for k,v in native.items() if k!='text'},
                   observed_context=context, identity_valid=identity_valid, isolation_valid=isolation_valid,
                   source_unchanged=unchanged, protocol_valid=protocol_valid,
                   stdout_sha256=sha(stdout), stderr_sha256=sha(stderr), process_exit=process.returncode,
                   timed_out=timed_out, elapsed_seconds=round(time.monotonic()-started,3), outcome=outcome,
                   gate=gate, raw_provider_response_saved_by_adapter=False,
                   native_rollout_kind='only-this-new-synthetic-invocation',
                   provider_request_count='not-exposed-by-cli', finished_at=time.time())
    write_new(run_dir / 'gate.json', receipt)
    if not normal or not protocol_valid:
        write_new(stop_file, dict(run_id=run_id, reason='native completion, identity or isolation failure'))
    observed = dict(code_digest=code_digest if unchanged else 'source-changed',
                    model=f'{context["model"]}/{context["effort"]}',
                    harness=f'{actual_version}@{profile["codex_sha256"]}' if protocol_valid else 'native-protocol-invalid',
                    tool_availability_digest=file_sha(sys.argv[0]))
    return dict(schema='ctxpect-runner-result-v1', run_id=run_id, request_digest=request['request_digest'],
                sandbox='external-runner', observed=observed, outcome=outcome,
                gate_evidence_digest=file_sha(run_dir / 'gate.json'))


def main(profile_path=None):
    parser = argparse.ArgumentParser()
    parser.add_argument('--profile', type=Path)
    args = parser.parse_args([] if profile_path else None)
    profile = read_json(profile_path or args.profile)
    request = json.loads(sys.stdin.buffer.read(LIMIT + 1))
    try:
        result = execute(profile, request)
    except Exception as error:
        # Do not silently continue paid calls after a protocol/identity failure.
        root = Path(profile['evidence_root']); root.mkdir(parents=True, exist_ok=True)
        stop = root / 'adapter-stopped.json'
        if not stop.exists():
            write_new(stop, dict(error_type=type(error).__name__, reason=str(error)[:300]))
        result = dict(schema='ctxpect-runner-result-v1', run_id=request.get('run_id'),
                      request_digest=request.get('request_digest'), sandbox='external-runner',
                      outcome='refusal', observed=dict(code_digest='unverified', model='unverified',
                      harness='unverified', tool_availability_digest='unverified'))
    print(canonical(result))


if __name__ == '__main__':
    main()
