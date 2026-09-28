/** Retry publication only when already-published bytes match the tested tarballs. */
import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { setTimeout } from 'node:timers/promises';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { targets } from './release.mts';

export interface ReleasePackage { name: string; version: string; path: string; integrity: string }
export interface Registry {
  integrity(pkg: ReleasePackage): Promise<string | undefined>;
  publish(pkg: ReleasePackage): Promise<void>;
}

export class PendingPublicationError extends Error {}
export interface PublishOptions {
  wait?: (milliseconds: number) => Promise<void>;
  report?: (message: string) => void;
}

export async function publishPackages(packages: readonly ReleasePackage[], registry: Registry, options: PublishOptions = {}): Promise<void> {
  const wait = options.wait ?? (async milliseconds => { await setTimeout(milliseconds); });
  const report = options.report ?? (() => {});
  // Preflight the complete set before the first mutation.
  const existing = await Promise.all(packages.map(pkg => registry.integrity(pkg)));
  for (const [i,pkg] of packages.entries()) {
    if (existing[i] !== undefined && existing[i] !== pkg.integrity) throw new Error(`Registry integrity mismatch: ${pkg.name}@${pkg.version}`);
  }
  for (const [i,pkg] of packages.entries()) {
    if (existing[i] !== undefined) { report(`Verified existing ${pkg.name}@${pkg.version}; skipping upload`); continue; }
    report(`Publishing ${i + 1}/${packages.length}: ${pkg.name}@${pkg.version}`);
    try { await registry.publish(pkg); }
    catch (error) {
      if (!(error instanceof PendingPublicationError)) throw error;
      report(`npm already has a staged upload for ${pkg.name}; waiting for registry confirmation`);
    }
    // npm can accept an upload several minutes before its version endpoint is visible.
    // Never upload again while waiting, and never advance on a checksum mismatch.
    for (let attempt = 0; ; attempt++) {
      const integrity = await registry.integrity(pkg);
      if (integrity === pkg.integrity) { report(`Confirmed ${pkg.name}@${pkg.version}`); break; }
      if (integrity !== undefined) throw new Error(`Registry integrity mismatch: ${pkg.name}@${pkg.version}`);
      if (attempt === 60) throw new Error(`Published integrity not yet confirmed: ${pkg.name}@${pkg.version} after 60 waits of 5 seconds; check npm processing or staged approval before retrying with the same tarballs`);
      report(`Waiting for npm to make ${pkg.name}@${pkg.version} available (${attempt + 1}/60)`);
      await wait(5000);
    }
  }
  report(`Confirmed all ${packages.length} release packages`);
}

export async function publishTarball(pkg: ReleasePackage): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const child = spawn('npm', ['publish', pkg.path, '--access', 'public', '--ignore-scripts'], {stdio:['inherit','inherit','pipe']});
    let tail = '';
    let stagedConflict = false;
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (chunk: string) => {
      process.stderr.write(chunk);
      tail = (tail + chunk).slice(-8192);
      stagedConflict ||= /Cannot publish over previously staged version/.test(tail);
    });
    child.on('error', reject);
    child.on('close', (code, signal) => {
      if (code === 0) resolve();
      else if (stagedConflict) reject(new PendingPublicationError(`npm has a staged version of ${pkg.name}@${pkg.version}`));
      else reject(new Error(`npm publish failed for ${pkg.name}@${pkg.version} (${signal ?? code})`));
    });
  });
}

export function releasePackages(directory: string): ReleasePackage[] {
  const files = readdirSync(directory).filter(file => file.endsWith('.tgz'));
  if (files.length !== targets.length + 1) throw new Error(`Expected exactly ${targets.length + 1} release tarballs`);
  const packages = files.map(file => {
    const path = resolve(directory,file);
    const value: unknown = JSON.parse(execFileSync('tar',['-xOf','./'+file,'package/package.json'],{cwd:resolve(directory),encoding:'utf8'}));
    if (typeof value !== 'object' || value === null || !('name' in value) || typeof value.name !== 'string' ||
        !/^@[a-z0-9-]+\/[a-z0-9-]+$/.test(value.name) || !('version' in value) || typeof value.version !== 'string' ||
        !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(value.version)) throw new Error('Invalid tarball manifest');
    return {name:value.name,version:value.version,path,integrity:'sha512-'+createHash('sha512').update(readFileSync(path)).digest('base64')};
  });
  const main = packages.filter(pkg => !targets.some(target => pkg.name.endsWith('-'+target.suffix)));
  if (main.length !== 1 || !main[0]) throw new Error('Expected one main package');
  const root = main[0];
  const platforms = targets.map(target => {
    const matches = packages.filter(pkg => pkg.name === `${root.name}-${target.suffix}` && pkg.version === root.version);
    if (matches.length !== 1 || !matches[0]) throw new Error('Missing or mismatched platform package');
    return matches[0];
  });
  return [...platforms,root];
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const directory = process.argv[2];
  if (!directory) throw new Error('Usage: release-publish.mts TARBALL_DIRECTORY');
  await publishPackages(releasePackages(directory), {
    async integrity(pkg) {
      const response = await fetch(`https://registry.npmjs.org/${encodeURIComponent(pkg.name)}/${pkg.version}`, {signal:AbortSignal.timeout(30000)});
      if (response.status === 404) return undefined;
      if (!response.ok) throw new Error(`Registry lookup failed: HTTP ${response.status}`);
      const value: unknown = await response.json();
      if (typeof value !== 'object' || value === null || !('dist' in value) || typeof value.dist !== 'object' || value.dist === null || !('integrity' in value.dist) || typeof value.dist.integrity !== 'string') throw new Error('Registry response lacks integrity');
      return value.dist.integrity;
    },
    publish: publishTarball,
  }, {report: console.log});
}
