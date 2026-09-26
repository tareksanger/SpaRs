/** Install the release tarballs through an isolated loopback registry and execute the installed API and CLI. */
import assert from 'node:assert/strict';
import { execFile, execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { targets } from './release.mts';
import {releasePackages} from './release-publish.mts';
import {createServer} from 'node:http';
import {promisify} from 'node:util';
const execute = promisify(execFile);

export async function smoke(tarballs: string, model: string): Promise<void> {
  const target = targets.find(item => item.os === process.platform && item.cpu === process.arch);
  assert.ok(target,'Unsupported smoke-test platform');
  const packages = readdirSync(tarballs).filter(file => file.endsWith('.tgz'));
  const native = packages.filter(file => file.includes(`-${target.suffix}-`));
  const main = packages.filter(file => !targets.some(item => file.includes(`-${item.suffix}-`)));
  assert.equal(native.length,1);
  assert.equal(main.length,1);
  const releases = releasePackages(tarballs);
  const project = mkdtempSync(join(tmpdir(),'spars-npm-smoke-'));
  const fetched = new Set<string>();
  let registryUrl = '';
  const server = createServer((request,response) => {
    const path = decodeURIComponent((request.url ?? '').slice(1));
    const pkg = releases.find(item => item.name === path || `tarball/${item.name}` === path);
    const platform = targets.find(item => pkg?.name.endsWith('-'+item.suffix));
    if (!pkg || !platform) { response.writeHead(404);response.end('{}');return; }
    if (path.startsWith('tarball/')) {
      fetched.add(platform.suffix);
      response.writeHead(200,{'Content-Type':'application/octet-stream'});response.end(readFileSync(pkg.path));return;
    }
    const version = {name:pkg.name,version:pkg.version,os:[platform.os],cpu:[platform.cpu],...(platform.libc ? {libc:[platform.libc]} : {}),dist:{tarball:`${registryUrl}/tarball/${encodeURIComponent(pkg.name)}`,integrity:pkg.integrity}};
    response.writeHead(200,{'Content-Type':'application/json'});
    response.end(JSON.stringify({name:pkg.name,'dist-tags':{latest:pkg.version},versions:{[pkg.version]:version}}));
  });
  try {
    await new Promise<void>((done,reject) => {server.once('error',reject);server.listen(0,'127.0.0.1',done);});
    const address = server.address();
    assert.ok(address && typeof address === 'object');
    registryUrl = `http://127.0.0.1:${address.port}`;
    writeFileSync(join(project,'package.json'),JSON.stringify({name:'release-consumer',private:true}));
    await execute('npm',['install','--registry',registryUrl,'--ignore-scripts','--no-audit','--no-fund','--cache',join(project,'cache'),resolve(tarballs,main[0]!)],{cwd:project});
    assert.deepEqual([...fetched],[target.suffix],'npm must fetch only the host optional dependency');
    const manifest: unknown = JSON.parse(readFileSync(new URL('../package.json',import.meta.url),'utf8'));
    assert.ok(typeof manifest === 'object' && manifest !== null && 'name' in manifest && typeof manifest.name === 'string');
    const name = manifest.name;
    const installed: unknown = JSON.parse(readFileSync(join(project,'node_modules',name,'package.json'),'utf8'));
    assert.ok(typeof installed === 'object' && installed !== null && 'optionalDependencies' in installed && 'version' in installed);
    assert.deepEqual(installed.optionalDependencies,Object.fromEntries(targets.map(item => [`${name}-${item.suffix}`,installed.version])));
    const nativeManifest: unknown = JSON.parse(readFileSync(join(project,'node_modules',`${name}-${target.suffix}`,'package.json'),'utf8'));
    assert.ok(typeof nativeManifest === 'object' && nativeManifest !== null && 'os' in nativeManifest && 'cpu' in nativeManifest);
    assert.deepEqual(nativeManifest.os,[target.os]);
    assert.deepEqual(nativeManifest.cpu,[target.cpu]);
    assert.deepEqual('libc' in nativeManifest ? nativeManifest.libc : undefined,target.libc ? [target.libc] : undefined);
    const source = `import assert from 'node:assert/strict';
import {loadModel, __napiBindingTarget} from ${JSON.stringify(name)};
assert.equal(__napiBindingTarget,'native');
const model = await loadModel(${JSON.stringify(resolve(model))});
const doc = await model.process('Alice works in London.');
assert.equal(doc.tokens[0]?.text,'Alice');
assert.ok(doc.tokens.every(token => token.lemma !== null && token.head !== null));
assert.ok(doc.entities?.some(entity => entity.label === 'GPE'));
assert.ok(doc.sentences?.length && doc.nounChunks?.length);
await assert.rejects(loadModel('missing_model'));
`;
    writeFileSync(join(project,'check.mjs'),source);
    execFileSync(process.execPath,['check.mjs'],{cwd:project,stdio:'inherit',env:{...process.env,NAPI_RS_ENFORCE_VERSION_CHECK:'1'}});
    const cli = execFileSync(process.execPath,[join(project,'node_modules',name,'bin/spars.mjs'),'--help'],{encoding:'utf8'});
    assert.match(cli,/spars download/);
    writeFileSync(join(project,'check.mts'),`import {loadModel} from ${JSON.stringify(name)};\nconst model = await loadModel('en_core_web_sm');\nconst text: string = (await model.process('Hello.')).tokens[0]!.text;\nvoid text;\n`);
    execFileSync(process.execPath,[fileURLToPath(new URL('../node_modules/typescript/bin/tsc',import.meta.url)),'--noEmit','--strict','--module','NodeNext','--moduleResolution','NodeNext','--target','ES2023','check.mts'],{cwd:project,stdio:'inherit'});
    console.log('PASS packed npm optional dependency, native inference, CLI, and TypeScript consumer');
  } finally { server.closeAllConnections();server.close();rmSync(project,{recursive:true,force:true}); }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [tarballs,model] = process.argv.slice(2);
  if (!tarballs || !model) throw new Error('Usage: release-smoke.mts TARBALL_DIRECTORY MODEL_DIRECTORY');
  await smoke(tarballs,model);
}
