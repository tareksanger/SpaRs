/** Retry publication only when already-published bytes match the tested tarballs. */
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { targets } from './release.mts';

export interface ReleasePackage { name: string; version: string; path: string; integrity: string }
export interface Registry {
  integrity(pkg: ReleasePackage): Promise<string | undefined>;
  publish(pkg: ReleasePackage): Promise<void>;
}

export async function publishPackages(packages: readonly ReleasePackage[], registry: Registry): Promise<void> {
  // Preflight the complete set before the first mutation.
  const existing = await Promise.all(packages.map(pkg => registry.integrity(pkg)));
  for (const [i,pkg] of packages.entries()) {
    if (existing[i] !== undefined && existing[i] !== pkg.integrity) throw new Error(`Registry integrity mismatch: ${pkg.name}@${pkg.version}`);
  }
  for (const [i,pkg] of packages.entries()) {
    if (existing[i] !== undefined) continue;
    await registry.publish(pkg);
    if (await registry.integrity(pkg) !== pkg.integrity) throw new Error(`Published integrity not yet confirmed: ${pkg.name}; rerun this job with the same tarballs`);
  }
}

export function releasePackages(directory: string): ReleasePackage[] {
  const files = readdirSync(directory).filter(file => file.endsWith('.tgz'));
  if (files.length !== targets.length + 1) throw new Error(`Expected exactly ${targets.length + 1} release tarballs`);
  const packages = files.map(file => {
    const path = resolve(directory,file);
    const value: unknown = JSON.parse(execFileSync('tar',['-xOf',path,'package/package.json'],{encoding:'utf8'}));
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
    async publish(pkg) {
      execFileSync('npm',['publish',join(pkg.path),'--access','public','--ignore-scripts'],{stdio:'inherit'});
    },
  });
}
