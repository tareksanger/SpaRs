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
    def test_remote_actions_use_full_commit_hashes(self) -> None:
        root = Path(__file__).resolve().parent.parent
        workflows = sorted((root / '.github/workflows').glob('*.y*ml'))
        self.assertTrue(workflows, 'No workflows were checked')
        for path in workflows:
            with self.subTest(workflow=path.name):
                check_action_pins(path.read_text())

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
