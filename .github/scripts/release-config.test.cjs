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
    bumpPatchForMinorPreMajor: config['bump-patch-for-minor-pre-major'], extraFiles: config['extra-files'] });
}
const commits = message => parseConventionalCommits([{ sha: 'a'.repeat(40), message, files: ['Cargo.toml'] }]);

test('pinned Rust strategy proposes 0.1.0 with versioned notes on first release', async () => {
  assert.equal(config['release-type'], 'rust');
  const pr = await (await strategy()).buildReleasePullRequest(commits('feat(ci): automate reviewed source releases'));
  assert.ok(pr);
  assert.equal(pr.version.toString(), '0.1.0');
  const changelog = pr.updates.find(update => update.path === 'CHANGELOG.md');
  assert.ok(changelog);
  assert.match(changelog.updater.updateContent('# Changelog\n'), /^## 0\.1\.0 /m);
});

test('upstream updater changes only spars-nlp in every dependent lockfile', async () => {
  const s = await strategy();
  const updates = await s.extraFileUpdates(Version.parse('0.42.0'), new Map(), 'YYYY-MM-DD');
  assert.deepEqual(updates.map(update => update.path).sort(),
    ['consumer/Cargo.lock', 'installer/Cargo.lock', 'bindings/node/Cargo.lock'].sort());
  for (const update of updates) {
    const input = readFileSync(update.path, 'utf8');
    const before = toml.parse(input);
    const after = toml.parse(update.updater.updateContent(input));
    const expected = structuredClone(before);
    const matches = expected.package.filter(pkg => pkg.name === 'spars-nlp');
    assert.equal(matches.length, 1);
    matches[0].version = '0.42.0';
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
