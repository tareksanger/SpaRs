// Publish only the merged release PR whose exact commit passed native-fidelity.
module.exports = async ({ github, context, core }) => {
  const run = context.payload.workflow_run;
  const repo = context.repo;
  if (run.conclusion !== 'success' || run.event !== 'push' ||
      run.head_branch !== 'main' || run.head_repository.full_name !== `${repo.owner}/${repo.repo}`) {
    throw new Error('Expected successful main-branch push CI from this repository');
  }
  const sha = run.head_sha;
  const prs = await github.paginate(github.rest.repos.listPullRequestsAssociatedWithCommit, {
    ...repo, commit_sha: sha, per_page: 100,
  });
  const candidates = prs.filter(pr => pr.merged_at && pr.merge_commit_sha === sha &&
    pr.base.ref === 'main' && pr.labels.some(label => label.name === 'autorelease: pending'));
  if (candidates.length === 0) {
    core.info('No pending release PR at the verified commit');
    return;
  }
  if (candidates.length !== 1) throw new Error('Ambiguous release PR');
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
    await github.rest.git.createRef({ ...repo, ref: `refs/tags/${tag}`, sha });
  }
  const existing = await getOrMissing(() => github.rest.repos.getReleaseByTag({ ...repo, tag }));
  if (existing) {
    if (existing.draft || existing.prerelease || existing.body !== body) throw new Error('Existing release differs from reviewed release notes');
  } else {
    await github.rest.repos.createRelease({ ...repo, tag_name: tag, target_commitish: sha,
      name: tag, body, draft: false, prerelease: false });
  }
  const issue_number = candidates[0].number;
  await github.rest.issues.addLabels({ ...repo, issue_number, labels: ['autorelease: tagged'] });
  await github.rest.issues.removeLabel({ ...repo, issue_number, name: 'autorelease: pending' });
};
