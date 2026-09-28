/** Retain and recover the exact tested npm release set without replacing assets. */
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {basename, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {releasePackages} from './release-publish.mts';

export const manifestName = 'npm-release.json';
interface Asset {name: string; integrity: string}
export interface Manifest {schema: 1; tag: string; sha: string; files: Asset[]}
export type Gh = (args: string[]) => string;
const gh: Gh = args => execFileSync('gh',args,{encoding:'utf8'});
const stableTag = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function checkTag(tag: string): void {
  if (!stableTag.test(tag)) throw new Error('Expected stable vX.Y.Z tag');
}
function integrity(path: string): string {
  return 'sha512-'+createHash('sha512').update(readFileSync(path)).digest('base64');
}
export function validateManifest(value: unknown, tag: string): Manifest {
  checkTag(tag);
  if (!record(value) || value.schema !== 1 || value.tag !== tag || typeof value.sha !== 'string' ||
      !/^[0-9a-f]{40}$/.test(value.sha) || !Array.isArray(value.files) || !value.files.length) throw new Error('Invalid npm release manifest');
  const files: Asset[] = value.files.map((file: unknown) => {
    if (!record(file) || typeof file.name !== 'string' || !/^[a-z0-9][a-z0-9._-]*\.tgz$/.test(file.name) ||
        typeof file.integrity !== 'string' || !/^sha512-[A-Za-z0-9+/]{86}==$/.test(file.integrity)) throw new Error('Invalid release asset');
    return {name:file.name,integrity:file.integrity};
  });
  if (new Set(files.map(file=>file.name)).size !== files.length) throw new Error('Duplicate release asset');
  return {schema:1,tag,sha:value.sha,files};
}
export function verify(directory: string, tag: string): Manifest {
  const manifest = validateManifest(JSON.parse(readFileSync(join(directory,manifestName),'utf8')),tag);
  // Check the complete byte inventory before invoking any archive parser.
  const names = readdirSync(directory).filter(name=>name.endsWith('.tgz'));
  if (names.length !== manifest.files.length || manifest.files.some(file=>!names.includes(file.name))) throw new Error('Incomplete release manifest');
  for (const file of manifest.files) {
    if (integrity(join(directory,file.name)) !== file.integrity) throw new Error(`Release checksum mismatch: ${file.name}`);
  }
  const packages = releasePackages(directory);
  if (packages.length !== manifest.files.length) throw new Error('Incomplete release manifest');
  for (const pkg of packages) {
    const asset = manifest.files.find(file=>file.name === basename(pkg.path));
    if (!asset || asset.integrity !== pkg.integrity || pkg.version !== tag.slice(1)) throw new Error('Release checksum or version mismatch');
  }
  return manifest;
}
export function seal(directory: string, tag: string, sha: string): void {
  const files = releasePackages(directory).map(pkg=>({name:basename(pkg.path),integrity:pkg.integrity})).sort((a,b)=>a.name.localeCompare(b.name));
  const manifest = validateManifest({schema:1,tag,sha,files},tag);
  if (releasePackages(directory).some(pkg=>pkg.version !== tag.slice(1))) throw new Error('Release version mismatch');
  writeFileSync(join(directory,manifestName),JSON.stringify(manifest,null,2)+'\n',{flag:'wx'});
}
interface Release {immutable: boolean; assets: string[]}
function release(tag: string, invoke: Gh): Release {
  checkTag(tag);
  const value: unknown = JSON.parse(invoke(['api',`repos/{owner}/{repo}/releases/tags/${tag}`]));
  if (!record(value) || value.tag_name !== tag || value.draft !== false || value.prerelease !== false ||
      typeof value.immutable !== 'boolean' || !Array.isArray(value.assets)) throw new Error('Expected published stable release');
  const assets = value.assets.map((asset: unknown) => {
    if (!record(asset) || typeof asset.name !== 'string') throw new Error('Invalid release asset metadata');
    return asset.name;
  });
  return {immutable:value.immutable,assets};
}
export function upload(directory: string, tag: string, invoke: Gh = gh): void {
  const manifest = verify(directory,tag);
  const remote = release(tag,invoke);
  const names = [...manifest.files.map(file=>file.name),manifestName];
  const missing = names.filter(name=>!remote.assets.includes(name));
  if (remote.immutable && missing.length) throw new Error('Immutable release cannot accept assets; prepare a new release');
  const scratch = mkdtempSync(join(tmpdir(),'spars-npm-'));
  try {
    // Preflight every existing file before any upload. The manifest is uploaded last.
    for (const name of names.filter(name=>remote.assets.includes(name))) {
      invoke(['release','download',tag,'--pattern',name,'--dir',scratch]);
      if (integrity(join(scratch,name)) !== integrity(join(directory,name))) throw new Error(`Release asset differs: ${name}; retained files must not be replaced`);
    }
    for (const name of missing) invoke(['release','upload',tag,resolve(directory,name)]);
  } finally {rmSync(scratch,{recursive:true,force:true});}
}
export function download(directory: string, tag: string, invoke: Gh = gh): void {
  const remote = release(tag,invoke);
  if (!remote.assets.includes(manifestName)) throw new Error('Release has no completed npm asset set');
  mkdirSync(directory,{recursive:true});
  if (readdirSync(directory).length) throw new Error('Download directory must be empty');
  invoke(['release','download',tag,'--pattern',manifestName,'--dir',directory]);
  const manifest = validateManifest(JSON.parse(readFileSync(join(directory,manifestName),'utf8')),tag);
  for (const file of manifest.files) {
    if (!remote.assets.includes(file.name)) throw new Error(`Missing release asset: ${file.name}`);
  }
  for (const file of manifest.files) invoke(['release','download',tag,'--pattern',file.name,'--dir',directory]);
  verify(directory,tag);
}
export function runAssets(args: string[], invoke: Gh = gh, publish: (directory: string) => void = directory => {
  execFileSync(process.execPath,['bindings/node/scripts/release-publish.mts',resolve(directory)],{stdio:'inherit'});
}, log: (message: string) => void = console.log): void {
  const [mode,tag,output,sha] = args;
  if (!tag || !output) throw new Error('Usage: release-assets.mts seal|upload|download|recover TAG DIRECTORY [SHA]');
  checkTag(tag);
  let directory = output;
  if (output === '-' && (mode === 'download' || mode === 'recover')) {
    mkdirSync('target/npm-recovery',{recursive:true});
    directory = mkdtempSync(join('target/npm-recovery',tag+'-'));
  }
  if (mode === 'seal' && sha) seal(directory,tag,sha);
  else if (mode === 'upload') upload(directory,tag,invoke);
  else if (mode === 'download' || mode === 'recover') {
    download(directory,tag,invoke);
    log(`Verified npm release files: ${directory}`);
    if (mode === 'recover') publish(directory);
  } else throw new Error('Invalid asset command');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) runAssets(process.argv.slice(2));
