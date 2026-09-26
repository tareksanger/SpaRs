// Resolve a released, CI-verified commit before any native build or registry access.
module.exports = async ({github, context, tag}) => {
  const branch = context.payload.repository?.default_branch;
  if (!branch || context.eventName !== 'workflow_dispatch' || context.ref !== `refs/heads/${branch}`) {
    throw new Error('Run npm releases manually from the default branch');
  }
  if (typeof tag !== 'string' || !/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(tag)) throw new Error('Expected stable vX.Y.Z tag');
  const repo = context.repo;
  const {data:release} = await github.rest.repos.getReleaseByTag({...repo,tag});
  if (release.draft || release.prerelease || release.tag_name !== tag) throw new Error('Expected published stable GitHub release');
  const {data:ref} = await github.rest.git.getRef({...repo,ref:`tags/${tag}`});
  if (ref.object.type !== 'commit' || !/^[0-9a-f]{40}$/.test(ref.object.sha)) throw new Error('Expected release commit tag');
  const sha = ref.object.sha;
  if (release.target_commitish !== sha) throw new Error('Release tag differs from its recorded release commit');
  const {data} = await github.rest.actions.listWorkflowRuns({...repo,workflow_id:'ci.yml',head_sha:sha,event:'push',branch,per_page:100});
  const run = data.workflow_runs[0];
  if (!run || run.status !== 'completed' || run.conclusion !== 'success' || run.head_sha !== sha || run.event !== 'push' || run.head_branch !== branch || run.head_repository?.full_name !== `${repo.owner}/${repo.repo}`) throw new Error('Release commit needs successful default-branch push CI');
  const {data:file} = await github.rest.repos.getContent({...repo,path:'bindings/node/package.json',ref:sha});
  if (file.type !== 'file' || file.encoding !== 'base64') throw new Error('Missing Node manifest');
  const manifest = JSON.parse(Buffer.from(file.content,'base64').toString('utf8'));
  if (manifest.version !== tag.slice(1)) throw new Error('Node version does not match release tag');
  return {sha,version:manifest.version};
};
