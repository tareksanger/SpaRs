"""Keep workflow action references compatible with the repository pinning policy."""
from pathlib import Path
import re
import unittest


def check_action_pins(text: str) -> None:
    count = 0
    for number, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith('#') or not re.search(r'''(?:\buses|["']uses["'])\s*:''', line):
            continue
        # Reject alternate YAML layouts instead of silently overlooking actions.
        match = re.fullmatch(
            r'''\s*(?:-\s*)?(?:uses|"uses"|'uses'):\s*(?P<quote>["']?)[\w.-]+/[\w./-]+@[0-9a-f]{40}(?P=quote)\s*(?:#.*)?''', line)
        if match is None:
            raise ValueError(f'Line {number}: use a block-style action declaration pinned to a full commit hash')
        count += 1
    if count == 0:
        raise ValueError('No action references were checked')


class WorkflowTests(unittest.TestCase):
    def test_ci_runs_for_pull_requests_and_only_main_pushes(self) -> None:
        workflow = (Path(__file__).resolve().parent.parent / '.github/workflows/ci.yml').read_text()
        triggers = workflow.split('\non:', 1)[1].split('\npermissions:', 1)[0]
        self.assertEqual(triggers, '\n  push:\n    branches: [main]\n  pull_request:')

    def test_remote_actions_use_full_commit_hashes(self) -> None:
        root = Path(__file__).resolve().parent.parent
        workflows = sorted((root / '.github/workflows').glob('*.y*ml'))
        self.assertTrue(workflows, 'No workflows were checked')
        for path in workflows:
            with self.subTest(workflow=path.name):
                check_action_pins(path.read_text())

    def test_model_reference_regeneration_has_a_required_canonical_job(self) -> None:
        workflow = (Path(__file__).resolve().parent.parent / '.github/workflows/ci.yml').read_text()
        self.assertRegex(workflow, r"os: ubuntu-latest\n\s+model-reference-args: --model-exports-only")
        self.assertRegex(workflow, r"os: macos-latest\n\s+model-reference-args: ''")
        self.assertIn('test "$(uname -m)" = arm64', workflow)
        self.assertIn('tools/verify.py ${{ matrix.model-reference-args }}', workflow)
        job = workflow.split('  reference-and-rust:', 1)[1].split('  native-installation:', 1)[0]
        self.assertIn('name: reference-and-rust (${{ matrix.os }})', job)
        self.assertNotIn('continue-on-error:', job)
        self.assertEqual(re.findall(r'^\s+if: (.*)$', job, re.MULTILINE), ["runner.os == 'macOS'", 'always()'])

    def test_unpinned_and_alternate_action_declarations_fail(self) -> None:
        pinned = '  - uses: example/action@' + 'a' * 40 + ' # v1\n'
        check_action_pins(pinned)
        check_action_pins('  - "uses": "example/action@' + 'a' * 40 + '"\n')
        for unpinned in ('  - uses: example/action@main', '  - { uses: example/action@main }',
                         '  - "uses": "example/action@v1"', '  - uses:\n      example/action@main',
                         '  - uses: example/action@abcdef'):
            with self.subTest(declaration=unpinned), self.assertRaises(ValueError):
                check_action_pins(pinned + unpinned)
        with self.assertRaisesRegex(ValueError, 'No action'):
            check_action_pins('# uses: example/action@main\n')


if __name__ == '__main__':
    unittest.main()
