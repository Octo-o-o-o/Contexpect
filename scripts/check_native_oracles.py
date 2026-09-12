#!/usr/bin/env python3
"""Exercise installed oracles on disposable synthetic projects, without account state."""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import shutil
import subprocess
import tempfile


def invoke(argv, cwd, env):
    try:
        result = subprocess.run(argv, cwd=cwd, env=env, input=b'', capture_output=True, timeout=25)
        try:
            parsed = json.loads(result.stdout)
        except (ValueError, UnicodeDecodeError):
            parsed = None
        return result.returncode, parsed, hashlib.sha256(result.stdout).hexdigest()
    except subprocess.TimeoutExpired:
        return 124, None, None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--codex', type=pathlib.Path)
    parser.add_argument('--grok', type=pathlib.Path)
    args = parser.parse_args()
    report = {'schema': 'ctxpect-native-probe-report-v1', 'synthetic_input': True,
              'actual_harness': True, 'account_state_read': False, 'os': platform.platform(), 'cases': []}
    for family, expected, command in [('grok', '1.0.13', ['inspect', '--json']), ('codex', '0.147.0', ['debug', 'prompt-input', 'CTXPECT_USER_CANARY_73'])]:
        override = getattr(args, family)
        executable = str(override.resolve()) if override else shutil.which(family)
        if not executable:
            report['cases'].append({'family': family, 'status': 'not-installed'}); continue
        version = subprocess.run([executable, '--version'], capture_output=True, text=True, timeout=10).stdout.strip()
        frozen_version_matches = version.split()[1] == expected
        with tempfile.TemporaryDirectory(prefix='ctxpect-oracle-') as td:
            root = pathlib.Path(td); project = root/'project'; project.mkdir()
            profile = root/'profile'; profile.mkdir(); (profile/'codex').mkdir()
            env = {'PATH': '/usr/bin:/bin', 'HOME': str(profile), 'CODEX_HOME': str(profile/'codex'),
                   'GROK_HOME': str(profile/'grok'), 'XDG_CONFIG_HOME': str(profile/'config'), 'LANG': 'en_US.UTF-8'}
            for case, content in [('present', 'CTXPECT_ROOT_CANARY_73: use exact arithmetic.\n'), ('changed', 'CTXPECT_CHANGED_CANARY_73: preserve input order.\n'), ('missing', None)]:
                source = project/'AGENTS.md'
                if content is None:
                    source.unlink()
                else:
                    source.write_text(content)
                code, data, digest = invoke([executable, *command], project, env)
                item = dict(family=family, installed_version=version, frozen_version_matches=frozen_version_matches,
                            case=case, exit_code=code, output_digest=digest, native_claim_upgraded=False)
                if code != 0 or data is None:
                    item.update(status='failed', reason='oracle invocation or JSON failed')
                elif family == 'grok':
                    instructions = data.get('projectInstructions', [])
                    matches = [x for x in instructions if pathlib.Path(x.get('path', '')).name.lower() == 'agents.md']
                    passed = len(matches) == (0 if content is None else 1)
                    if content is not None and matches:
                        passed &= matches[0].get('sizeBytes') == len(content.encode())
                    item.update(status='pass' if passed else 'failed', observed_facet='instruction discovery metadata',
                                body_observed=False, item_count=len(matches))
                else:
                    blob = json.dumps(data)
                    marker = 'CTXPECT_CHANGED_CANARY_73' if case == 'changed' else 'CTXPECT_ROOT_CANARY_73'
                    passed = ('CTXPECT_USER_CANARY_73' in blob and ((marker in blob) == (content is not None)))
                    if case != 'present':
                        passed &= 'CTXPECT_ROOT_CANARY_73' not in blob
                    item.update(status='pass' if passed else 'failed', observed_facet='debug prompt input',
                                body_observed=content is not None, executed_model=False)
                report['cases'].append(item)
    report['full_frozen_matrix_passed'] = False
    report['reason'] = 'bounded native probe; not all frozen coordinates/capabilities exercised'
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))
    return 1 if any(c['status']=='failed' for c in report['cases']) else 0

if __name__ == '__main__':
    raise SystemExit(main())
