import assert from 'node:assert/strict';
import { loadModel, __napiBindingTarget } from '../index.js';

const path = process.argv[2];
assert.ok(path, 'usage: node bindings/node/scripts/smoke.mts MODEL_DIRECTORY');
assert.equal(__napiBindingTarget, 'native');
const model = await loadModel(path);
const doc = await model.process('Alice works in London.');
assert.equal(doc.tokens[0]?.text, 'Alice');
assert.ok(doc.tokens.every(token => token.lemma !== null && token.head !== null));
assert.ok(doc.entities?.some(entity => entity.label === 'GPE'));
assert.ok(doc.sentences?.length);
assert.ok(doc.nounChunks?.length);
console.log('PASS native Node inference');
