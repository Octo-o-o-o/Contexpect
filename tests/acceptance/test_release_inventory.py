"""Release evidence identity must fail closed even when a receipt says pass."""
import hashlib
import importlib.util
import pathlib
import sys
import tempfile
import unittest

ROOT=pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts'))
spec=importlib.util.spec_from_file_location('release_inventory',ROOT/'scripts/release_inventory.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


class ReleaseInventoryTests(unittest.TestCase):
    def test_all_canonical_surfaces_are_enumerated(self):
        rows=module.requirements()
        ids={r['id'] for r in rows}
        self.assertEqual(len(rows),len(ids))
        self.assertGreater(sum(r['kind']=='capability' for r in rows),1000)
        self.assertEqual(sum(r['kind']=='software-gate' for r in rows),16)
        self.assertIn('integration/qa-eight-participants-six-tasks',ids)
        self.assertIn('lane/windows-11-24h2-x86_64/soak-72h-1000-changes',ids)

    def test_file_identity_is_not_semantic_or_independent_acceptance(self):
        with tempfile.TemporaryDirectory() as td:
            root=pathlib.Path(td);(root/'receipt').write_bytes(b'pass')
            candidate={'head':'candidate'}
            required=[dict(id='a',kind='test',build='frozen',os_lane='lane')]
            item=dict(id='a',artifact='receipt',sha256=hashlib.sha256(b'pass').hexdigest(),candidate=candidate,os_build='different',os_lane='lane',verdict='pass')
            evidence=dict(schema='ctxpect-release-evidence-v1',candidate=candidate,items=[item])
            report=module.evaluate(required,evidence,root,candidate)
            self.assertFalse(report['evidence_inventory_complete'])
            self.assertEqual(report['requirements'][0]['reason'],'OS build mismatch')
            item['os_build']='frozen'
            report=module.evaluate(required,evidence,root,candidate)
            self.assertTrue(report['evidence_inventory_complete'])
            self.assertFalse(report['full_product_accepted'])
            (root/'receipt').write_bytes(b'changed')
            self.assertFalse(module.evaluate(required,evidence,root,candidate)['evidence_inventory_complete'])

    def test_missing_duplicate_unknown_and_escape_do_not_complete_inventory(self):
        with tempfile.TemporaryDirectory() as td:
            root=pathlib.Path(td);candidate={};required=[dict(id='a',kind='test')]
            evidence=dict(schema='ctxpect-release-evidence-v1',candidate=candidate,items=[])
            self.assertFalse(module.evaluate(required,evidence,root,candidate)['evidence_inventory_complete'])
            item=dict(id='a',artifact='../elsewhere',sha256='0'*64,candidate=candidate,verdict='pass')
            evidence['items']=[item]
            self.assertFalse(module.evaluate(required,evidence,root,candidate)['evidence_inventory_complete'])
            evidence['items']=[item,item]
            with self.assertRaises(ValueError):module.evaluate(required,evidence,root,candidate)
            evidence['items']=[dict(item,id='made-up')]
            with self.assertRaises(ValueError):module.evaluate(required,evidence,root,candidate)
