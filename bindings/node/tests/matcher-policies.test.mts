import assert from 'node:assert/strict';
import { before, test } from 'node:test';
import { configureExecution, configureInputLimits, DependencyMatcher, loadModel, Model, NativeDocument, PhraseMatcher, TokenMatcher } from '../index.js';
import { modelPath } from './fixtures.mts';

let model: Model;
before(async () => { model = await loadModel(modelPath); });

interface Matcher {
  findMatches(document: NativeDocument): Promise<unknown>;
  remove(rule: string): void;
}
interface Case { matcher: Matcher; add(): void }
function matchers(doc: NativeDocument): Case[] {
  const phrase = new PhraseMatcher();
  const token = new TokenMatcher();
  const dependency = new DependencyMatcher();
  return [
    { matcher: phrase, add: () => phrase.add('r', [doc]) },
    { matcher: token, add: () => token.add('r', [{ tokens: [{ constraints: [], repetition: { kind: 'once' } }] }]) },
    { matcher: dependency, add: () => dependency.add('r', [{ nodes: [{ id: 'a', constraints: [] }] }]) },
  ];
}

test('all matchers protect queued rules, share admission and recover after completion', async () => {
  const doc = await model.processDocument('Alice runs.');
  for (const { matcher, add } of matchers(doc)) {
    add();
    configureExecution({ maxActive: 1, maxQueued: 1 });
    const active = model.processDocument('Alice visits London.');
    const queued = matcher.findMatches(doc);
    const settled = Promise.allSettled([active, queued]);
    try {
      assert.throws(add, { code: 'SPARS_BUSY' });
      assert.throws(() => matcher.remove('r'), { code: 'SPARS_BUSY' });
      const matcherOverflow = assert.rejects(matcher.findMatches(doc), { code: 'SPARS_BUSY' });
      const inferenceOverflow = assert.rejects(model.process('overflow'), { code: 'SPARS_BUSY' });
      await Promise.all([matcherOverflow, inferenceOverflow]);
      await queued;
      matcher.remove('r');
      add();
      assert.ok(Array.isArray(await matcher.findMatches(doc)));
    } finally { await settled; configureExecution(null); }
  }
});

test('failed searches release queued admission and matcher mutation protection', async () => {
  const raw = await model.processDocument('Alice', 'Tokenizer');
  const matcher = new DependencyMatcher();
  matcher.add('r', [{ nodes: [{ id: 'a', constraints: [] }] }]);
  configureExecution({ maxActive: 1, maxQueued: 1 });
  const active = model.process('start');
  const failed = matcher.findMatches(raw);
  const settled = Promise.allSettled([active, failed]);
  try {
    assert.throws(() => matcher.remove('r'), { code: 'SPARS_BUSY' });
    await assert.rejects(failed);
    matcher.remove('r');
    assert.deepEqual(await matcher.findMatches(raw), []);
  } finally { await settled; configureExecution(null); }
});

test('matcher input limits reject before admission and validate receivers and documents', async () => {
  const doc = await model.processDocument('😀', 'Tokenizer');
  configureInputLimits({ maxTextLength: 1, maxBatchSize: 1, maxBatchTextLength: 1 });
  try {
    for (const { matcher, add } of matchers(doc)) {
      add();
      await assert.rejects(matcher.findMatches(doc), { code: 'SPARS_INPUT_LIMIT' });
      matcher.remove('r');
      assert.throws(() => Reflect.apply(matcher.findMatches, {}, [doc]), TypeError);
      for (const input of [undefined, null, {}, doc.toObject(), 'text']) {
        assert.throws(() => Reflect.apply(matcher.findMatches, matcher, [input]), TypeError);
      }
    }
    configureExecution({ maxActive: 1, maxQueued: 0 }); // rejection must not occupy a slot
  } finally { configureExecution(null); configureInputLimits(null); }
});

test('unknown token pattern fields reject at every nesting level without partial registration', () => {
  const matcher = new TokenMatcher();
  const constraint = { attribute: 'text', predicate: { kind: 'equals', value: 'x' } };
  const item = { constraints: [constraint], repetition: { kind: 'once' } };
  const good = { tokens: [item] };
  const malformed: unknown[] = [
    { ...good, callback: true },
    { tokens: [{ ...item, OP: '!' }] },
    { tokens: [{ ...item, constraints: [{ ...constraint, LOWER: 'x' }] }] },
    { tokens: [{ ...item, constraints: [{ ...constraint, predicate: { ...constraint.predicate, REGEX: 'x' } }] }] },
    { tokens: [{ ...item, repetition: { kind: 'once', greedy: true } }] },
    { ...good, [Symbol('unsupported')]: true },
  ];
  for (const pattern of malformed) {
    assert.throws(() => Reflect.apply(matcher.add, matcher, ['bad', [good, pattern]]), { code: 'SPARS_INVALID_PATTERN' });
    assert.equal(matcher.contains('bad'), false);
    assert.equal(matcher.size, 0);
  }
});

test('unknown dependency fields reject at every nesting level without partial registration', () => {
  const matcher = new DependencyMatcher();
  const constraint = { attribute: 'text', predicate: { kind: 'equals', value: 'x' } };
  const first = { id: 'a', constraints: [constraint] };
  const link = { left: 'a', relation: '>' };
  const second = { id: 'b', constraints: [], link };
  const good = { nodes: [first, second] };
  const malformed: unknown[] = [
    { ...good, callback: true },
    { nodes: [{ ...first, RIGHT_ATTRS: {} }, second] },
    { nodes: [{ ...first, constraints: [{ ...constraint, OP: '!' }] }, second] },
    { nodes: [{ ...first, constraints: [{ ...constraint, predicate: { ...constraint.predicate, REGEX: 'x' } }] }, second] },
    { nodes: [first, { ...second, link: { ...link, REL_OP: '<' } }] },
    { nodes: [{ ...first, [Symbol('unsupported')]: true }, second] },
  ];
  for (const pattern of malformed) {
    assert.throws(() => Reflect.apply(matcher.add, matcher, ['bad', [good, pattern]]), { code: 'SPARS_INVALID_PATTERN' });
    assert.equal(matcher.contains('bad'), false);
    assert.equal(matcher.size, 0);
  }
});
