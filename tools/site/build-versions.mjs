// Build every published copy of the documentation site into target/site, and check each one.
//
// The root copy holds the latest release, each release has a copy under its tag, and `main` has a
// copy under "next". Until a release contains this builder, the root copy holds the checkout and
// there is no version menu.
import { execFileSync, spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isReleaseTag, NEXT_VERSION, REPOSITORY_DIRECTORY } from './paths.mjs';

// A release can be built only with the site settings it shipped with, and those must know how to
// build a versioned copy. This file's presence in the tag is that knowledge.
const BUILDER_PATH = 'tools/site/build-versions.mjs';
const SITE_FOLDER = 'target/site';
const SITE_CHECK = join(REPOSITORY_DIRECTORY, 'tools/site/check-site.mjs');
// Each release is unpacked here, built with its own tools, and removed.
const RELEASES_DIRECTORY = join(REPOSITORY_DIRECTORY, 'target/site-releases');
const CHECKOUT = 'checkout';

function git(...args) {
  return execFileSync('git', args, { cwd: REPOSITORY_DIRECTORY, encoding: 'utf8' });
}

// Release tags, newest first.
function releaseTags() {
  return git('tag', '--list', 'v*', '--sort=-version:refname').split('\n').filter(isReleaseTag);
}

function tagHasBuilder(tag) {
  const listed = git('ls-tree', '--name-only', tag, BUILDER_PATH);
  return listed.trim() === BUILDER_PATH;
}

// Decide which copies to build. `tags` are release tags, newest first. Each copy names its
// `version` (undefined for the root copy) and its `source`: a tag, or the checkout. The root copy
// comes first, because building it empties target/site.
export function planCopies(tags, hasBuilder) {
  const releases = tags.filter(hasBuilder);
  const skipped = tags.filter((tag) => !releases.includes(tag));
  const [latest] = releases;
  if (latest === undefined) {
    return { copies: [{ version: undefined, source: CHECKOUT }], menu: [], skipped };
  }
  return {
    copies: [
      { version: undefined, source: latest },
      ...releases.map((tag) => ({ version: tag, source: tag })),
      { version: NEXT_VERSION, source: CHECKOUT },
    ],
    menu: [...releases, NEXT_VERSION],
    skipped,
  };
}

function describe(copy) {
  return `${copy.version ?? 'root'} copy from ${copy.source === CHECKOUT ? 'this checkout' : `tag ${copy.source}`}`;
}

function run(command, args, directory, env = process.env) {
  const done = spawnSync(command, args, { cwd: directory, env, stdio: 'inherit' });
  if (done.status !== 0) throw new Error(`${command} ${args.join(' ')} failed in ${directory}.`);
}

// The variables that tell the site settings and the site check which copy they are working on.
function copyEnvironment(copy, menu) {
  const env = { ...process.env };
  delete env.SPARS_SITE_VERSION;
  delete env.SPARS_SITE_VERSIONS;
  if (copy.version !== undefined) env.SPARS_SITE_VERSION = copy.version;
  if (menu.length > 0) env.SPARS_SITE_VERSIONS = menu.join(',');
  return env;
}

// Build one copy with the site settings and tools in `directory`, into that directory's target/site.
function build(copy, menu, directory) {
  run(join(directory, 'tools/node_modules/.bin/vitepress'), ['build', 'tools/site'], directory, copyEnvironment(copy, menu));
}

// Check a copy where it is published, with this checkout's check, whichever release built it.
function check(copy, menu) {
  run(process.execPath, [SITE_CHECK], REPOSITORY_DIRECTORY, copyEnvironment(copy, menu));
}

// Unpack a release's files, as committed, and install the tools its lock file pins.
function unpackRelease(tag) {
  const directory = join(RELEASES_DIRECTORY, tag);
  rmSync(directory, { recursive: true, force: true });
  mkdirSync(directory, { recursive: true });
  run('bash', ['-o', 'pipefail', '-c', 'git archive "$1" | tar -x -C "$2"', 'unpack', tag, directory], REPOSITORY_DIRECTORY);
  run('npm', ['--prefix', 'tools', 'ci', '--ignore-scripts', '--no-audit', '--no-fund'], directory);
  return directory;
}

// Move a copy built inside an unpacked release to its place in this checkout's target/site.
function publish(copy, releaseDirectory) {
  const folder = join(SITE_FOLDER, copy.version ?? '');
  const destination = join(REPOSITORY_DIRECTORY, folder);
  rmSync(destination, { recursive: true, force: true });
  cpSync(join(releaseDirectory, folder), destination, { recursive: true });
}

function main() {
  const tags = releaseTags();
  const plan = planCopies(tags, tagHasBuilder);
  console.log(`Release tags: ${tags.join(', ') || 'none'}`);
  for (const tag of plan.skipped) console.log(`Skipping ${tag}: it has no ${BUILDER_PATH}, so it cannot build a versioned copy.`);
  console.log(`Version menu: ${plan.menu.join(', ') || 'off'}`);
  const unpacked = new Map();
  try {
    for (const copy of plan.copies) {
      console.log(`Building the ${describe(copy)}.`);
      if (copy.source === CHECKOUT) {
        build(copy, plan.menu, REPOSITORY_DIRECTORY);
      } else {
        if (!unpacked.has(copy.source)) unpacked.set(copy.source, unpackRelease(copy.source));
        const releaseDirectory = unpacked.get(copy.source);
        build(copy, plan.menu, releaseDirectory);
        publish(copy, releaseDirectory);
      }
      check(copy, plan.menu);
    }
  } finally {
    rmSync(RELEASES_DIRECTORY, { recursive: true, force: true });
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) main();
