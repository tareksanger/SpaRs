// Build a CI-verified source commit without moving the existing release tag.
module.exports = async ({github, context, tag, source}) => {
  const repo = context.repo;
  if (typeof context.sha !== 'string' || !/^[0-9a-f]{40}$/.test(context.sha)) throw new Error('Expected exact workflow commit');
  const sha = source || context.sha;
  if (source && context.eventName === 'pull_request') throw new Error('PRs cannot select release source');
  if (typeof sha !== 'string' || !/^[0-9a-f]{40}$/.test(sha)) throw new Error('Expected exact workflow commit');
  const readVersion = async () => {
    const {data:file} = await github.rest.repos.getContent({...repo,path:'bindings/node/package.json',ref:sha});
    if (file.type !== 'file' || file.encoding !== 'base64') throw new Error('Missing Node manifest');
    const manifest = JSON.parse(Buffer.from(file.content,'base64').toString('utf8'));
    if (typeof manifest.version !== 'string' || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(manifest.version)) throw new Error('Invalid Node package version');
    return manifest.version;
  };
  // PRs test the merge commit without requiring a release or registry credentials.
  if (context.eventName === 'pull_request') {
    if (!/^refs\/pull\/[1-9]\d*\/merge$/.test(context.ref)) throw new Error('Expected pull request merge ref');
    return {sha,version:await readVersion()};
  }
  const branch = context.payload.repository?.default_branch;
  if (!branch || context.eventName !== 'workflow_dispatch' || context.ref !== `refs/heads/${branch}`) {
    throw new Error('Run npm releases manually from the default branch');
  }
  if (typeof tag !== 'string' || !/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(tag)) throw new Error('Expected stable vX.Y.Z tag');
  const {data:release} = await github.rest.repos.getReleaseByTag({...repo,tag});
  if (release.draft || release.prerelease || release.tag_name !== tag) throw new Error('Expected published stable GitHub release');
  const {data:ref} = await github.rest.git.getRef({...repo,ref:`tags/${tag}`});
  if (ref.object.type !== 'commit' || !/^[0-9a-f]{40}$/.test(ref.object.sha)) throw new Error('Expected release commit tag');
  const tagSha = ref.object.sha;
  if (release.target_commitish !== tagSha) throw new Error('Release tag differs from its recorded release commit');
  if (source) {
    const {data:ancestry} = await github.rest.repos.compareCommits({...repo,base:sha,head:context.sha});
    if (!['identical','ahead'].includes(ancestry.status)) throw new Error('Source must belong to the default branch history');
  }
  const {data:comparison} = await github.rest.repos.compareCommits({...repo,base:tagSha,head:sha});
  if (!['identical','ahead'].includes(comparison.status)) throw new Error('Workflow commit must contain the release tag');
  const {data} = await github.rest.actions.listWorkflowRuns({...repo,workflow_id:'ci.yml',head_sha:sha,event:'push',branch,per_page:100});
  const run = data.workflow_runs[0];
  if (!run || run.status !== 'completed' || run.conclusion !== 'success' || run.head_sha !== sha || run.event !== 'push' || run.head_branch !== branch || run.head_repository?.full_name !== `${repo.owner}/${repo.repo}`) throw new Error('Source commit needs successful default-branch push CI');
  const version = await readVersion();
  if (version !== tag.slice(1)) throw new Error('Node version does not match release tag');
  return {sha,version};
};
