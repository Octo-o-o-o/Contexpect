#!/usr/bin/env python3
"""Live age/SSHSIG integration using two disposable local profiles, never user keys."""
import argparse
import copy
import hashlib
import json
import pathlib
import subprocess
import tempfile
import time


def sha(data):
    return hashlib.sha256(data).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value))
    return path


def run(argv, *, data=None):
    return subprocess.run([str(a) for a in argv], input=data, capture_output=True, timeout=30)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--age', required=True, type=pathlib.Path)
    parser.add_argument('--age-keygen', required=True, type=pathlib.Path)
    parser.add_argument('--ssh-keygen', default='/usr/bin/ssh-keygen', type=pathlib.Path)
    parser.add_argument('--bin', default='target/debug/ctxpect', type=pathlib.Path)
    args = parser.parse_args()
    binary, age, keygen, ssh = (p.resolve() for p in (args.bin, args.age, args.age_keygen, args.ssh_keygen))
    checks = []
    with tempfile.TemporaryDirectory(prefix='ctxpect-secure-sync-') as td:
        root = pathlib.Path(td).resolve()
        project = root / 'project'; project.mkdir()
        recipients = {}
        for name in ['a', 'b', 'outsider']:
            key = root / f'{name}.agekey'
            assert run([keygen, '-o', key]).returncode == 0
            public = run([keygen, '-y', key]); assert public.returncode == 0
            recipients[name] = public.stdout.decode().strip()
            assert run([ssh, '-q', '-t', 'ed25519', '-N', '', '-C', 'test-only', '-f', root / f'{name}.ssh']).returncode == 0
        allowed = root / 'allowed'
        allowed.write_text(''.join(f'{n} {(root / (n + ".ssh.pub")).read_text()}' for n in ['a', 'b']))
        revoked = root / 'revoked'; revoked.write_text('')
        base = dict(schema='ctxpect-age-ssh-profile-v1', group='test-group', epoch=1,
                    trust_valid_until=int(time.time()) + 3600,
                    recipients=sorted([recipients['a'], recipients['b']]),
                    signers={n: recipients[n] for n in ['a', 'b']},
                    age={'path': str(age), 'sha256': sha(age.read_bytes())},
                    ssh_keygen={'path': str(ssh), 'sha256': sha(ssh.read_bytes())},
                    allowed_signers=str(allowed), revoked_signers=str(revoked))
        profiles = {}
        stores = {}
        for name in ['a', 'b']:
            profile = dict(base, sender=name, identity=str(root / f'{name}.agekey'), signing_key=str(root / f'{name}.ssh'))
            profiles[name] = write(root / f'{name}.json', profile)
            store = root / f'store-{name}'; stores[name] = store
            (store / 'policies').mkdir(parents=True); (store / 'exceptions').mkdir()
            write(store / 'policies/active.json', [dict(layer=l, mode='detect-only' if l in ['user','session'] else 'enforceable', rules=[]) for l in ['organization','team','project','user','session']])
            for action in ['sync.seal', 'sync.apply']:
                write(store / f"exceptions/{action.replace('.', '-')}.json", dict(exception_id=action.replace('.', '-'), requester='test', action=action,
                    project_digest=sha(f'project:{project}'.encode()), target='*', state='approved', created_at=1,
                    expires_at=4102444800, reason='disposable integration grant', approver='test-only'))
        payload = {'schema': 'ctxpect-sync-assets-v1', 'assets': [dict(id='instruction', kind='instruction', classification='device-group', content='SYNC_TEST_CANARY arithmetic only') ]}
        payload_path = write(root / 'payload.json', payload)
        def call(name, action, source=None, more=(), profile=None):
            argv = [binary, 'sync', action, '--adapter', 'age-ssh-v1', '--profile', profile or profiles[name], '--project', project, '--store', stores[name], '--json']
            if source: argv += ['--from', source]
            proc = run(argv + list(more))
            result = json.loads(proc.stdout)
            return proc.returncode, result
        def succeeds(result, label):
            code, doc = result
            assert code == 0, (label, code, doc)
            checks.append(label)
            return doc
        def rejects(result, reason):
            code, doc = result
            assert code != 0 and doc.get('error', {}).get('code') == reason, (reason, code, doc)
            checks.append(reason)
        packet = root / 'packet.json'
        succeeds(call('a', 'seal', payload_path, ['--dest', packet]), 'real age encrypt and SSH sign')
        assert b'SYNC_TEST_CANARY' not in packet.read_bytes(); checks.append('transport contains no plaintext canary')
        preview = succeeds(call('b', 'preview', packet), 'other profile decrypt and verify')
        rejects(call('b', 'apply', packet), 'sync.preview_required')
        succeeds(call('b', 'apply', packet, ['--tx', preview['tx_id']]), 'frozen preview apply')
        state = json.loads((stores['b'] / 'secureheads/test-group.json').read_text())
        ciphertext=state['document']['envelope']['ciphertext'].encode()
        decrypted=run([age,'--decrypt','--identity',root/'b.agekey'],data=ciphertext)
        assert decrypted.returncode==0 and json.loads(decrypted.stdout)==payload
        assert 'SYNC_TEST_CANARY' not in json.dumps(state)
        checks.append('exact received bytes decrypt externally; local store has no plaintext')
        rejects(call('b', 'preview', packet), 'sync.replay')
        fork = root / 'fork.json'
        succeeds(call('a', 'seal', payload_path, ['--dest', fork]), 'same-parent independently encrypted export')
        rejects(call('b', 'preview', fork), 'sync.fork')
        # Advance A after its own export, then make the second generation.
        preview_a = succeeds(call('a', 'preview', packet), 'sender local verification')
        succeeds(call('a', 'apply', packet, ['--tx', preview_a['tx_id']]), 'sender head advance')
        next_packet = root / 'next.json'
        succeeds(call('a', 'seal', payload_path, ['--dest', next_packet]), 'next generation')
        p = succeeds(call('b', 'preview', next_packet), 'next generation preview')
        # Expiration is a durable field, not a process-local timer.
        frozen_path = stores['b'] / f"securepreviews/{p['tx_id']}.json"
        frozen_bytes = frozen_path.read_bytes()
        expired = json.loads(frozen_bytes); expired['expires_at'] = 1
        write(frozen_path, expired)
        rejects(call('b', 'apply', next_packet, ['--tx', p['tx_id']]), 'sync.preview_changed')
        frozen_path.write_bytes(frozen_bytes)
        stable_head = (stores['b'] / 'secureheads/test-group.json').read_bytes()
        changed = copy.deepcopy(base); changed.update(sender='b', identity=str(root/'b.agekey'), signing_key=str(root/'b.ssh'), trust_valid_until=int(time.time())+7200)
        changed_path = write(root/'changed-profile.json', changed)
        rejects(call('b', 'apply', next_packet, ['--tx', p['tx_id']], changed_path), 'sync.preview_changed')
        tampered = json.loads(next_packet.read_text()); tampered['envelope']['generation'] += 1
        rejects(call('b', 'preview', write(root/'tampered.json', tampered)), 'sync.signature_untrusted')
        cipher_bad = json.loads(next_packet.read_text()); cipher_bad['envelope']['ciphertext'] += 'x'
        rejects(call('b', 'preview', write(root/'cipher-bad.json', cipher_bad)), 'sync.cipher_digest_mismatch')
        outsider = dict(changed, identity=str(root/'outsider.agekey'))
        rejects(call('b', 'preview', next_packet, profile=write(root/'outsider.json', outsider)), 'sync.decrypt_failed')
        revoked.write_text((root/'a.ssh.pub').read_text())
        rejects(call('b', 'preview', next_packet), 'sync.signature_untrusted')
        revoked.write_text('')
        removed = dict(changed, recipients=[recipients['b']], signers={'b':recipients['b']}, epoch=2)
        rejects(call('b', 'preview', next_packet, profile=write(root/'removed.json', removed)), 'sync.recipient_epoch_mismatch')
        stale = dict(changed, trust_valid_until=1)
        rejects(call('b', 'preview', next_packet, profile=write(root/'stale.json', stale)), 'sync.trust_stale')
        drift = copy.deepcopy(changed); drift['age']['sha256'] = '0'*64
        rejects(call('b', 'preview', next_packet, profile=write(root/'drift.json', drift)), 'sync.tool_drift')
        secret = copy.deepcopy(payload); secret['assets'][0]['content'] = '-----BEGIN PRIVATE KEY-----\nTEST\n-----END PRIVATE KEY-----'
        rejects(call('a','seal',write(root/'secret.json',secret),['--dest',root/'forbidden.json']), 'sync.secret_in_bundle')
        forbidden = copy.deepcopy(payload); forbidden['assets'][0]['classification']='never-sync'
        rejects(call('a','seal',write(root/'forbidden-input.json',forbidden),['--dest',root/'forbidden.json']), 'sync.asset_not_allowed')
        assert not (root/'forbidden.json').exists()
        assert (stores['b'] / 'secureheads/test-group.json').read_bytes() == stable_head
        checks.append('rejected packets preserve accepted encrypted head')
        succeeds(call('b','apply',next_packet,['--tx',p['tx_id']]), 'unchanged preview remains applicable after rejected attempts')
        rejects(call('b','preview',packet), 'sync.rollback')
        # A removed recipient must fail cryptographic decryption of FUTURE bytes.
        future_packet = root / 'future.json'
        removed_profile = root / 'removed.json'
        succeeds(call('b', 'seal', payload_path, ['--dest', future_packet], removed_profile), 'epoch rotation seals future bundle')
        future_cipher = json.loads(future_packet.read_text())['envelope']['ciphertext'].encode()
        assert run([age, '--decrypt', '--identity', root/'a.agekey'], data=future_cipher).returncode != 0
        assert run([age, '--decrypt', '--identity', root/'b.agekey'], data=future_cipher).returncode == 0
        checks.append('removed recipient cannot decrypt future bundle; remaining recipient can')
        print(json.dumps({'scope':'two-disposable-local-profiles-real-age-ssh', 'passed':len(checks), 'checks':checks, 'native_projection':'not-applied','human_acceptance':False}))

if __name__ == '__main__':
    main()
