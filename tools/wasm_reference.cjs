// Development-only secondary oracle; never imported by the native library.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve('reference/wasm');
const wasm_bindgen = require(path.join(root, 'bindings.js'));
wasm_bindgen.initSync({module:fs.readFileSync(path.join(root,'WASM'))});
const model = new wasm_bindgen.SpacyModel(
  fs.readFileSync(path.join(root,'MAN'),'utf8'),
  fs.readFileSync(path.join(root,'ST')),
  fs.readFileSync(path.join(root,'K2R'),'utf8'),
  fs.readFileSync(path.join(root,'R2W'),'utf8'));
const inputs=JSON.parse(fs.readFileSync(process.argv[2] || 'fixtures/development.json','utf8'));
console.log(JSON.stringify(inputs.map(text=>({text,output:JSON.parse(model.processJson(text))})),null,2));
model.free();
