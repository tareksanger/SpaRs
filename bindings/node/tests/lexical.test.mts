import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { before, test } from 'node:test';
import { loadModel, Model } from '../index.js';
import type { Lexeme } from '../index.js';
import { array, modelPath, record, root } from './fixtures.mts';

let model: Model;
before(async () => { model = await loadModel(modelPath); });

function string(value: unknown): string { assert.ok(typeof value === 'string'); return value; }
function boolean(value: unknown): boolean { assert.ok(typeof value === 'boolean'); return value; }
function bigint(value: unknown): bigint { assert.ok(typeof value === 'bigint'); return value; }

function reference(value: unknown): Lexeme {
  const lex = record(value);
  return {
    orth: bigint(lex.orth), norm: string(lex.norm), shape: string(lex.shape),
    prefix: string(lex.prefix), suffix: string(lex.suffix),
    isAlpha: boolean(lex.is_alpha), isDigit: boolean(lex.is_digit),
    isLower: boolean(lex.is_lower), isUpper: boolean(lex.is_upper),
    isTitle: boolean(lex.is_title), isSpace: boolean(lex.is_space),
    isAscii: boolean(lex.is_ascii), isPunct: boolean(lex.is_punct),
    isCurrency: boolean(lex.is_currency), isStop: boolean(lex.is_stop),
    isBracket: boolean(lex.is_bracket), isQuote: boolean(lex.is_quote),
    isLeftPunct: boolean(lex.is_left_punct), isRightPunct: boolean(lex.is_right_punct),
    likeNum: boolean(lex.like_num), likeEmail: boolean(lex.like_email),
    likeUrl: boolean(lex.like_url), hasVector: boolean(lex.has_vector),
  };
}

test('all lexical fields and exact uint64 hashes match the frozen official reference', () => {
  // Node 24 exposes each primitive's original JSON spelling: converting an already
  // parsed Number would silently round the reference hashes before comparison.
  const parsed: unknown = JSON.parse(readFileSync(`${root}fixtures/lexical.expected.json`, 'utf8'),
    (key: string, value: unknown, context?: { source: string }): unknown => {
      if (key !== 'orth') return value;
      assert.ok(context && /^\d+$/.test(context.source));
      return BigInt(context.source);
    });
  const cases = array(parsed);
  assert.equal(cases.length, 4374);
  let largeIds = 0;
  for (const value of cases) {
    const entry = record(value);
    const text = string(entry.text);
    const expected = reference(entry.expected);
    if (expected.orth > BigInt(Number.MAX_SAFE_INTEGER)) largeIds++;
    assert.deepEqual(model.lexeme(text), expected, JSON.stringify(text));
  }
  assert.ok(largeIds > 0, 'must exercise IDs outside the exact Number range');
});

test('Unicode, empty strings and embedded NUL retain lexical boundaries', () => {
  for (const text of ['', '😀', '👩‍🔬', 'Café', 'e\u0301', 'a\0b', '𰀀']) {
    const lex = model.lexeme(text);
    const points = Array.from(text);
    assert.equal(lex.prefix, points.slice(0, 1).join(''));
    assert.equal(lex.suffix, points.slice(-3).join(''));
    assert.equal(typeof lex.orth, 'bigint');
    assert.deepEqual(model.lexeme(text), lex);
  }
});

test('lexical queries reject malformed UTF-16 and non-string inputs', () => {
  for (const text of ['\ud800', '\udc00', 'a\ud800b', '\udc00\ud800']) {
    assert.throws(() => model.lexeme(text), { code: 'SPARS_INVALID_TEXT' });
  }
  for (const value of [undefined, null, 42, {}, ['word']]) {
    assert.throws(() => Reflect.apply(model.lexeme, model, [value]));
  }
});

test('mutating an owned lexical result cannot affect repeated queries or inference', async () => {
  const expected = model.lexeme('London');
  const changed = model.lexeme('London');
  changed.orth = 0n;
  changed.norm = 'changed';
  changed.isAlpha = false;
  assert.deepEqual(model.lexeme('London'), expected);
  const pending = model.process('London is a city.');
  for (let i = 0; i < 20; i++) assert.deepEqual(model.lexeme('London'), expected);
  assert.equal((await pending).tokens[0]?.text, 'London');
  assert.deepEqual(model.lexeme('London'), expected);
});
