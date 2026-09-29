import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { NativeDocument, TokenMatcher } from '../index.js';
import type { TokenPattern, TokenConstraint, TokenPredicate, TokenRepetition } from '../index.js';
function record(value: unknown): Record<string, unknown> {
  assert.ok(value !== null && typeof value === 'object' && !Array.isArray(value));
  return Object.fromEntries(Object.entries(value));
}
function array(value: unknown): unknown[] { assert.ok(Array.isArray(value)); return value; }
function string(value: unknown): string { assert.equal(typeof value, 'string'); return String(value); }
function number(value: unknown): number { assert.ok(typeof value === 'number' && Number.isSafeInteger(value)); return value; }
function predicate(value: unknown): TokenPredicate {
  const data = record(value);
  return { kind: string(data.kind), ...(data.value === undefined ? {} : { value: string(data.value) }), ...(data.values === undefined ? {} : { values: array(data.values).map(string) }) };
}
function constraint(value: unknown): TokenConstraint {
  const data = record(value);
  return { attribute: string(data.attribute), predicate: predicate(data.predicate) };
}
function repetition(value: unknown): TokenRepetition {
  const data = record(value);
  return { kind: string(data.kind), ...(data.min === undefined ? {} : { min: number(data.min) }), ...(data.max === undefined || data.max === null ? {} : { max: number(data.max) }) };
}
function pattern(value: unknown): TokenPattern {
  return { tokens: array(record(value).tokens).map(value => {
    const item = record(value);
    return { constraints: array(item.constraints).map(constraint), repetition: repetition(item.repetition) };
  }) };
}
function hasCode(code: string): (error: unknown) => boolean {
  return (error: unknown): boolean => error instanceof Error && 'code' in error && error.code === code;
}
function plainDoc(text: string): NativeDocument {
  let start = 0;
  const tokens = [...text].map((norm, idx) => {
    const previous = start;
    start += Buffer.byteLength(norm);
    return { start: previous, end: start, idx, norm, whitespace: false };
  });
  return NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: { text, tokens } }));
}
function wildcard(repetition: TokenRepetition = { kind: 'once' }): TokenPattern {
  return { tokens: [{ constraints: [], repetition }] };
}
for (const [filename, cases, rules, matches] of [
  ['token-match-v1.expected.json', 10, 480, 704],
  ['token-match-exhaustive-v1.expected.json', 31, 4805, 6261],
  ['token-match-branching-v1.expected.json', 31, 248, 288],
  ['token-match-lower-v1.expected.json', 4, 172, 280],
] as const) {
  test(`TokenMatcher preserves frozen ordered reference corpus ${filename}`, async () => {
    const parsed: unknown = JSON.parse(readFileSync(new URL(`../../../fixtures/${filename}`, import.meta.url), 'utf8'));
    const items = array(record(parsed).cases).map(record);
    assert.equal(items.length, cases);
    let ruleCount = 0;
    let matchCount = 0;
    for (const item of items) {
      const matcher = new TokenMatcher();
      const doc = NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: item }));
      for (const raw of array(item.rules)) {
        const rule = record(raw);
        matcher.add(string(rule.name), array(rule.patterns).map(pattern));
        ruleCount++;
      }
      const expected = array(item.expected).map(value => {
        const match = record(value);
        return { rule: string(match.rule), start: number(match.start), end: number(match.end) };
      });
      matchCount += expected.length;
      assert.equal(matcher.size, new Set(array(item.rules).map(value => string(record(value).name))).size);
      for (const actual of await Promise.all([matcher.findMatches(doc), matcher.findMatches(doc)])) {
        assert.deepEqual(actual, expected, string(item.id));
      }
    }
    assert.equal(ruleCount, rules);
    assert.equal(matchCount, matches);
  });
}

test('TokenMatcher lifecycle, ownership, and pending mutation', async () => {
  const matcher = new TokenMatcher();
  const input = plainDoc('xx');
  const two: TokenPattern = { tokens: [...wildcard().tokens, ...wildcard().tokens] };
  matcher.add('a', [two]);
  matcher.add('b', [wildcard()]);
  matcher.add('a', [wildcard(), wildcard()]);
  assert.equal(matcher.size, 2);
  assert.equal(matcher.contains('a'), true);
  assert.equal(matcher.get('a')?.length, 3);
  two.tokens.splice(0);
  matcher.get('a')?.splice(0);
  assert.equal(matcher.get('a')?.length, 3);
  const pending = matcher.findMatches(input);
  assert.throws(() => matcher.remove('a'), hasCode('SPARS_BUSY'));
  assert.throws(() => matcher.add('c', []), hasCode('SPARS_BUSY'));
  assert.deepEqual(await pending, [
    { rule: 'b', start: 0, end: 1 }, { rule: 'a', start: 0, end: 1 },
    { rule: 'a', start: 0, end: 2 }, { rule: 'b', start: 1, end: 2 }, { rule: 'a', start: 1, end: 2 },
  ]);
  matcher.remove('a');
  assert.equal(matcher.get('a'), null);
  assert.equal(matcher.contains('a'), false);
  assert.throws(() => matcher.remove('a'), hasCode('SPARS_INVALID_PATTERN'));
  assert.deepEqual(await matcher.findMatches(plainDoc('x')), [{ rule: 'b', start: 0, end: 1 }]);
});

test('TokenMatcher rejects malformed patterns atomically and reports unavailable annotations', async () => {
  const matcher = new TokenMatcher();
  const invalid: TokenPattern[] = [
    { tokens: [] }, wildcard({ kind: 'unknown' }), wildcard({ kind: 'once', min: 0 }),
    wildcard({ kind: 'range' }), wildcard({ kind: 'range', min: 2, max: 1 }),
    ...[-1, 0.5, NaN, Infinity, 4294967296].map(min => wildcard({ kind: 'range', min })),
    wildcard({ kind: 'range', min: 4097 }),
    ...[
      { attribute: 'unknown', predicate: { kind: 'equals', value: 'a' } },
      { attribute: 'LOWER', predicate: { kind: 'equals', value: 'a' } },
      { attribute: 'lower', predicate: { kind: 'morph_superset', values: ['a'] } },
      { attribute: 'text', predicate: { kind: 'unknown', values: [] } },
      { attribute: 'text', predicate: { kind: 'equals' } },
      { attribute: 'text', predicate: { kind: 'equals', value: 'a', values: [] } },
      { attribute: 'text', predicate: { kind: 'in', value: 'a', values: [] } },
      { attribute: 'text', predicate: { kind: 'in' } },
      { attribute: 'text', predicate: { kind: 'morph_superset', values: ['Case=Nom'] } },
      { attribute: 'morphology', predicate: { kind: 'morph_superset', values: ['invalid'] } },
    ].map(constraint => ({ tokens: [{ constraints: [constraint], repetition: { kind: 'once' } }] })),
  ];
  for (const pattern of invalid) {
    assert.throws(() => matcher.add('invalid', [wildcard(), pattern]), hasCode('SPARS_INVALID_PATTERN'));
    assert.equal(matcher.size, 0);
  }
  for (const call of [() => matcher.contains('\ud800'), () => matcher.get('\ud800'), () => matcher.remove('\ud800'), () => matcher.add('\ud800', [])]) {
    assert.throws(call, hasCode('SPARS_INVALID_TEXT'));
  }
  assert.throws(() => matcher.add('bad', [{ tokens: [{ constraints: [{ attribute: 'text', predicate: { kind: 'equals', value: '\ud800' } }], repetition: { kind: 'once' } }] }]), hasCode('SPARS_INVALID_TEXT'));
  matcher.add('lemma', [{ tokens: [{ constraints: [{ attribute: 'lemma', predicate: { kind: 'equals', value: 'x' } }], repetition: { kind: 'once' } }] }]);
  await assert.rejects(matcher.findMatches(plainDoc('x')), /lemma/i);
});
