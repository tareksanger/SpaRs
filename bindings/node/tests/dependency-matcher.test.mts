import assert from 'node:assert/strict';
import { test } from 'node:test';
import { DependencyMatcher, loadModel, NativeDocument } from '../index.js';
import type { DependencyPattern, DependencyNode, Model, TokenConstraint, TokenPredicate } from '../index.js';
import { array, modelPath, readJson, record, root } from './fixtures.mts';

function string(value: unknown): string { assert.ok(typeof value === 'string'); return value; }
function constraint(value: unknown): TokenConstraint {
  const c = record(value);
  const p = record(c.predicate);
  const predicate: TokenPredicate = { kind: string(p.kind) };
  if (p.operator !== undefined && p.operator !== null) predicate.operator = string(p.operator);
  if (typeof p.value === 'boolean' || typeof p.value === 'number') predicate.value = p.value;
  else if (p.value !== undefined && p.value !== null) predicate.value = string(p.value);
  if (p.values !== undefined && p.values !== null) {
    const values = array(p.values);
    predicate.values = values.length > 0 && values.every(item => typeof item === 'number') ? values.map(item => { assert.ok(Number.isSafeInteger(item)); return Number(item); }) : values.map(string);
  }
  return { attribute: string(c.attribute), predicate };
}
function pattern(value: unknown): DependencyPattern {
  return { nodes: array(record(value).nodes).map(value => {
    const n = record(value);
    const node: DependencyNode = { id: string(n.id), constraints: array(n.constraints).map(constraint) };
    if (n.link !== null && n.link !== undefined) {
      const link = record(n.link);
      node.link = { left: string(link.left), relation: string(link.relation) };
    }
    return node;
  }) };
}
function document(value: unknown): NativeDocument {
  const c = record(value);
  return NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: {
    text: c.text, tokens: c.tokens, sentences: c.sentences, entities: c.entities, noun_chunks: c.noun_chunks,
  } }));
}
const singleton: DependencyPattern = { nodes: [{ id: 'a', constraints: [] }] };
function tiny(): NativeDocument {
  return NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: {
    text: 'a b', tokens: [
      { start: 0, end: 1, idx: 0, whitespace: true, norm: 'a', head: 1, dep: 'dep' },
      { start: 2, end: 3, idx: 2, whitespace: false, norm: 'b', head: 1, dep: 'ROOT' },
    ], sentences: null, entities: null, noun_chunks: null,
  } }));
}

test('all dependency relations, predicates and ordering match frozen official suites', async () => {
  const relations = new Set<string>();
  let lowerRules = 0;
  let lowerMatches = 0;
  let lengthRules = 0;
  let lengthMatches = 0;
  for (const [suite, count] of [['dependency-match-v1', 6], ['dependency-match-regressions-v1', 2], ['dependency-match-lower-v1', 4], ['dependency-match-length-v1', 3]] as const) {
    const fixture = record(readJson(`${root}fixtures/${suite}.expected.json`));
    assert.deepEqual(fixture.versions, { spacy: '3.8.14', thinc: '8.3.13' });
    const cases = array(fixture.cases);
    assert.equal(cases.length, count);
    for (const value of cases) {
      const c = record(value);
      const matcher = new DependencyMatcher();
      const rules = array(c.rules);
      const registered = new Map<string, DependencyPattern[]>();
      if (suite === 'dependency-match-v1') assert.equal(rules.length, 52);
      if (suite === 'dependency-match-lower-v1') {
        lowerRules += rules.length;
        lowerMatches += array(c.expected).length;
      }
      if (suite === 'dependency-match-length-v1') {
        lengthRules += rules.length;
        lengthMatches += array(c.expected).length;
      }
      for (const value of rules) {
        const rule = record(value);
        const patterns = array(rule.patterns).map(pattern);
        for (const p of patterns) for (const node of p.nodes) if (node.link) relations.add(node.link.relation);
        const name = string(rule.name);
        matcher.add(name, patterns);
        const expectedPatterns = [...(registered.get(name) ?? []), ...patterns];
        registered.set(name, expectedPatterns);
        const actualPatterns = matcher.get(name);
        assert.ok(actualPatterns);
        assert.deepEqual(actualPatterns.map(pattern), expectedPatterns);
      }
      if (suite === 'dependency-match-v1') assert.equal(matcher.size, 51);
      const doc = document(c);
      assert.deepEqual(await matcher.findMatches(doc), c.expected, string(c.id));
      assert.deepEqual(await matcher.findMatches(doc), c.expected);
      const concurrent = await Promise.all([matcher.findMatches(doc), matcher.findMatches(doc)]);
      assert.deepEqual(concurrent, [c.expected, c.expected]);
    }
  }
  assert.equal(relations.size, 20);
  assert.equal(lowerRules, 160);
  assert.equal(lowerMatches, 223);
  assert.equal(lengthRules, 96);
  assert.equal(lengthMatches, 321);
});

const FLAG_RULES = 42;
const FLAG_MATCHES = 376;
test('lexical flags match the frozen official suite with a model lexicon', async () => {
  const model = await loadModel(modelPath);
  const fixture = record(readJson(`${root}fixtures/dependency-match-flags-v1.expected.json`));
  const cases = array(fixture.cases);
  assert.equal(cases.length, 3);
  let rules = 0;
  let matches = 0;
  for (const value of cases) {
    const c = record(value);
    const matcher = new DependencyMatcher(model);
    for (const raw of array(c.rules)) {
      const rule = record(raw);
      matcher.add(string(rule.name), array(rule.patterns).map(pattern));
      rules++;
    }
    matches += array(c.expected).length;
    const doc = document(c);
    assert.deepEqual(await matcher.findMatches(doc), c.expected, string(c.id));
  }
  assert.equal(rules, FLAG_RULES);
  assert.equal(matches, FLAG_MATCHES);
  const flag: DependencyPattern = { nodes: [{ id: 'a', constraints: [{ attribute: 'like_num', predicate: { kind: 'flag', value: true } }] }] };
  const plain = new DependencyMatcher();
  assert.throws(() => plain.add('number', [flag]), (error: unknown) => error instanceof Error && 'code' in error && error.code === 'SPARS_INVALID_PATTERN');
  assert.equal(plain.size, 0);
  const withModel = new DependencyMatcher(model);
  withModel.add('number', [flag]);
  assert.deepStrictEqual(withModel.get('number'), [flag]);
  assert.throws(() => new DependencyMatcher({} as unknown as Model));
});

test('remaining lexical flags match the frozen official suite', async () => {
  const model = await loadModel(modelPath);
  const fixture = record(readJson(`${root}fixtures/dependency-match-more-flags-v1.expected.json`));
  const cases = array(fixture.cases);
  assert.equal(cases.length, 3);
  let rules = 0;
  let matches = 0;
  for (const value of cases) {
    const c = record(value);
    const matcher = new DependencyMatcher(model);
    for (const raw of array(c.rules)) {
      const rule = record(raw);
      matcher.add(string(rule.name), array(rule.patterns).map(pattern));
      rules++;
    }
    matches += array(c.expected).length;
    assert.deepEqual(await matcher.findMatches(document(c)), c.expected, string(c.id));
  }
  assert.equal(rules, 90);
  assert.equal(matches, 1209);
});

test('rule lifecycle appends patterns and isolates returned objects', async () => {
  const matcher = new DependencyMatcher();
  const doc = tiny();
  assert.equal(matcher.size, 0);
  assert.equal(matcher.get('absent'), null);
  assert.equal(matcher.contains('absent'), false);
  assert.deepEqual(await matcher.findMatches(doc), []);
  matcher.add('rule', [singleton]);
  assert.equal(matcher.contains('rule'), true);
  const saved = matcher.get('rule');
  assert.ok(saved);
  assert.equal(saved.length, 1);
  const first = saved[0];
  assert.ok(first);
  first.nodes.length = 0;
  assert.equal(matcher.get('rule')?.[0]?.nodes.length, 1);
  matcher.add('rule', [singleton]);
  assert.equal(matcher.size, 1);
  assert.equal((await matcher.findMatches(doc)).length, 4);
  matcher.remove('rule');
  assert.equal(matcher.size, 0);
  assert.throws(() => matcher.remove('rule'));
});

test('invalid patterns and predicates fail atomically', () => {
  const matcher = new DependencyMatcher();
  matcher.add('good', [singleton]);
  const invalid: DependencyPattern[] = [
    { nodes: [] },
    { nodes: [{ id: 'a', constraints: [], link: { left: 'a', relation: '>' } }] },
    { nodes: [{ id: 'a', constraints: [] }, { id: 'b', constraints: [] }] },
    { nodes: [{ id: 'a', constraints: [] }, { id: 'a', constraints: [], link: { left: 'a', relation: '>' } }] },
    { nodes: [{ id: 'a', constraints: [] }, { id: 'b', constraints: [], link: { left: 'missing', relation: '>' } }] },
    { nodes: [{ id: 'a', constraints: [] }, { id: 'b', constraints: [], link: { left: 'a', relation: 'unknown' } }] },
    { nodes: [{ id: 'a', constraints: [{ attribute: 'unknown', predicate: { kind: 'equals', value: 'a' } }] }] },
    { nodes: [{ id: 'a', constraints: [{ attribute: 'LOWER', predicate: { kind: 'equals', value: 'a' } }] }] },
    { nodes: [{ id: 'a', constraints: [{ attribute: 'lower', predicate: { kind: 'morph_superset', values: ['a'] } }] }] },
    { nodes: [{ id: 'a', constraints: [{ attribute: 'text', predicate: { kind: 'regex', value: 'a' } }] }] },
    { nodes: [{ id: 'a', constraints: [{ attribute: 'text', predicate: { kind: 'equals' } }] }] },
    { nodes: [{ id: 'a', constraints: [{ attribute: 'text', predicate: { kind: 'equals', value: 'a', values: ['a'] } }] }] },
  ];
  for (const p of invalid) {
    assert.throws(() => matcher.add('bad', [singleton, p]));
    assert.equal(matcher.contains('bad'), false);
    assert.throws(() => matcher.add('good', [singleton, p]));
    assert.equal(matcher.get('good')?.length, 1);
  }
  assert.throws(() => matcher.add('\ud800', [singleton]), { code: 'SPARS_INVALID_TEXT' });
  assert.throws(() => matcher.add('bad', [{ nodes: [{ id: '\ud800', constraints: [] }] }]), { code: 'SPARS_INVALID_TEXT' });
});

test('pending searches retain documents and block rule mutation until resolution', async () => {
  const matcher = new DependencyMatcher();
  matcher.add('rule', [singleton]);
  const pending = matcher.findMatches(tiny());
  assert.throws(() => matcher.add('next', [singleton]), { code: 'SPARS_BUSY' });
  assert.throws(() => matcher.remove('rule'), { code: 'SPARS_BUSY' });
  global.gc?.();
  assert.deepEqual(await pending, [{ rule: 'rule', tokens: [0] }, { rule: 'rule', tokens: [1] }]);
  matcher.remove('rule');
});

test('missing dependency heads and unavailable constrained annotations reject asynchronously', async () => {
  const doc = NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: {
    text: 'a', tokens: [{ start: 0, end: 1, idx: 0, whitespace: false, norm: 'a' }],
    sentences: null, entities: null, noun_chunks: null,
  } }));
  const matcher = new DependencyMatcher();
  matcher.add('rule', [singleton]);
  await assert.rejects(matcher.findMatches(doc));
  matcher.remove('rule');
  matcher.add('lemma', [{ nodes: [{ id: 'a', constraints: [{ attribute: 'lemma', predicate: { kind: 'equals', value: 'a' } }] }] }]);
  await assert.rejects(matcher.findMatches(tiny()));
});
