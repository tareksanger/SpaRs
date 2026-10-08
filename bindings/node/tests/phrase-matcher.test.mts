import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { loadModel, NativeDocument, PhraseMatcher } from '../index.js';
import type { PhraseAttribute } from '../index.js';
import { modelPath } from './fixtures.mts';
import { member, PHRASE_ATTRIBUTES } from './vocabulary.mts';

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
/** Construct a matcher with a deliberately unsupported attribute name. */
function construct(attribute: string): PhraseMatcher {
  return Reflect.construct(PhraseMatcher, [attribute]);
}
async function checkCase(item: Case, starts: boolean[] = [], attribute: PhraseAttribute = 'ORTH'): Promise<void> {
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
  for (const attribute of ['lemma', 'SHAPE', 'ENT_TYPE', 'MORPHOLOGY']) {
    assert.throws(() => construct(attribute), hasCode('SPARS_UNSUPPORTED'));
  }
  assert.throws(() => construct('\ud800'), hasCode('SPARS_INVALID_TEXT'));
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

type AnnotatedToken = { word: string; space: boolean; norm: string; lemma: string | null; pos: string | null; tag: string | null; dep: string | null; morph: string | null };
type AnnotatedCase = {
  id: string; attribute: PhraseAttribute; tokens: AnnotatedToken[];
  operations: { action: string; rule: string; patterns: AnnotatedToken[][] }[];
  states: { error: string | null; rules: string[]; patterns: string[][][]; matches: Match[] }[];
};
function optional(value: unknown): string | null { return value === null ? null : string(value); }
function annotatedToken(value: unknown): AnnotatedToken {
  const data = record(value);
  return {
    word: string(data.word), space: boolean(data.space), norm: string(data.norm), lemma: optional(data.lemma),
    pos: optional(data.pos), tag: optional(data.tag), dep: optional(data.dep), morph: optional(data.morph),
  };
}
function parseAnnotatedCase(value: unknown): AnnotatedCase {
  const data = record(value);
  return {
    id: string(data.id), attribute: member(PHRASE_ATTRIBUTES, data.attribute), tokens: array(data.tokens).map(annotatedToken),
    operations: array(data.operations).map(value => {
      const operation = record(value);
      return { action: string(operation.action), rule: string(operation.rule), patterns: array(operation.patterns).map(pattern => array(pattern).map(annotatedToken)) };
    }),
    states: array(data.states).map(value => {
      const state = record(value);
      return {
        error: optional(state.error), rules: array(state.rules).map(string),
        patterns: array(state.patterns).map(rule => array(rule).map(pattern => array(pattern).map(string))),
        matches: array(state.matches).map(value => {
          const match = record(value);
          return { rule: string(match.rule), start: number(match.start), end: number(match.end) };
        }),
      };
    }),
  };
}
// Python's list ordering: element by element in code point order, then by length.
function compareCodePoints(left: string, right: string): number {
  const a = [...left].map(c => c.codePointAt(0) ?? 0);
  const b = [...right].map(c => c.codePointAt(0) ?? 0);
  for (let i = 0; i < Math.min(a.length, b.length); i++) if (a[i] !== b[i]) return (a[i] ?? 0) - (b[i] ?? 0);
  return a.length - b.length;
}
function compareSequences(left: string[], right: string[]): number {
  for (let i = 0; i < Math.min(left.length, right.length); i++) {
    const order = compareCodePoints(left[i] ?? '', right[i] ?? '');
    if (order !== 0) return order;
  }
  return left.length - right.length;
}
function annotatedDoc(tokens: AnnotatedToken[]): NativeDocument {
  let text = '';
  const stored = tokens.map(token => {
    const start = Buffer.byteLength(text);
    const idx = [...text].length;
    text += token.word;
    const end = Buffer.byteLength(text);
    if (token.space) text += ' ';
    return { start, end, idx, whitespace: token.space, norm: token.norm, lemma: token.lemma, pos: token.pos, tag: token.tag, dep: token.dep, morphology: token.morph };
  });
  return NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: { text, tokens: stored } }));
}

const required: Record<string, keyof AnnotatedToken | null> = { NORM: null, LEMMA: 'lemma', POS: 'pos', TAG: 'tag', DEP: 'dep', MORPH: 'morph' };

test('PhraseMatcher matches annotations like the pinned reference', async () => {
  const items = array(fixture('phrase-match-annotations-v1.expected.json').cases).map(parseAnnotatedCase);
  const states = items.flatMap(item => item.states);
  assert.deepEqual(
    [items.length, states.length, states.reduce((count, state) => count + state.matches.length, 0), states.filter(state => state.error !== null).length],
    [18, 204, 705, 45],
  );
  for (const item of items) {
    const input = annotatedDoc(item.tokens);
    const matcher = new PhraseMatcher(item.attribute);
    assert.equal(matcher.attribute, item.attribute);
    const field = required[item.attribute];
    assert.ok(field !== undefined);
    // spaCy registers the rule and keeps patterns before a rejected one; SpaRs changes
    // nothing. Such a rule is compared only for its SpaRs registration (the value)
    // until a later successful addition makes both agree.
    const diverged = new Map<string, boolean>();
    const labels = new Set<string>();
    for (const [index, operation] of item.operations.entries()) {
      const expected = item.states[index];
      assert.ok(expected);
      const context = `${item.id} state ${index}`;
      labels.add(operation.rule);
      const before = matcher.contains(operation.rule);
      const apply = (): void => {
        if (operation.action === 'add') matcher.add(operation.rule, operation.patterns.map(annotatedDoc));
        else { assert.equal(operation.action, 'remove'); assert.ok(!diverged.has(operation.rule), context); matcher.remove(operation.rule); }
      };
      if (expected.error === null) {
        apply();
        diverged.delete(operation.rule);
      } else {
        assert.equal(expected.error, 'ValueError', context);
        assert.throws(apply, hasCode('SPARS_INVALID_PATTERN'), context);
        assert.ok(field !== null, context);
        const nonempty = operation.patterns.filter(pattern => pattern.length > 0);
        const storedBefore = nonempty.findIndex(pattern => pattern.every(token => token[field] === null));
        if (!before || storedBefore > 0) diverged.set(operation.rule, before);
      }
      const rules = expected.rules.filter(rule => diverged.get(rule) ?? true);
      assert.equal(matcher.size, rules.length, context);
      for (const label of labels) assert.equal(matcher.contains(label), rules.includes(label), context);
      for (const [position, rule] of expected.rules.entries()) {
        if (diverged.has(rule)) continue;
        const stored = matcher.get(rule);
        assert.ok(stored, context);
        assert.deepEqual([...stored].sort(compareSequences), expected.patterns[position], context);
      }
      const keep = (matches: Match[]): Match[] => matches.filter(match => !diverged.has(match.rule));
      const results = await Promise.all([matcher.findMatches(input), matcher.findMatches(input)]);
      for (const result of results) assert.deepEqual(keep(result), keep(expected.matches), context);
    }
    assert.equal(diverged.size, 0, item.id);
  }
});

test('PhraseMatcher MORPH rejects noncanonical morphology with SPARS_UNSUPPORTED', async () => {
  const token = (morph: string): AnnotatedToken => ({ word: 'ran', space: false, norm: 'ran', lemma: 'run', pos: 'VERB', tag: 'VBD', dep: 'ROOT', morph });
  const noncanonical = annotatedDoc([token('VerbForm=Fin|Tense=Past')]);
  const matcher = new PhraseMatcher('MORPH');
  assert.throws(() => matcher.add('r', [noncanonical]), hasCode('SPARS_UNSUPPORTED'));
  assert.equal(matcher.size, 0);
  matcher.add('r', [annotatedDoc([token('Tense=Past|VerbForm=Fin')])]);
  await assert.rejects(matcher.findMatches(noncanonical), hasCode('SPARS_UNSUPPORTED'));
  const lemma = new PhraseMatcher('LEMMA');
  lemma.add('r', [noncanonical]);
  assert.deepEqual(await lemma.findMatches(noncanonical), [{ rule: 'r', start: 0, end: 1 }]);
});

const LEXICAL: readonly PhraseAttribute[] = ['IS_ALPHA', 'IS_ASCII', 'IS_DIGIT', 'IS_LOWER', 'IS_UPPER', 'IS_TITLE', 'IS_PUNCT', 'IS_SPACE', 'IS_BRACKET', 'IS_QUOTE',
  'IS_LEFT_PUNCT', 'IS_RIGHT_PUNCT', 'IS_CURRENCY', 'IS_STOP', 'LIKE_NUM', 'LIKE_URL', 'LIKE_EMAIL', 'LENGTH'];

test('PhraseMatcher accepts every supported attribute name and rejects the rest', () => {
  // Every declared attribute is accepted and read back; the declared list is exhaustive (see vocabulary.mts).
  for (const name of PHRASE_ATTRIBUTES) {
    assert.equal(new PhraseMatcher(name).attribute, name);
  }
  assert.deepEqual(['ORTH', 'TEXT', 'LOWER', 'NORM', 'LEMMA', 'POS', 'TAG', 'DEP', 'MORPH', ...LEXICAL], [...PHRASE_ATTRIBUTES]);
  for (const name of ['is_alpha', 'ISALPHA', 'SHAPE', 'IS_SENT_START', 'SPACY', 'ENT_TYPE']) {
    assert.throws(() => construct(name), hasCode('SPARS_UNSUPPORTED'));
  }
  const matcher = new PhraseMatcher('IS_ALPHA');
  assert.throws(() => matcher.add('r', [doc(['a'])]), hasCode('SPARS_INVALID_PATTERN'));
  assert.throws(() => matcher.add('r', []), hasCode('SPARS_INVALID_PATTERN'));
  assert.equal(matcher.size, 0);
});

// spaCy stores a flag as 0 or 1 and a length as itself; SpaRs stores text.
function storedValue(attribute: string, key: number): string {
  if (attribute === 'LENGTH') return String(key);
  assert.ok(key === 0 || key === 1);
  return key === 1 ? 'true' : 'false';
}

test('PhraseMatcher lexical flags and LENGTH match the pinned reference', async () => {
  const model = await loadModel(modelPath);
  const cases = array(fixture('phrase-match-lexical-v1.expected.json').cases);
  assert.equal(cases.length, 36);
  let states = 0;
  let matches = 0;
  for (const value of cases) {
    const item = record(value);
    const attribute = member(LEXICAL, item.attribute);
    const input = doc(array(item.words).map(string), array(item.spaces).map(boolean));
    // LENGTH needs no model; the flags use the model's language rules.
    const matcher = attribute === 'LENGTH' ? new PhraseMatcher(attribute) : new PhraseMatcher(attribute, model);
    const operations = array(item.operations).map(record);
    const expectedStates = array(item.states).map(record);
    assert.equal(operations.length, expectedStates.length);
    for (const [index, operation] of operations.entries()) {
      const expected = expectedStates[index];
      assert.ok(expected);
      const context = `${string(item.id)} state ${index}`;
      assert.equal(string(operation.action), 'add', context);
      assert.equal(expected.error, null, context);
      matcher.add(string(operation.rule), array(operation.patterns).map(pattern => doc(array(pattern).map(string))));
      const rules = array(expected.rules).map(string);
      assert.equal(matcher.size, rules.length, context);
      for (const [position, rule] of rules.entries()) {
        const keys: string[][] = array(array(expected.patterns)[position]).map(pattern => array(pattern).map(number).map(key => storedValue(attribute, key)));
        const stored = matcher.get(rule);
        assert.ok(stored, context);
        assert.deepEqual([...stored].sort(compareSequences), keys.sort(compareSequences), context);
      }
      const expectedMatches = array(expected.matches).map(value => {
        const match = record(value);
        return { rule: string(match.rule), start: number(match.start), end: number(match.end) };
      });
      const results = await Promise.all([matcher.findMatches(input), matcher.findMatches(input)]);
      for (const result of results) assert.deepEqual(result, expectedMatches, context);
      states += 1;
      matches += expectedMatches.length;
    }
  }
  assert.deepEqual([states, matches], [180, 11956]);
});
