import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { NativeDocument, PhraseMatcher } from '../index.js';

type Match = { rule: string; start: number; end: number };
type Operation = { action: string; rule: string; patterns: string[][] };
type State = { error: string | null; rules: string[]; matches: Match[] };
type Case = { id: string; words: string[]; spaces: boolean[]; operations: Operation[]; states: State[] };
function record(value: unknown): Record<string, unknown> {
  assert.ok(value !== null && typeof value === 'object' && !Array.isArray(value));
  return Object.fromEntries(Object.entries(value));
}
function array(value: unknown): unknown[] { assert.ok(Array.isArray(value)); return value; }
function string(value: unknown): string { assert.equal(typeof value, 'string'); return String(value); }
function number(value: unknown): number { assert.ok(typeof value === 'number' && Number.isSafeInteger(value)); return value; }
function boolean(value: unknown): boolean { assert.equal(typeof value, 'boolean'); return value === true; }
function parseCase(value: unknown): Case {
  const data = record(value);
  return {
    id: string(data.id), words: array(data.words).map(string), spaces: array(data.spaces).map(boolean),
    operations: array(data.operations).map(value => {
      const operation = record(value);
      return { action: string(operation.action), rule: string(operation.rule), patterns: array(operation.patterns).map(pattern => array(pattern).map(string)) };
    }),
    states: array(data.states).map(value => {
      const state = record(value);
      return { error: state.error === null ? null : string(state.error), rules: array(state.rules).map(string), matches: array(state.matches).map(value => {
        const match = record(value);
        return { rule: string(match.rule), start: number(match.start), end: number(match.end) };
      }) };
    }),
  };
}
function fixture(name: string): Record<string, unknown> {
  const value: unknown = JSON.parse(readFileSync(new URL(`../../../fixtures/${name}`, import.meta.url), 'utf8'));
  return record(value);
}
function doc(words: string[], spaces: boolean[] = words.map(() => false), starts: boolean[] = []): NativeDocument {
  assert.equal(words.length, spaces.length);
  let text = '';
  const tokens = words.map((word, index) => {
    const start = Buffer.byteLength(text);
    const idx = [...text].length;
    text += word;
    const end = Buffer.byteLength(text);
    const whitespace = spaces[index];
    if (whitespace) text += ' ';
    return { start, end, idx, whitespace, norm: word, sentence_start: starts[index] ?? null };
  });
  return NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: { text, tokens } }));
}
function hasCode(code: string): (error: unknown) => boolean {
  return (error: unknown): boolean => error instanceof Error && 'code' in error && error.code === code;
}
async function checkCase(item: Case, starts: boolean[] = [], attribute: string = 'ORTH'): Promise<void> {
  const input = NativeDocument.fromSnapshot(doc(item.words, item.spaces, starts).toSnapshot());
  const matcher = new PhraseMatcher(attribute);
  const labels = new Set<string>();
  assert.equal(item.operations.length, item.states.length);
  for (const [index, operation] of item.operations.entries()) {
    const expected = item.states[index];
    assert.ok(expected);
    labels.add(operation.rule);
    const apply = (): void => {
      if (operation.action === 'add') matcher.add(operation.rule, operation.patterns.map(pattern => doc(pattern)));
      else { assert.equal(operation.action, 'remove'); matcher.remove(operation.rule); }
    };
    if (expected.error !== null) assert.throws(apply, hasCode('SPARS_INVALID_PATTERN'));
    else apply();
    assert.equal(matcher.size, expected.rules.length, `${item.id} state ${index}`);
    for (const label of labels) {
      assert.equal(matcher.contains(label), expected.rules.includes(label));
      assert.equal(matcher.get(label) !== null, expected.rules.includes(label));
    }
    const results = await Promise.all([matcher.findMatches(input), matcher.findMatches(input)]);
    for (const result of results) assert.deepEqual(result, expected.matches, `${item.id} state ${index}`);
  }
}

for (const [filename, cases, states, matches] of [
  ['phrase-match-v1.expected.json', 19, 529, 14835],
  ['phrase-match-holdout-v1.expected.json', 24, 720, 17084],
] as const) {
  test(`PhraseMatcher preserves frozen ordered reference corpus ${filename}`, async () => {
    const items = array(fixture(filename).cases).map(parseCase);
    assert.equal(items.length, cases);
    assert.equal(items.reduce((count, item) => count + item.states.length, 0), states);
    assert.equal(items.reduce((count, item) => count + item.states.reduce((count, state) => count + state.matches.length, 0), 0), matches);
    for (const item of items) await checkCase(item);
  });
}

test('PhraseMatcher crosses explicit sentence boundaries', async () => {
  const data = fixture('phrase-match-edges-v1.expected.json');
  await checkCase(parseCase(data.sentence), array(data.sentence_starts).map(boolean));
});

test('PhraseMatcher owns patterns, validates strings, and prevents pending mutation', async () => {
  const matcher = new PhraseMatcher('TEXT');
  assert.equal(matcher.attribute, 'TEXT');
  assert.equal(new PhraseMatcher().attribute, 'ORTH');
  assert.throws(() => new PhraseMatcher('LEMMA'), hasCode('SPARS_UNSUPPORTED'));
  assert.throws(() => new PhraseMatcher('\ud800'), hasCode('SPARS_INVALID_TEXT'));
  for (const call of [() => matcher.contains('\ud800'), () => matcher.get('\ud800'), () => matcher.remove('\ud800'), () => matcher.add('\ud800', [])]) {
    assert.throws(call, hasCode('SPARS_INVALID_TEXT'));
  }
  matcher.add('r', [doc(['é', '🙂']), doc(['é', '🙂']), doc([])]);
  assert.deepEqual(matcher.get('r'), [['é', '🙂']]);
  const returned = matcher.get('r');
  assert.ok(returned);
  returned[0]?.push('mutated');
  assert.deepEqual(matcher.get('r'), [['é', '🙂']]);
  const input = doc(['É', '🙂', 'é', '🙂']);
  const pending = matcher.findMatches(input);
  assert.throws(() => matcher.add('later', [input]), hasCode('SPARS_BUSY'));
  assert.throws(() => matcher.remove('r'), hasCode('SPARS_BUSY'));
  assert.deepEqual(await pending, [{ rule: 'r', start: 2, end: 4 }]);
  matcher.remove('r');
  assert.equal(matcher.get('r'), null);
  assert.deepEqual(await matcher.findMatches(input), []);
  matcher.add('empty', [doc([])]);
  assert.equal(matcher.size, 1);
  assert.deepEqual(matcher.get('empty'), []);
});

test('PhraseMatcher LOWER follows pinned Unicode reference lifecycle', async () => {
  const items = array(fixture('phrase-match-lower-v1.expected.json').cases).map(parseCase);
  assert.equal(items.length, 4);
  assert.equal(items.reduce((count, item) => count + item.states.length, 0), 24);
  assert.equal(items.flatMap(item => item.states).reduce((count, state) => count + state.matches.length, 0), 351);
  for (const item of items) await checkCase(item, [], 'LOWER');
  const matcher = new PhraseMatcher('LOWER');
  assert.equal(matcher.attribute, 'LOWER');
  matcher.add('hotel', [doc(['The', 'Ritz']), doc(['the', 'ritz'])]);
  assert.deepEqual(matcher.get('hotel'), [['the', 'ritz']]);
});
