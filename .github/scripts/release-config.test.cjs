const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
// Initialize upstream registries before importing a concrete strategy.
require('../../tools/node_modules/release-please');
const { buildStrategy } = require('../../tools/node_modules/release-please/build/src/factory');
const { Version } = require('../../tools/node_modules/release-please/build/src/version');
const { parseConventionalCommits } = require('../../tools/node_modules/release-please/build/src/commit');
const toml = require('../../tools/node_modules/@iarna/toml');
const config = JSON.parse(readFileSync('release-please-config.json', 'utf8')).packages['.'];
const github = {
  repository: { owner: 'owner', repo: 'repo', defaultBranch: 'main' },
  getFileContentsOnBranch: async path => ({ parsedContent: readFileSync(path, 'utf8') }),
};
function strategy() {
  return buildStrategy({ releaseType: config['release-type'], github, targetBranch: 'main', path: '.',
    packageName: config['package-name'], includeComponentInTag: config['include-component-in-tag'],
    bumpMinorPreMajor: config['bump-minor-pre-major'],
    bumpPatchForMinorPreMajor: config['bump-patch-for-minor-pre-major'], extraFiles: config['extra-files'], versionFile: config['version-file'], initialVersion: config['initial-version'] });
}
const commits = message => parseConventionalCommits([{ sha: 'a'.repeat(40), message, files: ['Cargo.toml'] }]);

test('pinned workspace strategy proposes 0.1.0 with versioned notes on first release', async () => {
  assert.equal(config['release-type'], 'simple');
  const pr = await (await strategy()).buildReleasePullRequest(commits('feat(ci): automate reviewed source releases'));
  assert.ok(pr);
  assert.equal(pr.version.toString(), '0.1.0');
  const changelog = pr.updates.find(update => update.path === 'CHANGELOG.md');
  assert.ok(changelog);
  assert.match(changelog.updater.updateContent('# Changelog\n'), /^## 0\.1\.0 /m);
});

test('upstream updater changes only project versions in every dependent lockfile', async () => {
  const s = await strategy();
  const updates = (await s.extraFileUpdates(Version.parse('0.42.0'), new Map(), 'YYYY-MM-DD'))
    .filter(update => update.path.endsWith('Cargo.lock'));
  assert.deepEqual(updates.map(update => update.path).sort(),
    ['Cargo.lock', 'consumer/Cargo.lock']);
  for (const update of updates) {
    const input = readFileSync(update.path, 'utf8');
    const before = toml.parse(input);
    const after = toml.parse(update.updater.updateContent(input));
    const expected = structuredClone(before);
    const matches = expected.package.filter(pkg => ['spars-nlp', 'spars-node', 'spars-model'].includes(pkg.name));
    assert.ok(matches.some(pkg => pkg.name === 'spars-nlp'));
    for (const pkg of matches) pkg.version = '0.42.0';
    assert.deepEqual(after, expected, update.path);
  }
});

test('configured pre-1.0 and stable version bumps follow documented policy', async () => {
  for (const [current, message, expected] of [
    ['0.1.0', 'fix: correct output', '0.1.1'],
    ['0.1.0', 'feat: add output', '0.2.0'],
    ['0.1.0', 'feat!: change output', '0.2.0'],
    ['1.0.0', 'feat!: change output', '2.0.0'],
  ]) {
    assert.equal((await strategy()).versioningStrategy.bump(Version.parse(current), commits(message)).toString(), expected);
  }
});

test('release PR synchronizes Rust, installer and Node manifests without changing publication policy', async () => {
  const { TagName } = require('../../tools/node_modules/release-please/build/src/util/tag-name');
  const latest = { tag: TagName.parse('v0.1.0'), sha: 'b'.repeat(40), notes: 'previous' };
  const pr = await (await strategy()).buildReleasePullRequest(commits('feat: new behavior'), latest);
  assert.ok(pr);
  assert.equal(pr.version.toString(), '0.2.0');
  const lockUpdates = pr.updates.filter(update => update.path === 'Cargo.lock');
  assert.equal(lockUpdates.length, 1);
  const lockInput = readFileSync('Cargo.lock', 'utf8');
  const expectedLock = toml.parse(lockInput);
  const local = expectedLock.package.filter(pkg => ['spars-nlp', 'spars-model', 'spars-node'].includes(pkg.name));
  assert.equal(local.length, 3);
  for (const pkg of local) pkg.version = '0.2.0';
  assert.deepEqual(toml.parse(lockUpdates[0].updater.updateContent(lockInput)), expectedLock);
  const rootManifest = pr.updates.find(update => update.path === 'Cargo.toml');
  assert.equal(rootManifest, undefined, 'The virtual workspace has no package version');
  const versionUpdate = pr.updates.find(update => update.path === 'version.txt');
  assert.ok(versionUpdate);
  assert.equal(versionUpdate.updater.updateContent(readFileSync('version.txt', 'utf8')), '0.2.0\n');
  const targets = ['crates/spars/Cargo.toml', 'bindings/node/Cargo.toml', 'installer/Cargo.toml',
    'bindings/node/package.json', 'bindings/node/package-lock.json'];
  for (const path of targets) {
    const update = pr.updates.find(update => update.path === path);
    assert.ok(update, path);
    assert.equal(pr.updates.filter(item => item.path === path).length, 1, 'Extra updates must compose');
    const input = readFileSync(path, 'utf8');
    const parse = path.endsWith('.json') ? JSON.parse : toml.parse;
    const before = parse(input);
    const expected = structuredClone(before);
    if (path.endsWith('.toml')) expected.package.version = '0.2.0';
    else {
      expected.version = '0.2.0';
      if (expected.packages) expected.packages[''].version = '0.2.0';
    }
    assert.deepEqual(parse(update.updater.updateContent(input)), expected, path);
  }
});
