import assert from 'node:assert/strict';
import { test } from 'node:test';
import { restrictConstructor, addExecutionLoader, addExecutionTypes } from '../scripts/declarations.mts';

test('declaration correction preserves methods and rejects unexpected generator changes', () => {
  const source = 'export declare class Model {\n  process(text: string): Promise<Document>\n}';
  const expected = source.replace('{', '{\n  private constructor()');
  assert.equal(restrictConstructor(source), expected);
  assert.equal(restrictConstructor(expected), expected);
  for (const bad of ['', source + source, 'export declare class Model { constructor() }']) {
    assert.throws(() => restrictConstructor(bad));
  }
});

test('execution wrapper is generated once and rejects changed loader contracts', () => {
  const loader = 'module.exports.Model = nativeBinding.Model';
  const wrapped = addExecutionLoader(loader);
  assert.match(wrapped, /execution\.cjs/);
  assert.match(wrapped, /module.exports.configureExecution/);
  assert.match(wrapped, /module.exports.configureInputLimits/);
  assert.equal(addExecutionLoader(wrapped), wrapped);
  assert.throws(() => addExecutionLoader(''), /Expected native Model/);
  const declaration = addExecutionTypes('');
  assert.match(declaration, /configureExecution/);
  assert.match(declaration, /configureInputLimits/);
  assert.equal(addExecutionTypes(declaration), declaration);
});
