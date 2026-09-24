const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const publish = require('./release.cjs');

function fixture() {
  const sha = 'a'.repeat(40);
  const calls = [];
  const files = {
    '.release-please-manifest.json': '{".":"0.1.0"}',
    'CHANGELOG.md': '# Changelog\n\n## 0.1.0 (2026-01-01)\n\n### Features\n\n- Native inference\n\n## 0.0.1\n\nOld notes\n',
  };
  const pr = { number: 42, merged_at: '2026-01-01', merge_commit_sha: sha,
    base: { ref: 'main' }, labels: [{ name: 'autorelease: pending' }] };
  const context = { repo: { owner: 'owner', repo: 'repo' }, payload: { workflow_run: {
    conclusion: 'success', event: 'push', head_branch: 'main', head_sha: sha,
    head_repository: { full_name: 'owner/repo' },
  } } };
  const missing = async () => { throw Object.assign(new Error('missing'), { status: 404 }); };
  const record = name => async args => { calls.push({ operation: name, ...args }); return { data: {} }; };
  const github = { paginate: async () => [pr], rest: {
    repos: { listPullRequestsAssociatedWithCommit() {},
      getContent: async ({ path, ref }) => {
        assert.equal(ref, sha, 'Release inputs must come from the tested commit');
        return { data: { type: 'file', encoding: 'base64', content: Buffer.from(files[path]).toString('base64') } };
      }, getReleaseByTag: missing, createRelease: record('release') },
    git: { getRef: missing, createRef: record('tag') },
    issues: { addLabels: record('add'), removeLabel: record('remove') },
  } };
  return { sha, calls, files, pr, context, github, core: { info() {} } };
}

test('publishes the exact tested commit and only its reviewed notes', async () => {
  const f = fixture();
  await publish(f);
  assert.deepEqual(f.calls.map(c => c.operation), ['tag', 'release', 'add', 'remove']);
  assert.equal(f.calls[0].sha, f.sha);
  assert.equal(f.calls[1].target_commitish, f.sha);
  assert.equal(f.calls[1].tag_name, 'v0.1.0');
  assert.equal(f.calls[1].body, '### Features\n\n- Native inference');
});

test('accepts linked changelog headers from Release Please', async () => {
  const f = fixture();
  f.files['CHANGELOG.md'] = '## [0.1.0](https://example.com/compare) (2026-01-01)\n\nNotes\n';
  await publish(f);
  assert.equal(f.calls[1].body, 'Notes');
});

test('rejects failed CI, PR runs, other branches, and forks before mutations', async () => {
  for (const patch of [{ conclusion: 'failure' }, { event: 'pull_request' },
    { head_branch: 'other' }, { head_repository: { full_name: 'fork/repo' } }]) {
    const f = fixture();
    Object.assign(f.context.payload.workflow_run, patch);
    await assert.rejects(publish(f), /Expected successful/);
    assert.deepEqual(f.calls, []);
  }
});

test('ordinary commits and different release merge SHAs do not release', async () => {
  for (const change of [f => { f.pr.labels = []; }, f => { f.pr.merge_commit_sha = 'b'.repeat(40); },
    f => { f.pr.merged_at = null; }, f => { f.pr.base.ref = 'other'; }]) {
    const f = fixture(); change(f); await publish(f); assert.deepEqual(f.calls, []);
  }
});

test('invalid version or absent/empty notes fail before tag creation', async () => {
  for (const change of [f => { f.files['.release-please-manifest.json'] = '{}'; },
    f => { f.files['.release-please-manifest.json'] = '{".":"01.2.3"}'; },
    f => { f.files['CHANGELOG.md'] = '## 0.2.0\n\nWrong version'; },
    f => { f.files['CHANGELOG.md'] = '## 0.1.0\n\n'; },
    f => { f.files['CHANGELOG.md'] = '## 0.1.0 (2026-01-01)'; }]) {
    const f = fixture(); change(f); await assert.rejects(publish(f)); assert.deepEqual(f.calls, []);
  }
});

test('conflicting tags and API failures fail closed', async () => {
  const f = fixture();
  f.github.rest.git.getRef = async () => ({ data: { object: { type: 'commit', sha: 'b'.repeat(40) } } });
  await assert.rejects(publish(f), /conflicts/);
  assert.deepEqual(f.calls, []);
  f.github.rest.git.getRef = async () => { throw Object.assign(new Error('denied'), { status: 403 }); };
  await assert.rejects(publish(f), /denied/);
  assert.deepEqual(f.calls, []);
});

test('retry reuses tag and release, then repairs lifecycle labels', async () => {
  const f = fixture();
  f.github.rest.git.getRef = async () => ({ data: { object: { type: 'commit', sha: f.sha } } });
  f.github.rest.repos.getReleaseByTag = async () => ({ data: { draft: false, body: '### Features\n\n- Native inference' } });
  await publish(f);
  assert.deepEqual(f.calls.map(c => c.operation), ['add', 'remove']);
  f.calls.length = 0;
  f.github.rest.repos.getReleaseByTag = async () => ({ data: { draft: true, body: 'Changed notes' } });
  await assert.rejects(publish(f), /differs/);
  assert.deepEqual(f.calls, []);
});

test('release API failure retains pending label for retry', async () => {
  const f = fixture();
  f.github.rest.repos.createRelease = async () => { throw new Error('unavailable'); };
  await assert.rejects(publish(f), /unavailable/);
  assert.deepEqual(f.calls.map(c => c.operation), ['tag']);
});

test('executes actual title workflow script against valid and invalid titles', () => {
  const workflow = readFileSync('.github/workflows/pr-title.yml', 'utf8');
  const source = workflow.split('          script: |\n')[1].split('\n').map(line => line.slice(12)).join('\n');
  const validate = new Function('context', 'core', source);
  for (const [title, valid] of [
    ['feat: add models', true], ['fix(tokenizer): preserve whitespace', true],
    ['feat!: change API', true], ['chore(main): release 0.1.0', true],
    ['build(deps): bump package', true], ['ci: $(touch /tmp/never-executed)', true],
    ['add models', false], ['Feat: add models', false], ['feat: ', false],
    ['feat(): title', false], ['feat: title\nextra', false], ['feat: title ', false],
  ]) {
    let failed = false;
    validate({ payload: { pull_request: { title } } }, { setFailed() { failed = true; } });
    assert.equal(!failed, valid, title);
  }
});

test('rejects each existing release mismatch independently', async () => {
  for (const mismatch of [{ draft: true }, { prerelease: true }, { body: 'Changed notes' }]) {
    const f = fixture();
    f.github.rest.git.getRef = async () => ({ data: { object: { type: 'commit', sha: f.sha } } });
    f.github.rest.repos.getReleaseByTag = async () => ({ data: {
      draft: false, prerelease: false, body: '### Features\n\n- Native inference', ...mismatch } });
    await assert.rejects(publish(f), /differs/);
    assert.deepEqual(f.calls, []);
  }
});

test('ambiguous PRs and invalid file responses fail before mutations', async () => {
  const f = fixture();
  f.github.paginate = async () => [f.pr, { ...f.pr, number: 43 }];
  await assert.rejects(publish(f), /Ambiguous/);
  assert.deepEqual(f.calls, []);
  for (const data of [{ type: 'dir' }, { type: 'file', encoding: 'none' }]) {
    const g = fixture();
    g.github.rest.repos.getContent = async () => ({ data });
    await assert.rejects(publish(g), /Cannot read/);
    assert.deepEqual(g.calls, []);
  }
});

test('label failure after publishing can be retried without another release', async () => {
  const f = fixture();
  const remove = f.github.rest.issues.removeLabel;
  f.github.rest.issues.removeLabel = async () => { throw new Error('label unavailable'); };
  await assert.rejects(publish(f), /label unavailable/);
  f.github.rest.git.getRef = async () => ({ data: { object: { type: 'commit', sha: f.sha } } });
  f.github.rest.repos.getReleaseByTag = async () => ({ data: { draft: false, prerelease: false, body: '### Features\n\n- Native inference' } });
  f.github.rest.issues.removeLabel = remove;
  f.calls.length = 0;
  await publish(f);
  assert.deepEqual(f.calls.map(c => c.operation), ['add', 'remove']);
});
