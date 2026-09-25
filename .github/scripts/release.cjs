// Only merging a prepared release PR starts publication; registry uploads remain manual.
module.exports = async ({ github, context, core,
  wait = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)),
  attempts = 120 }) => {
  const repo = context.repo;
  const defaultBranch = context.payload.repository?.default_branch;
  if (typeof defaultBranch !== 'string' || defaultBranch.trim() === '') {
    throw new Error('Expected repository default branch metadata');
  }
  const event = context.payload.pull_request;
  if (context.eventName !== 'pull_request' || context.payload.action !== 'closed' ||
      !event?.merged || event.base.ref !== defaultBranch ||
      !event.labels.some(label => label.name === 'autorelease: pending')) {
    throw new Error('Publish requires a merged release PR');
  }
  const issue_number = event.number;
  const { data: pr } = await github.rest.pulls.get({ ...repo, pull_number: issue_number });
  const pending = pr.labels.some(label => label.name === 'autorelease: pending');
  const tagged = pr.labels.some(label => label.name === 'autorelease: tagged');
  const sha = pr.merge_commit_sha;
  if (!pr.merged_at || pr.base.ref !== defaultBranch ||
      pr.base.repo.full_name !== `${repo.owner}/${repo.repo}` ||
      typeof sha !== 'string' || !/^[0-9a-f]{40}$/.test(sha) || sha !== event.merge_commit_sha || (!pending && !tagged)) {
    throw new Error('Select a merged release PR targeting this repository default branch');
  }
  // Read CI for the release merge commit, even if the default branch has advanced since then.
  // Do not filter to successful runs: a newer failed/in-progress run must block publication.
  let verified = false;
  for (let attempt = 0; attempt < attempts; attempt++) {
    const { data } = await github.rest.actions.listWorkflowRuns({ ...repo, workflow_id: 'ci.yml',
      head_sha: sha, event: 'push', branch: defaultBranch, per_page: 100 });
    const run = data.workflow_runs[0];
    if (run) {
      if (run.head_sha !== sha || run.event !== 'push' || run.head_branch !== defaultBranch ||
          run.head_repository?.full_name !== `${repo.owner}/${repo.repo}`) {
        throw new Error('Expected successful default-branch push CI for the release merge commit');
      }
      if (run.status === 'completed') {
        if (run.conclusion !== 'success') {
          throw new Error('Release merge CI was not successful; fix or rerun CI, then rerun this release job');
        }
        core.info(`Publishing PR #${issue_number} at ${sha}, verified by CI run ${run.id}`);
        verified = true;
        break;
      }
    }
    core.info('Waiting for native-fidelity push CI on the release merge commit');
    if (attempt + 1 < attempts) await wait(20000);
  }
  if (!verified) throw new Error('Timed out waiting for successful release merge CI; rerun this release job once CI passes');
  const read = async path => {
    const { data } = await github.rest.repos.getContent({ ...repo, path, ref: sha });
    if (data.type !== 'file' || data.encoding !== 'base64') throw new Error(`Cannot read ${path}`);
    return Buffer.from(data.content, 'base64').toString('utf8');
  };
  const manifest = JSON.parse(await read('.release-please-manifest.json'));
  const version = manifest['.'];
  if (typeof version !== 'string' || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error('Expected a stable three-part release version');
  }
  const changelog = await read('CHANGELOG.md');
  const sections = changelog.split(/^## /m).slice(1);
  const entry = sections.find(section => section.startsWith(`${version} `) ||
    section.startsWith(`${version}\n`) || section.startsWith(`[${version}](`));
  if (!entry || !entry.includes('\n')) throw new Error(`Missing changelog entry for ${version}`);
  const body = entry.slice(entry.indexOf('\n') + 1).trim();
  if (!body) throw new Error('Empty release notes');
  const tag = `v${version}`;
  // Establish the tag explicitly, then verify it on retries. Never move an existing tag.
  const getOrMissing = async operation => {
    try { return (await operation()).data; }
    catch (error) { if (error.status === 404) return undefined; throw error; }
  };
  const ref = await getOrMissing(() => github.rest.git.getRef({ ...repo, ref: `tags/${tag}` }));
  if (ref) {
    if (ref.object.type !== 'commit' || ref.object.sha !== sha) throw new Error('Release tag conflicts with verified commit');
  } else {
    if (!pending) throw new Error('Previously published release tag is missing');
    await github.rest.git.createRef({ ...repo, ref: `refs/tags/${tag}`, sha });
  }
  const existing = await getOrMissing(() => github.rest.repos.getReleaseByTag({ ...repo, tag }));
  if (existing) {
    if (existing.draft || existing.prerelease || existing.body !== body) throw new Error('Existing release differs from reviewed release notes');
  } else {
    if (!pending) throw new Error('Previously published GitHub Release is missing');
    await github.rest.repos.createRelease({ ...repo, tag_name: tag, target_commitish: sha,
      name: tag, body, draft: false, prerelease: false });
  }
  if (pending) {
    await github.rest.issues.addLabels({ ...repo, issue_number, labels: ['autorelease: tagged'] });
    await github.rest.issues.removeLabel({ ...repo, issue_number, name: 'autorelease: pending' });
  }
};
