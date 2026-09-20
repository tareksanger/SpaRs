import assert from 'node:assert/strict';
import { test } from 'node:test';
import { restrictConstructor } from '../scripts/declarations.mts';

test('declaration correction preserves methods and rejects unexpected generator changes', () => {
  const source = 'export declare class Model {\n  process(text: string): Promise<Document>\n}';
  const expected = source.replace('{', '{\n  private constructor()');
  assert.equal(restrictConstructor(source), expected);
  assert.equal(restrictConstructor(expected), expected);
  for (const bad of ['', source + source, 'export declare class Model { constructor() }']) {
    assert.throws(() => restrictConstructor(bad));
  }
});
