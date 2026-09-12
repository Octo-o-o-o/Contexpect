#!/usr/bin/env python3
"""Build an unsigned local CLI archive and smoke its disposable install lifecycle.

No global install or production updater: upgrade here means side-by-side candidate
replacement followed by rollback. This cannot certify cross-version data migration.
"""
import argparse
import hashlib
import io
import json
import pathlib
import platform
import shutil
import subprocess
import tarfile
import tempfile
from release_inventory import candidate_identity

ROOT = pathlib.Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def probe(binary, project, store):
    run = subprocess.run([str(binary), 'inspect', '--project', str(project), '--store', str(store), '--json'],
                         stdin=subprocess.DEVNULL, capture_output=True, timeout=30)
    value = json.loads(run.stdout)
    if run.returncode != 0 or not isinstance(value, dict):
        raise RuntimeError('installed candidate inspect failed')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin', type=pathlib.Path, default=ROOT/'target/debug/ctxpect')
    parser.add_argument('--output', required=True, type=pathlib.Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    binary = args.bin.resolve().read_bytes()
    files = {'bin/ctxpect': binary, 'LICENSE': (ROOT/'LICENSE').read_bytes()}
    manifest = dict(schema='ctxpect-unsigned-cli-candidate-v1', candidate=candidate_identity(),
                    host=dict(system=platform.system(), machine=platform.machine()), signed=False,
                    build_profile='caller-supplied-binary', source_build_verified=False, files={p:digest(b) for p,b in files.items()})
    files['manifest.json'] = (json.dumps(manifest, indent=2)+'\n').encode()
    archive = args.output/'ctxpect-cli-candidate.tar.gz'
    # Refuse to overwrite an earlier candidate artifact.
    with archive.open('xb') as output, tarfile.open(fileobj=output, mode='w:gz') as tar:
        for name, data in files.items():
            member = tarfile.TarInfo(name); member.size=len(data)
            member.mode=0o755 if name.startswith('bin/') else 0o644
            tar.addfile(member, io.BytesIO(data))
    checks=[]
    with tempfile.TemporaryDirectory(prefix='ctxpect-install-smoke-') as td:
        root=pathlib.Path(td); project=root/'project'; project.mkdir()
        source=project/'AGENTS.md'; source.write_text('unmanaged native instructions\n')
        original=source.read_bytes(); store=root/'user-store'
        versions=[]
        for version in ['candidate-a','candidate-b']:
            prefix=root/version; prefix.mkdir()
            # The exact bounded member set is validated before any extraction.
            with tarfile.open(archive) as tar:
                if sorted(m.name for m in tar)!=sorted(files):raise RuntimeError('unexpected archive member')
                for member in tar.getmembers():
                    if not member.isfile():raise RuntimeError('non-regular archive member')
                    data=tar.extractfile(member).read()
                    if data!=files[member.name]:raise RuntimeError('archive integrity mismatch')
                    target=prefix/member.name; target.parent.mkdir(parents=True,exist_ok=True)
                    target.write_bytes(data); target.chmod(member.mode)
            probe(prefix/'bin/ctxpect',project,store); versions.append(prefix)
            checks.append('temporary install '+version)
        # Both versions use the same candidate: an updater/data migration is not tested.
        probe(versions[0]/'bin/ctxpect',project,store); checks.append('side-by-side rollback smoke')
        for prefix in versions:shutil.rmtree(prefix)
        if source.read_bytes()!=original or not store.exists():raise RuntimeError('uninstall altered user data')
        checks.append('remove owned prefixes preserves project and store')
    report=dict(schema='ctxpect-cli-package-smoke-v1', archive_sha256=digest(archive.read_bytes()),
                binary_sha256=digest(binary), candidate=manifest['candidate'], checks=checks,
                signed=False, full_release=False, cross_version_migration='not-tested')
    (args.output/'smoke.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))


if __name__=='__main__':main()
