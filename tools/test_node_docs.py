"""Check TypeScript extraction, execution failures, and temporary-file cleanup."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from subprocess import CompletedProcess
from check_docs import rust_examples
from node_docs import examples, run_example


class NodeDocumentationTests(unittest.TestCase):
    def test_only_canonical_typescript_fences_execute(self) -> None:
        source = '```sh\necho hi\n```\n```typescript\nthrow new Error("failure");\n```'
        self.assertEqual(examples(source), ['throw new Error("failure");\n'])
        self.assertEqual(rust_examples(source), 0)
        surrounded = '````text\nexample\n````\n```typescript\nthrow new Error("must run");\n```'
        self.assertEqual(examples(surrounded), ['throw new Error("must run");\n'])
        for fence in ['```typescript,ignore', '~~~typescript', '   ```typescript']:
            with self.assertRaises(ValueError):
                rust_examples(fence + '\nconst x = 1;\n' + fence[:3])

    def test_type_or_runtime_failure_fails_and_cleans_up(self) -> None:
        for type_code, run_code in [(1, 0), (0, 1), (0, 0)]:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                folder = root/'bindings/node'
                folder.mkdir(parents=True)
                replies = [CompletedProcess(['tsc'], type_code, '', 'type diagnostic'),
                           CompletedProcess(['node'], run_code, '', 'runtime diagnostic')]
                with patch('node_docs.subprocess.run', side_effect=replies) as run:
                    result = run_example(root, 'throw new Error("failure");')
                    self.assertEqual(result.returncode, type_code or run_code)
                    self.assertEqual(run.call_count, 1 if type_code else 2)
                self.assertEqual(list(folder.iterdir()), [])

    def test_real_type_and_runtime_failures_are_detected(self) -> None:
        root = Path(__file__).resolve().parent.parent
        typed = run_example(root, 'const count: number = "wrong";')
        self.assertNotEqual(typed.returncode, 0)
        self.assertIn('not assignable', typed.stdout)
        executed = run_example(root, 'throw new Error("intentional-guide-failure");')
        self.assertNotEqual(executed.returncode, 0)
        self.assertIn('intentional-guide-failure', executed.stderr)
