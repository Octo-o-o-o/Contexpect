#!/usr/bin/env python3
"""Development coordinate / protocol tests. Native success requires explicit --native-tool."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

P = argparse.ArgumentParser()
P.add_argument('--bin', type=Path, default=Path('target/debug/ctxpect'))
P.add_argument('--native-tool', type=Path)
P.add_argument('--output', type=Path)
ARGS = P.parse_args()
BIN = ARGS.bin.resolve()


class Integration(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='ctx-integration-')
        self.root = Path(self.tmp.name).resolve()
        self.project = self.root / 'project'
        self.project.mkdir()
        (self.project / '.git').mkdir()
        (self.project / 'AGENTS.md').write_text('Use red marker.\n')
        tool = ARGS.native_tool or BIN
        self.req = dict(schema_major=1, action='inspect', request_id='a', run_id='a', project_id='p',
                        project=str(self.project), cwd='.', store=str(self.root / 'store'),
                        profile='isolated-default-trusted-instructions-v1', version='0.153.3',
                        os_lane='macos-27-arm64', native=False,
                        harness_tool={'path': str(tool.resolve()), 'sha256': hashlib.sha256(tool.read_bytes()).hexdigest()})

    def tearDown(self):
        self.tmp.cleanup()

    def call(self, **updates):
        r = dict(self.req, **updates)
        x = subprocess.run([str(BIN), 'integration'], input=json.dumps(r), text=True, capture_output=True, timeout=45)
        return x.returncode, json.loads(x.stdout)

    def test_current_and_receipt_reuse(self):
        code, a = self.call()
        self.assertEqual(code, 0)
        self.assertEqual(a['diagnostic_exit'], 0)
        self.assertEqual(a['formal_receipt']['document']['schema'], 'ctxpect-receipt-v1')
        _, b = self.call(run_id='b', request_id='b')
        self.assertEqual(a['formal_receipt'], b['formal_receipt'])
        self.assertEqual(a['input_digest'], b['input_digest'])
        self.assertEqual(a['native']['status'], 'Unknown')

    def test_override_explains_difference(self):
        _, a = self.call()
        (self.project / 'AGENTS.override.md').write_text('Use blue marker.\n')
        _, b = self.call()
        self.assertNotEqual(a['input_digest'], b['input_digest'])
        self.assertNotEqual(a['formal_receipt']['id'], b['formal_receipt']['id'])
        self.assertIn('overridden-by', json.dumps(b['diagnostic']))

    def test_unknown_version_and_os_are_not_pass(self):
        for delta in [dict(version='0.153.4'), dict(os_lane='windows-11-x86_64')]:
            code, result = self.call(**delta)
            self.assertEqual(code, 0)
            self.assertEqual(result['diagnostic_exit'], 3)
            self.assertTrue(result['diagnostic']['unknown'])

    def test_profile_schema_cwd_and_tool_refusal(self):
        for delta in [dict(profile='real-user'), dict(schema_major=2), dict(cwd='..'), dict(store=str(self.project / 'store')),
                      dict(harness_tool={'path': str(BIN), 'sha256': '0'*64})]:
            code, result = self.call(**delta)
            self.assertEqual(code, 1)
            self.assertIn('error', result)

    def test_symlink_and_fifo_refused_without_reading(self):
        (self.project / 'AGENTS.md').unlink()
        (self.project / 'AGENTS.md').symlink_to('/etc/passwd')
        self.assertEqual(self.call()[1]['error'], 'integration.symlink_refused')
        (self.project / 'AGENTS.md').unlink()
        import os
        os.mkfifo(self.project / 'AGENTS.md')
        self.assertEqual(self.call()[1]['error'], 'integration.input_not_regular')

    def test_empty_override_and_nearest_git_root(self):
        (self.project / 'AGENTS.override.md').write_text('  \n')
        self.assertEqual(self.call()[1]['diagnostic_exit'], 2)
        sub = self.project / 'sub'
        sub.mkdir()
        (sub / '.git').mkdir()
        self.assertEqual(self.call(cwd='sub')[1]['diagnostic_exit'], 2)
        (sub / 'AGENTS.md').write_text('nested')
        self.assertEqual(self.call(cwd='sub')[1]['diagnostic_exit'], 0)

    def test_input_limit_bad_json_and_no_body_in_output(self):
        for raw in ['{', ' ' * 65537]:
            x = subprocess.run([str(BIN), 'integration'], input=raw, text=True, capture_output=True, timeout=5)
            self.assertEqual(x.returncode, 1)
        self.assertNotIn('Use red marker.', json.dumps(self.call()[1]))
        self.assertNotIn(str(self.root), json.dumps(self.call()[1]))

    def test_drifting_scan_stops_after_bounded_retry(self):
        import threading
        import time
        tool = self.root / 'synthetic-tool'
        tool.write_text('#!/bin/sh\nif [ "$1" = "--version" ]; then echo "codex-cli 0.153.3"; else sleep 0.3; echo "[]"; fi\n')
        tool.chmod(0o700)
        stop = threading.Event()
        def edit():
            i = 0
            while not stop.wait(.005):
                i += 1
                (self.project / 'AGENTS.md').write_text('drift-' + str(i))
        worker = threading.Thread(target=edit)
        worker.start()
        try:
            code, result = self.call(native=True, harness_tool={'path': str(tool), 'sha256': hashlib.sha256(tool.read_bytes()).hexdigest()})
            self.assertEqual(code, 1)
            self.assertEqual(result['error'], 'integration.unstable')
        finally:
            stop.set()
            worker.join()

    @unittest.skipUnless(ARGS.native_tool, 'explicit native tool not supplied')
    def test_actual_native_isolated_diagnostic(self):
        before = (self.project / 'AGENTS.md').read_bytes()
        code, result = self.call(native=True)
        self.assertEqual(code, 0)
        self.assertEqual(result['native']['status'], 'observed', result['native'])
        self.assertFalse(result['native']['model_executed'])
        self.assertFalse(result['native']['provider_wire_payload'])
        self.assertEqual(before, (self.project / 'AGENTS.md').read_bytes())
        if ARGS.output:
            ARGS.output.write_text(json.dumps(result, indent=2))


unittest.main(argv=['integration'], verbosity=2)
