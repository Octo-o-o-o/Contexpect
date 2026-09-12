#!/usr/bin/env python3
"""Prepare, but never launch, a bounded synthetic Codex Effect experiment.

Creates its own local project/store and execution grants. It does not alter an
existing user policy, read authentication files, or invoke a model.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import urllib.parse
from codex_effect_runner import canonical, file_sha, write_new

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--codex', type=Path, required=True, help='native executable, not a shell wrapper')
    parser.add_argument('--output', type=Path, required=True, help='new, empty experiment directory')
    parser.add_argument('--model', required=True)
    parser.add_argument('--effort', choices=['low', 'medium', 'high', 'xhigh', 'max'], required=True)
    args = parser.parse_args()
    codex = args.codex.resolve(strict=True)
    with codex.open('rb') as file:
        if file.read(2) == b'#!':
            parser.error('--codex must pin the native executable behind any wrapper')
    version = subprocess.check_output([str(codex), '--version'], text=True, timeout=10).strip()
    if not version.startswith('codex-cli '):
        parser.error('the pinned tool is not a Codex CLI executable')
    out = args.output.resolve()
    if out.exists() and any(out.iterdir()):
        parser.error('output directory is not empty; a frozen experiment cannot be overwritten')
    out.mkdir(parents=True, exist_ok=True)
    engine = out/'engine.py'; suite_path = out/'suite.json'
    shutil.copyfile(ROOT/'scripts/codex_effect_runner.py', engine)
    shutil.copyfile(ROOT/'tests/effect/contract-context-suite-v1.json', suite_path)
    suite = json.loads(suite_path.read_text())
    project = out/'project'; project.mkdir()
    store = out/'store'
    for folder in ['policies', 'exceptions']:
        (store/folder).mkdir(parents=True)
    home = str(Path.home())
    environment = {'HOME': home, 'CODEX_HOME': os.environ.get('CODEX_HOME', str(Path(home)/'.codex'))}
    for key in ['http_proxy', 'https_proxy', 'all_proxy', 'no_proxy', 'NO_PROXY']:
        if key not in os.environ:
            continue
        value = os.environ[key]
        if key.lower() != 'no_proxy':
            parsed = urllib.parse.urlsplit(value)
            if parsed.username or parsed.password:
                parser.error('use a credential-free local proxy; no proxy credentials are copied')
        environment[key] = value
    files = {'README.md': 'Synthetic Contexpect contract reasoning evaluation. No tool use or source edits are part of the task.\n',
             'suite.lock.json': canonical(dict(suite_sha256=file_sha(suite_path), source_revision=suite['source_revision']))+'\n'}
    profile_path = out/'profile.json'
    profile = dict(schema='ctxpect-codex-effect-profile-v1', codex_path=str(codex), codex_sha256=file_sha(codex),
                   codex_version=version, model=args.model, effort=args.effort, suite_path=str(suite_path),
                   suite_sha256=file_sha(suite_path), evidence_root=str(out/'runs'), environment=environment,
                   call_timeout_seconds=260, input_files=sorted(files))
    write_new(profile_path, profile)
    launcher = out/'runner'
    launcher.write_text('#!/usr/bin/python3\nimport hashlib,importlib.util,pathlib\n'
        f'engine=pathlib.Path({str(engine)!r})\nprofile=pathlib.Path({str(profile_path)!r})\n'
        f'assert hashlib.sha256(engine.read_bytes()).hexdigest()=={file_sha(engine)!r},"engine drift"\n'
        f'assert hashlib.sha256(profile.read_bytes()).hexdigest()=={file_sha(profile_path)!r},"profile drift"\n'
        'spec=importlib.util.spec_from_file_location("ctxpect_codex_effect",engine)\n'
        'module=importlib.util.module_from_spec(spec)\nspec.loader.exec_module(module)\nmodule.main(profile)\n')
    launcher.chmod(0o700)
    frozen = int(time.time())
    contract = dict(schema='experiment-contract-v1', experiment_id=f'codex-contract-context-{frozen}',
                    primary_outcome='task-pass', margin_pp=10, pairing='paired-by-task', n_planned=8,
                    alpha='0.05', power='0.80', multiplicity='none', itt='count-as-fail',
                    locked=dict(code_digest=hashlib.sha256(canonical(files).encode()).hexdigest(),
                                model=f'{args.model}/{args.effort}', harness=f'{version}@{file_sha(codex)}',
                                tool_availability_digest=file_sha(launcher)),
                    frozen_at=f'{frozen}.000Z', invalidation=['model, effort, native protocol or isolation drift',
                    'harness/tool/source drift', 'sample count differs from 8 pairs', 'post-hoc task or golden editing'])
    request = dict(schema='ctxpect-command-runner-v1', contract=contract,
                   runner=dict(path=str(launcher), sha256=file_sha(launcher), version='ctxpect-command-runner-v1', sandbox='external-runner'),
                   files=files, budget=dict(max_runs=16, timeout_seconds=300, total_seconds=3600),
                   tasks=[dict(id=t['id'], control=canonical(dict(task_id=t['id'], context=False)),
                               treatment=canonical(dict(task_id=t['id'], context=True))) for t in suite['tasks']])
    write_new(out/'request.json', request)
    layers = ['organization', 'team', 'project', 'user', 'session']
    write_new(store/'policies/active.json', [dict(layer=l, mode='detect-only' if l in ['user', 'session'] else 'enforceable', rules=[]) for l in layers])
    for action in ['experiment.execute', 'experiment.persist']:
        id = action.replace('.', '-')
        write_new(store/f'exceptions/{id}.json', dict(exception_id=id, requester='local-experiment-owner', action=action,
                  project_digest=hashlib.sha256(f'project:{project}'.encode()).hexdigest(), target='*', state='approved',
                  created_at=frozen, expires_at=frozen+7200, reason='explicit bounded synthetic experiment preparation',
                  approver='local-experiment-only'))
    write_new(out/'preparation.json', dict(schema='ctxpect-live-effect-preparation-v1', request_sha256=file_sha(out/'request.json'),
              suite_sha256=file_sha(suite_path), engine_sha256=file_sha(engine), runner_sha256=file_sha(launcher),
              profile_sha256=file_sha(profile_path), frozen_at=contract['frozen_at'], model_invocations=0,
              primary='all four answers in a task must match the frozen golden',
              secondary='per-question correctness is descriptive only',
              scope=suite['scope'], design_power_assumption='benefit-only discordance 0.85 and independent pairs',
              power_under_assumption=0.8947872258203122, generic_10pp_power_established=False,
              launch_requires_explicit_model_budget_authorization=True))
    print(json.dumps(dict(prepared=True, model_invocations=0, pairs=8, maximum_cli_invocations=16,
                         request=str(out/'request.json'), project=str(project), store=str(store))))


if __name__ == '__main__':
    main()
