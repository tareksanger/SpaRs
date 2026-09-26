import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { stageRelease, targets } from '../scripts/release.mts';

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), 'spars-release-'));
  for (const target of targets) {
    const dir = join(root, 'inputs', target.suffix);
    mkdirSync(dir, {recursive:true});
    for (const [name, content] of Object.entries({
      'index.js': 'module.exports = {};', 'index.d.ts': 'export {};',
      'units.d.ts': 'export {};', 'spars.mjs': '#!/usr/bin/env node\n',
      'README.md': 'Usage', 'LICENSE': 'MIT', 'THIRD_PARTY_NOTICES.md': 'Notices',
      'DEPENDENCY_LICENSES.txt': 'Dependency notices',
      [`spars-node.${target.suffix}.node`]: 'binary',
      'release.json': JSON.stringify({name:'@spars/node',version:'0.2.0',sha:'a'.repeat(40),target:target.suffix}),
    })) writeFileSync(join(dir, name), content);
    mkdirSync(join(dir, 'licenses'));
    writeFileSync(join(dir, 'licenses', 'spacy-MIT.txt'), 'upstream license');
  }
  return root;
}

test('stage creates a missing output directory with exact platform dependencies and notices', () => {
  const root = fixture();
  try {
    const out = join(root, 'new', 'packages');
    stageRelease(join(root, 'inputs'), out, 'a'.repeat(40), '0.2.0');
    const main: unknown = JSON.parse(readFileSync(join(out, 'main/package.json'), 'utf8'));
    assert.ok(typeof main === 'object' && main !== null && 'optionalDependencies' in main);
    assert.deepEqual(main.optionalDependencies, {
      '@spars/node-linux-x64-gnu':'0.2.0', '@spars/node-darwin-arm64':'0.2.0',
    });
    assert.ok(!existsSync(join(out, 'main/spars-node.darwin-arm64.node')));
    for (const target of targets) {
      const manifest: unknown = JSON.parse(readFileSync(join(out, target.suffix, 'package.json'), 'utf8'));
      assert.ok(typeof manifest === 'object' && manifest !== null && 'os' in manifest && 'cpu' in manifest);
      assert.deepEqual(manifest.os, [target.os]);
      assert.deepEqual(manifest.cpu, [target.cpu]);
      assert.deepEqual('libc' in manifest ? manifest.libc : undefined,target.libc ? [target.libc] : undefined);
      assert.ok(existsSync(join(out, target.suffix, 'DEPENDENCY_LICENSES.txt')));
    }
    assert.throws(() => stageRelease(join(root, 'inputs'), out, 'a'.repeat(40), '0.2.0'), /already exists/);
  } finally { rmSync(root, {recursive:true,force:true}); }
});

for (const problem of ['missing binary', 'extra binary', 'wrong revision', 'wrong version', 'wrapper mismatch', 'missing notices']) {
  test(`staging rejects ${problem} before creating output`, () => {
    const root = fixture();
    try {
      const dir = join(root, 'inputs/darwin-arm64');
      if (problem === 'missing binary') rmSync(join(dir, 'spars-node.darwin-arm64.node'));
      if (problem === 'extra binary') writeFileSync(join(dir, 'unexpected.node'), 'binary');
      if (problem === 'wrong revision' || problem === 'wrong version') writeFileSync(join(dir, 'release.json'), JSON.stringify({name:'@spars/node',version:problem === 'wrong version' ? '0.1.0' : '0.2.0',sha:(problem === 'wrong revision' ? 'b' : 'a').repeat(40),target:'darwin-arm64'}));
      if (problem === 'wrapper mismatch') writeFileSync(join(dir, 'index.js'), 'different');
      if (problem === 'missing notices') rmSync(join(dir, 'DEPENDENCY_LICENSES.txt'));
      assert.throws(() => stageRelease(join(root,'inputs'),join(root,'output'),'a'.repeat(40),'0.2.0'));
      assert.ok(!existsSync(join(root,'output')));
    } finally { rmSync(root,{recursive:true,force:true}); }
  });
}

test('Cargo notices include nested licenses and reject incomplete graphs, texts, and unreviewed provenance', async () => {
  const {licenseTexts} = await import('../scripts/release.mts');
  const root = mkdtempSync(join(tmpdir(),'spars-notices-'));
  try {
    const dir=join(root,'dependency');
    mkdirSync(join(dir,'src/polyfill/once_cell'),{recursive:true});
    writeFileSync(join(dir,'LICENSE'),'Root notice');
    writeFileSync(join(dir,'src/polyfill/once_cell/LICENSE-MIT'),'Nested copyright must survive');
    const dependency={id:'dependency',name:'ring',version:'0.17.14',source:'registry',manifest_path:join(dir,'Cargo.toml'),license:'MIT',repository:'upstream'};
    const metadata={packages:[{id:'binding',name:'spars-node',version:'0.2.0',source:null,manifest_path:join(root,'Cargo.toml')},dependency],resolve:{nodes:[{id:'binding',dependencies:['dependency']},{id:'dependency',dependencies:[]}]}};
    const text=licenseTexts(metadata,root);
    assert.match(text,/Root notice/); assert.match(text,/Nested copyright must survive/);
    assert.ok(!text.includes(root),'No host paths in distributed notices');
    const broken=structuredClone(metadata);broken.resolve.nodes.pop();
    assert.throws(()=>licenseTexts(broken,root),/Incomplete dependency graph/);
    rmSync(join(dir,'LICENSE'));rmSync(join(dir,'src'),{recursive:true});
    assert.throws(()=>licenseTexts(metadata,root),/Missing license texts/);
    dependency.name='napi';dependency.repository='https://github.com/napi-rs/napi-rs';
    writeFileSync(join(dir,'.cargo_vcs_info.json'),JSON.stringify({git:{sha1:'b'.repeat(40)}}));
    assert.throws(()=>licenseTexts(metadata,root),/Review changed NAPI-RS/);
    writeFileSync(join(dir,'.cargo_vcs_info.json'),JSON.stringify({git:{sha1:'2763e12efc855748485129952a6ccb97ac991c06'}}));
    mkdirSync(join(root,'licenses'));writeFileSync(join(root,'licenses/napi-rs-MIT.txt'),'Pinned upstream notice');
    assert.match(licenseTexts(metadata,root),/Pinned upstream notice/);
  } finally {rmSync(root,{recursive:true,force:true});}
});
