"""Frozen Doc.set_ents fixture checks with an independent recomputation of every state."""
from dataclasses import asdict
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from entity_update_reference import ROOT, Case, EntitySpan, State, TokenEntity, Update, build, verify_source


def expected_state(before: State, update: Update) -> State:
    """Apply the documented `Doc.set_ents` contract with plain lists, without spaCy."""
    length = len(before.tokens)
    labeled = [e for e in update.entities if e.label]
    ranges = [(e.start, e.end) for e in update.entities] + [
        (r.start, r.end) for r in [*update.blocked, *update.missing, *update.outside]]
    if any(not 0 <= start <= end <= length for start, end in ranges):
        return State('IndexError', before.tokens, before.entities)
    covered = [i for e in labeled for i in range(e.start, e.end)] + [
        i for r in [*update.blocked, *update.missing, *update.outside] for i in range(r.start, r.end)]
    if len(covered) != len(set(covered)):
        return State('ValueError', before.tokens, before.entities)
    tags: list[tuple[str | None, str]] = [(t.iob, t.type or '') for t in before.tokens]
    for e in labeled:
        for i in range(e.start, e.end):
            tags[i] = ('B' if i == e.start else 'I', e.label)
    for ranges_, iob in ((update.blocked, 'B'), (update.missing, None), (update.outside, 'O')):
        for r in ranges_:
            for i in range(r.start, r.end):
                tags[i] = (iob, '')
    default = {'outside': 'O', 'missing': None, 'blocked': 'B'}
    if update.default != 'unmodified':
        for i in set(range(length)) - set(covered):
            tags[i] = (default[update.default], '')
    for i in range(1, length):
        (previous, previous_type), (iob, kind) = tags[i - 1], tags[i]
        if iob == 'I' and (previous in (None, 'O') or previous_type != kind):
            tags[i] = ('B', kind)
    tokens = [TokenEntity(iob, kind if iob else None) for iob, kind in tags]
    # spaCy's has_annotation: an empty document counts as annotated.
    if tokens and all(t.iob is None for t in tokens):
        return State(None, tokens, None)
    entities: list[EntitySpan] = []
    for i, (iob, kind) in enumerate(tags):
        if iob == 'B' and kind:
            end = i + 1
            while end < length and tags[end][0] == 'I':
                end += 1
            entities.append(EntitySpan(i, end, kind))
    return State(None, tokens, entities)


class EntityUpdateReferenceTests(unittest.TestCase):
    def test_source_is_pinned(self) -> None:
        versions, sources = verify_source()
        self.assertEqual(versions, {'spacy': '3.8.14'})
        self.assertEqual(list(sources), ['spacy/tokens/doc.pyx', 'spacy/tokens/span.pyx'])
        with patch('entity_update_reference.spacy.__version__', '3.8.16'):
            with self.assertRaises(ValueError):
                verify_source()

    def test_fixture_regenerates_and_agrees_with_independent_states(self) -> None:
        fixture = build()
        self.assertEqual(asdict(fixture), json.loads((ROOT / 'fixtures/entity-updates-v1.expected.json').read_text()))
        cases: list[Case] = fixture.cases
        errors = {'IndexError': 0, 'ValueError': 0}
        defaults: set[str] = set()
        # Inside tags repaired to B after a non-entity tag, and after a tag of another type.
        repaired = {'after-outside-or-missing': 0, 'after-other-type': 0}
        for case in cases:
            current = case.initial
            for step, (update, recorded) in enumerate(zip(case.updates, case.states, strict=True)):
                expected = expected_state(current, update)
                self.assertEqual(expected, recorded, f'{case.id} update {step}')
                if recorded.error:
                    errors[recorded.error] += 1
                else:
                    defaults.add(update.default)
                    for i, (previous, token) in enumerate(zip(recorded.tokens, recorded.tokens[1:]), 1):
                        requested = any(e.label and e.start < i < e.end for e in update.entities)
                        written = [*update.entities, *update.blocked, *update.missing, *update.outside]
                        kept = (update.default == 'unmodified' and current.tokens[i].iob == 'I'
                                and not any(r.start <= i < r.end for r in written))
                        if token.iob == 'B' and (requested or kept):
                            other = previous.iob in ('B', 'I') and previous.type != token.type
                            repaired['after-other-type' if other else 'after-outside-or-missing'] += 1
                current = recorded
        # The suite reaches both spaCy errors, every default and the repair pass.
        self.assertEqual(errors, {'IndexError': 18, 'ValueError': 4})
        self.assertEqual(defaults, {'outside', 'missing', 'blocked', 'unmodified'})
        self.assertTrue(all(repaired.values()), repaired)
        after_ner = next(c for c in cases if c.id == 'after-ner')
        self.assertEqual([e.label for e in after_ner.initial.entities or []], ['ORG', 'GPE', 'MONEY', 'PERSON', 'GPE'])

    def test_generator_refuses_to_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'nested/entity-updates.json'
            output.parent.mkdir()
            output.write_text('frozen')
            result = subprocess.run([sys.executable, str(ROOT / 'tools/entity_update_reference.py'), str(output)],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Refusing to overwrite', result.stderr)
            self.assertEqual(output.read_text(), 'frozen')


if __name__ == '__main__':
    unittest.main()
