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
    base: { ref: 'main', repo: { full_name: 'owner/repo' } }, labels: [{ name: 'autorelease: pending' }] };
  const context = { repo: { owner: 'owner', repo: 'repo' }, eventName: 'pull_request',
    payload: { action: 'closed', pull_request: { ...structuredClone(pr), merged: true } } };
  const run = { id: 123, conclusion: 'success', status: 'completed', event: 'push',
    head_branch: 'main', head_sha: sha, head_repository: { full_name: 'owner/repo' } };
  const missing = async () => { throw Object.assign(new Error('missing'), { status: 404 }); };
  const record = name => async args => { calls.push({ operation: name, ...args }); return { data: {} }; };
  const github = { rest: {
    pulls: { get: async args => { assert.equal(args.pull_number, 42); return { data: pr }; } },
    actions: { listWorkflowRuns: async args => {
      assert.equal(args.workflow_id, 'ci.yml'); assert.equal(args.head_sha, sha);
      assert.equal(args.event, 'push'); assert.equal(args.branch, 'main');
      assert.equal(args.status, undefined, 'Do not select an older successful run over a newer failure');
      return { data: { workflow_runs: [run] } };
    } },
    repos: { listPullRequestsAssociatedWithCommit() {},
      getContent: async ({ path, ref }) => {
        assert.equal(ref, sha, 'Release inputs must come from the tested commit');
        return { data: { type: 'file', encoding: 'base64', content: Buffer.from(files[path]).toString('base64') } };
      }, getReleaseByTag: missing, createRelease: record('release') },
    git: { getRef: missing, createRef: record('tag') },
    issues: { addLabels: record('add'), removeLabel: record('remove') },
  } };
  return { sha, calls, files, pr, run, context, github, attempts: 2, wait: async () => {}, core: { info() {} } };
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
    Object.assign(f.run, patch);
    await assert.rejects(publish(f), /successful/);
    assert.deepEqual(f.calls, []);
  }
});

test('only closing a merged prepared release PR authorizes publication', async () => {
  for (const change of [f => { f.context.eventName = 'push'; },
    f => { f.context.eventName = 'workflow_dispatch'; }, f => { f.context.payload.action = 'opened'; },
    f => { f.context.payload.pull_request.merged = false; },
    f => { f.context.payload.pull_request.labels = []; },
    f => { f.context.payload.pull_request.base.ref = 'other'; }]) {
    const f = fixture(); change(f);
    await assert.rejects(publish(f)); assert.deepEqual(f.calls, []);
  }
});

test('rejects ordinary or unmerged PRs and missing CI', async () => {
  for (const change of [f => { f.pr.labels = []; }, f => { f.pr.merge_commit_sha = null; },
    f => { f.pr.merged_at = null; }, f => { f.pr.base.ref = 'other'; },
    f => { f.pr.base.repo.full_name = 'fork/repo'; },
    f => { f.run.head_sha = 'b'.repeat(40); }, f => { f.run.status = 'in_progress'; },
    f => { f.github.rest.actions.listWorkflowRuns = async () => ({ data: { workflow_runs: [] } }); }]) {
    const f = fixture(); change(f);
    await assert.rejects(publish(f)); assert.deepEqual(f.calls, []);
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

test('invalid file responses fail before mutations', async () => {
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

test('workflow prepares only on command and publishes only on release PR merge', () => {
  const { parse } = require('../../tools/node_modules/yaml');
  const workflow = parse(readFileSync('.github/workflows/release.yml', 'utf8'));
  assert.deepEqual(Object.keys(workflow.on).sort(), ['pull_request', 'workflow_dispatch']);
  assert.deepEqual(workflow.on.pull_request, { types: ['closed'], branches: ['main'] });
  assert.match(workflow.jobs.prepare.if, /github.event_name == 'workflow_dispatch'/);
  assert.match(workflow.jobs.prepare.if, /github.ref == 'refs\/heads\/main'/);
  assert.match(workflow.jobs.publish.if, /github.event_name == 'pull_request'/);
  assert.match(workflow.jobs.publish.if, /github.event.pull_request.merged == true/);
  assert.match(workflow.jobs.publish.if, /contains\(github.event.pull_request.labels.\*.name, 'autorelease: pending'\)/);
  const prepare = workflow.jobs.prepare.steps.find(step => step.uses?.startsWith('googleapis/release-please-action@'));
  assert.equal(prepare.with['skip-github-release'], true);
  assert.equal(workflow.concurrency, undefined, 'Unrelated closed PRs must not occupy the release queue');
  for (const job of Object.values(workflow.jobs)) {
    assert.deepEqual(job.concurrency, { group: 'release-main', queue: 'max', 'cancel-in-progress': false });
  }
  assert.deepEqual(workflow.jobs.publish.permissions, {
    actions: 'read', contents: 'write', 'pull-requests': 'read', issues: 'write' });
  assert.ok(!workflow.jobs.publish.steps.some(step => step.uses?.startsWith('googleapis/release-please-action@')));
});

test('newer failed CI blocks publication even if an older run succeeded', async () => {
  const f = fixture();
  f.github.rest.actions.listWorkflowRuns = async () => ({ data: {
    workflow_runs: [{ ...f.run, conclusion: 'failure' }, f.run] } });
  await assert.rejects(publish(f), /successful/);
  assert.deepEqual(f.calls, []);
});

test('fully completed publication is repeatable without recreating a deleted release', async () => {
  const f = fixture();
  f.pr.labels = [{ name: 'autorelease: tagged' }];
  await assert.rejects(publish(f), /tag is missing/);
  assert.deepEqual(f.calls, []);
  f.github.rest.git.getRef = async () => ({ data: { object: { type: 'commit', sha: f.sha } } });
  await assert.rejects(publish(f), /GitHub Release is missing/);
  assert.deepEqual(f.calls, []);
  f.github.rest.repos.getReleaseByTag = async () => ({ data: {
    draft: false, prerelease: false, body: '### Features\n\n- Native inference' } });
  await publish(f);
  assert.deepEqual(f.calls, []);
});

test('waits through missing and running CI, then publishes after success', async () => {
  const f = fixture();
  let reads = 0;
  let waits = 0;
  f.attempts = 3;
  f.wait = async delay => { assert.equal(delay, 20000); waits++; assert.deepEqual(f.calls, []); };
  f.github.rest.actions.listWorkflowRuns = async () => ({ data: { workflow_runs:
    ++reads === 1 ? [] : [{ ...f.run, status: reads === 2 ? 'in_progress' : 'completed' }] } });
  await publish(f);
  assert.equal(waits, 2);
  assert.equal(f.calls[1].operation, 'release');
});
