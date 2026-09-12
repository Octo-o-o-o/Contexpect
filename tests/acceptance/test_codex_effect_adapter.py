"""Zero-provider-call checks of the external live-evaluation adapter contract."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('codex_effect_runner', ROOT/'scripts/codex_effect_runner.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)
SUITE = json.loads((ROOT/'tests/effect/contract-context-suite-v1.json').read_text())


class LiveEffectAdapterTests(unittest.TestCase):
    def test_all_eight_tasks_have_a_nontrivial_exact_gate(self):
        self.assertEqual(len(SUITE['tasks']), 8)
        self.assertEqual(len({t['id'] for t in SUITE['tasks']}), 8)
        for task in SUITE['tasks']:
            self.assertEqual(len(task['questions']), 4)
            answers = {q['id']: q['expected'] for q in task['questions']}
            self.assertTrue(adapter.score_answers(json.dumps({'answers': answers}), task)['passed'])
            for id, correct in answers.items():
                bad = dict(answers); bad[id] = next(c for c in 'ABCD' if c != correct)
                score = adapter.score_answers(json.dumps({'answers': bad}), task)
                self.assertFalse(score['passed'])
                self.assertEqual(score['correct'], 3)
            extra = dict(answers, unregistered='A')
            self.assertFalse(adapter.score_answers(json.dumps({'answers': extra}), task)['passed'])
            self.assertFalse(adapter.score_answers('not JSON', task)['passed'])

    def test_prompts_exclude_golden_answers_and_differ_only_by_context(self):
        for task in SUITE['tasks']:
            control = adapter.prompt_for(task, False)
            treatment = adapter.prompt_for(task, True)
            self.assertNotIn('"expected"', control + treatment)
            self.assertNotIn('"source_revision"', control + treatment)
            self.assertNotIn(task['contract_context'], control)
            self.assertEqual(treatment.replace(task['contract_context'], '本题没有额外项目合同摘录。'), control)

    def test_native_completion_is_required_and_usage_is_not_invented(self):
        id = '11111111-1111-1111-1111-111111111111'
        events = [dict(type='thread.started', thread_id=id),
                  dict(type='item.completed', item=dict(type='agent_message', text='{"answers":{}}'))]
        self.assertFalse(adapter.read_events(('\n'.join(map(json.dumps, events))).encode())['completed'])
        events.append(dict(type='turn.completed', usage=dict(input_tokens=123, output_tokens=4)))
        native = adapter.read_events(('\n'.join(map(json.dumps, events))).encode())
        self.assertTrue(native['completed'])
        self.assertEqual(native['usage']['input_tokens'], 123)
        events.append(dict(type='turn.failed'))
        self.assertFalse(adapter.read_events(('\n'.join(map(json.dumps, events))).encode())['completed'])

    def test_only_the_owned_session_is_read_and_actual_identity_is_preserved(self):
        id = '22222222-2222-2222-2222-222222222222'
        with tempfile.TemporaryDirectory() as td:
            directory = Path(td)/'sessions/2026/09/12'; directory.mkdir(parents=True)
            # A different session is deliberately unparsable and must never be opened.
            (directory/'unrelated.jsonl').write_text('PRIVATE INVALID JSON')
            owned = directory/f'rollout-{id}.jsonl'
            owned.write_text(json.dumps(dict(type='turn_context', payload=dict(model='different-model', effort='low', sandbox_policy=dict(type='read-only'), approval_policy='never')))+'\n')
            context = adapter.own_turn_context(td, id)
            self.assertEqual(context['model'], 'different-model')
            self.assertEqual(context['effort'], 'low')


if __name__ == '__main__':
    unittest.main()
