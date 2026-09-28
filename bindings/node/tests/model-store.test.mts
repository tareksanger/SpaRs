import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { ModelStore } from '../index.js';

function hasCode(code: string): (error: unknown) => boolean {
  return (error: unknown): boolean => error instanceof Error && 'code' in error && error.code === code;
}

test('ModelStore registers and resolves direct children without loading weights', async () => {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'spars-store-')));
  try {
    const store = new ModelStore(root);
    assert.equal(store.root, root);
    await assert.rejects(store.resolve('example'), hasCode('SPARS_IO'));
    const first = join(root, 'first');
    const second = join(root, 'second');
    mkdirSync(first);
    mkdirSync(second);
    assert.equal(await store.register('example', first), undefined);
    assert.equal(await store.resolve('example'), first);
    assert.equal(readFileSync(join(root, '.spars', 'example'), 'utf8'), 'first\n');
    await store.register('example', second);
    assert.equal(await store.resolve('example'), second);
    for (const name of ['', '../escape', 'UPPER', 'a'.repeat(129)]) {
      await assert.rejects(store.resolve(name), hasCode('SPARS_INVALID_MODEL'));
      await assert.rejects(store.register(name, first), hasCode('SPARS_INVALID_MODEL'));
    }
    mkdirSync(join(first, 'nested'));
    await assert.rejects(store.register('example', join(first, 'nested')), hasCode('SPARS_INVALID_MODEL'));
    await assert.rejects(store.register('example', root), hasCode('SPARS_INVALID_MODEL'));
    writeFileSync(join(root, 'file'), 'not a directory');
    await assert.rejects(store.register('example', join(root, 'file')), hasCode('SPARS_INVALID_MODEL'));
    await assert.rejects(store.register('example', join(root, 'absent')), hasCode('SPARS_IO'));
    await assert.rejects(store.resolve('\ud800'), hasCode('SPARS_INVALID_TEXT'));
    await assert.rejects(store.register('example', '\ud800'), hasCode('SPARS_INVALID_TEXT'));
    await assert.rejects(store.register('example', ''), hasCode('SPARS_INVALID_MODEL'));
    writeFileSync(join(root, '.spars', 'example'), '../escape\n');
    await assert.rejects(store.resolve('example'), hasCode('SPARS_INVALID_MODEL'));
    writeFileSync(join(root, '.spars', 'example'), 'x'.repeat(4097));
    await assert.rejects(store.resolve('example'), hasCode('SPARS_INVALID_MODEL'));
    assert.throws(() => new ModelStore(''), hasCode('SPARS_INVALID_MODEL'));
    assert.throws(() => new ModelStore('\ud800'), hasCode('SPARS_INVALID_TEXT'));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('ModelStore captures environment and relative paths before queued work', () => {
  const project = realpathSync(mkdtempSync(join(tmpdir(), 'spars-store-paths-')));
  const binding = new URL('../index.js', import.meta.url).href;
  try {
    const source = `
      import assert from 'node:assert/strict';
      import { mkdirSync } from 'node:fs';
      import { join } from 'node:path';
      import { ModelStore } from ${JSON.stringify(binding)};
      const cwd = process.cwd();
      mkdirSync('models/installed', {recursive: true});
      mkdirSync('other');
      const store = ModelStore.discover();
      assert.equal(store.root, join(cwd, 'models'));
      assert.equal(new ModelStore('models').root, store.root);
      process.env.SPARS_MODEL_DIR = 'different';
      const registering = store.register('example', 'models/installed');
      process.chdir('other');
      await registering;
      assert.equal(await store.resolve('example'), join(cwd, 'models/installed'));
      process.env.SPARS_MODEL_DIR = '';
      assert.throws(() => ModelStore.discover(), error => error.code === 'SPARS_INVALID_MODEL');
    `;
    const result = spawnSync(process.execPath, ['--input-type=module', '-e', source], {
      cwd: project, env: { ...process.env, SPARS_MODEL_DIR: 'models' }, encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
  } finally {
    rmSync(project, { recursive: true, force: true });
  }
});

// Windows symlinks require additional privileges; the native store's platform
// tests cover its directory checks independently of Node's privilege model.
test('ModelStore rejects symbolic selection directories', { skip: process.platform === 'win32' }, async () => {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'spars-store-symlink-')));
  try {
    const store = new ModelStore(root);
    const installed = join(root, 'installed');
    const index = join(root, 'index');
    mkdirSync(installed);
    mkdirSync(index);
    writeFileSync(join(index, 'example'), 'installed\n');
    symlinkSync(index, join(root, '.spars'), 'dir');
    await assert.rejects(store.resolve('example'), hasCode('SPARS_INVALID_MODEL'));
    await assert.rejects(store.register('example', installed), hasCode('SPARS_INVALID_MODEL'));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
