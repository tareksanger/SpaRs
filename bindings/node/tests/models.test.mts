import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, rmSync, existsSync, realpathSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
const binding = new URL('../index.js', import.meta.url).href;
const cli = fileURLToPath(new URL('../bin/spars.mjs', import.meta.url));
const archive = fileURLToPath(new URL('../../../assets/en_core_web_sm-3.8.0-py3-none-any.whl', import.meta.url));

test('download API, CLI, and named loading use the same environment directory', () => {
  const project = realpathSync(mkdtempSync(join(tmpdir(), 'spars-node-models-')));
  const store = join(project, 'models');
  try {
    const source = `
      import assert from 'node:assert/strict';
      import { mkdirSync, copyFileSync } from 'node:fs';
      import { downloadModel, loadModel } from ${JSON.stringify(binding)};
      let step = 'download';
      try {
      const promise = downloadModel('en_core_web_sm', {archive: ${JSON.stringify(archive)}});
      // Paths are captured when the API is called, not when its worker starts.
      process.env.SPARS_MODEL_DIR = ${JSON.stringify(join(project, 'wrong'))};
      const installed = await promise;
      assert.ok(installed.startsWith(${JSON.stringify(store)}));
      await assert.rejects(loadModel('en_core_web_sm'), /spars download/);
      step = 'explicit load';
      const explicit = await loadModel('en_core_web_sm', {path: ${JSON.stringify(store)}});
      assert.equal((await explicit.process('Alice visits London.')).tokens[0].text, 'Alice');
      process.env.SPARS_MODEL_DIR = ${JSON.stringify(store)};
      mkdirSync('en_core_web_sm');
      step = 'name load';
      const loading = loadModel('en_core_web_sm');
      process.env.SPARS_MODEL_DIR = ${JSON.stringify(join(project, 'wrong'))};
      const model = await loading;
      assert.ok((await model.process('Alice visits London.')).entities.length);
      step = 'invalid input';
      await assert.rejects(downloadModel('unknown_model'), /pinned release/);
      await assert.rejects(downloadModel('en_core_web_sm', {version:'9.0.0'}), /unsupported model release/);
      await assert.rejects(downloadModel('en_core_web_sm', {path:''}), /empty/);
      await assert.rejects(downloadModel('en_core_web_sm', {archive:'absent.whl'}));
      await assert.rejects(downloadModel('bad\\ud800'), error => error.code === 'SPARS_INVALID_TEXT');
      await assert.rejects(loadModel('en_core_web_sm', {path:''}), /empty/);
      await assert.rejects(loadModel('../absent'));
      step = 'relative path capture';
      copyFileSync(${JSON.stringify(archive)}, 'official.whl');
      const downloading = downloadModel('en_core_web_sm', {path:'relative models',archive:'official.whl'});
      mkdirSync('other'); process.chdir('other');
      const relativePath = await downloading;
      assert.ok(relativePath.startsWith(${JSON.stringify(join(project, 'relative models'))}));
      const relativeLoading = loadModel('en_core_web_sm', {path:'../relative models'});
      process.chdir('..');
      assert.ok((await (await relativeLoading).process('Hello.')).tokens.length);
      } catch (error) { throw new Error(step + ': ' + error.message, {cause:error}); }
    `;
    const result = spawnSync(process.execPath, ['--input-type=module', '-e', source], {
      cwd: project, env: {...process.env, SPARS_MODEL_DIR: store}, encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(existsSync(join(project, 'spars.json')), false);
    const run = spawnSync(process.execPath, [cli, 'download', 'en_core_web_sm', '--archive', archive], {
      cwd: project, env: {...process.env, SPARS_MODEL_DIR: store}, encoding: 'utf8',
    });
    assert.equal(run.status, 0, run.stderr);
    assert.ok(run.stdout.trim().startsWith(store));
    for (const args of [[], ['download'], ['download','en_core_web_sm','--path'], ['download','en_core_web_sm','--unknown','x'], ['download','en_core_web_sm','--path','a','--path','b']]) {
      assert.notEqual(spawnSync(process.execPath, [cli, ...args], {cwd:project}).status, 0);
    }
  } finally { rmSync(project, {recursive:true, force:true}); }
});

test('the npm CLI works from an installed package without TypeScript stripping', () => {
  const project = realpathSync(mkdtempSync(join(tmpdir(), 'spars-node-package-')));
  const nodeProject = fileURLToPath(new URL('../', import.meta.url));
  try {
    const packed = spawnSync('npm', ['pack', '--cache', join(project,'npm-cache'), '--offline', '--ignore-scripts', '--pack-destination', project], {cwd:nodeProject, encoding:'utf8'});
    assert.equal(packed.status, 0, packed.stderr);
    const tarball = packed.stdout.trim();
    assert.match(tarball, /^[a-zA-Z0-9._-]+\.tgz$/);
    const installed = spawnSync('npm', ['install', '--cache', join(project,'npm-cache'), '--offline', '--ignore-scripts', '--no-audit', '--no-fund', '--prefix', project, join(project,tarball)], {encoding:'utf8'});
    assert.equal(installed.status, 0, installed.stderr);
    const bin = join(project,'node_modules/@spars/node/bin/spars.mjs');
    const result = spawnSync(process.execPath, [bin,'download','en_core_web_sm','--archive',archive], {cwd:project, env:{...process.env,SPARS_MODEL_DIR:join(project,'models')},encoding:'utf8'});
    assert.equal(result.status, 0, result.stderr);
    const loaded = spawnSync(process.execPath,['--input-type=module','-e',"import {loadModel} from '@spars/node'; const m=await loadModel('en_core_web_sm'); if (!(await m.process('Hello.')).tokens.length) process.exit(1);"],{cwd:project,env:{...process.env,SPARS_MODEL_DIR:join(project,'models')},encoding:'utf8'});
    assert.equal(loaded.status,0,loaded.stderr);
  } finally { rmSync(project,{recursive:true,force:true}); }
});
