const {test} = require('node:test');
const assert = require('node:assert/strict');
const validate = require('./npm-release.cjs');
function fixture() {
  const sha = 'a'.repeat(40);
  const state = {
    context:{sha,eventName:'workflow_dispatch',ref:'refs/heads/trunk',repo:{owner:'owner',repo:'repo'},payload:{repository:{default_branch:'trunk'}}},
    tag:'v0.2.0',
    release:{draft:false,prerelease:false,tag_name:'v0.2.0',target_commitish:sha},
    ref:{object:{type:'commit',sha}},
    run:{status:'completed',conclusion:'success',head_sha:sha,event:'push',head_branch:'trunk',head_repository:{full_name:'owner/repo'}},
    version:'0.2.0',
    comparison:'identical',
  };
  const github = {rest:{
    repos:{compareCommits:async args=>{assert.equal(args.base,state.ref.object.sha);assert.equal(args.head,state.context.sha);return {data:{status:state.comparison}};},getReleaseByTag:async()=>({data:state.release}),getContent:async args => {
      assert.equal(args.ref,state.context.sha);
      return {data:{type:'file',encoding:'base64',content:Buffer.from(JSON.stringify({version:state.version})).toString('base64')}};
    }},
    git:{getRef:async()=>({data:state.ref})},
    actions:{listWorkflowRuns:async args => {
      assert.equal(args.head_sha,state.context.sha); assert.equal(args.branch,'trunk'); assert.equal(args.event,'push');
      return {data:{workflow_runs:[state.run]}};
    }},
  }};
  return {state,run:()=>validate({github,context:state.context,tag:state.tag})};
}
test('npm release resolves exact successful commit on renamed default branch', async()=>{
  const f=fixture(); assert.deepEqual(await f.run(),{sha:'a'.repeat(40),version:'0.2.0'});
});
for (const [name,mutate] of [
  ['non-default branch',s=>{s.context.ref='refs/heads/main';}],
  ['automatic event',s=>{s.context.eventName='push';}],
  ['invalid tag',s=>{s.tag='main';}],
  ['draft release',s=>{s.release.draft=true;}],
  ['prerelease',s=>{s.release.prerelease=true;}],
  ['annotated tag',s=>{s.ref.object.type='tag';}],
  ['moved tag',s=>{s.ref.object.sha='b'.repeat(40);}],
  ['failed CI',s=>{s.run.conclusion='failure';}],
  ['pending CI',s=>{s.run.status='in_progress';}],
  ['wrong CI commit',s=>{s.run.head_sha='b'.repeat(40);}],
  ['fork CI',s=>{s.run.head_repository.full_name='other/repo';}],
  ['PR CI',s=>{s.run.event='pull_request';}],
  ['missing workflow commit',s=>{s.context.sha=undefined;}],
  ['diverged source',s=>{s.comparison='diverged';}],
  ['older source',s=>{s.comparison='behind';}],
  ['version mismatch',s=>{s.version='0.1.0';}],
]) test(`npm release rejects ${name}`,async()=>{const f=fixture();mutate(f.state);await assert.rejects(f.run());});

test('workflow gates publishing on all platform smoke jobs and uses immutable checkout/artifacts', () => {
  const {readFileSync} = require('node:fs');
  const {parse} = require('../../tools/node_modules/yaml');
  const workflow = parse(readFileSync('.github/workflows/npm-release.yml','utf8'));
  assert.deepEqual(Object.keys(workflow.on).sort(),['pull_request','workflow_dispatch']);
  assert.equal(workflow.on.workflow_dispatch.inputs.publish.default,false);
  assert.equal(workflow.jobs.publish.if,"github.event_name == 'workflow_dispatch' && inputs.publish");
  assert.equal(workflow.jobs.publish.environment,'npm');
  assert.deepEqual(workflow.jobs.publish.needs,['validate','smoke']);
  assert.deepEqual(workflow.jobs.smoke.needs,['validate','assemble']);
  assert.deepEqual(workflow.jobs.assemble.needs,['validate','build']);
  assert.deepEqual(workflow.jobs.smoke.strategy.matrix.os,workflow.jobs.build.strategy.matrix.include.map(row=>row.os));
  for (const job of ['build','assemble','smoke','publish']) {
    const checkout=workflow.jobs[job].steps.find(step=>step.uses?.startsWith('actions/checkout@'));
    assert.equal(checkout.with.ref,'${{ needs.validate.outputs.sha }}');
    assert.equal(checkout.with['persist-credentials'],false);
  }
  for(const [name,job] of Object.entries(workflow.jobs)) {
    if(name!=='publish') assert.notEqual(job.permissions?.['id-token'],'write');
    for(const step of job.steps) if(step.uses) assert.match(step.uses,/@[0-9a-f]{40}$/);
  }
});


test('old release tag uses the CI-verified workflow commit at the same version',async()=>{
  const f=fixture();
  f.state.context.sha='b'.repeat(40);
  f.state.run.head_sha=f.state.context.sha;
  f.state.comparison='ahead';
  assert.deepEqual(await f.run(),{sha:'b'.repeat(40),version:'0.2.0'});
});

test('old tag CI cannot substitute for successful workflow-commit CI',async()=>{
  const f=fixture();f.state.context.sha='b'.repeat(40);f.state.comparison='ahead';
  await assert.rejects(f.run(),/Workflow commit needs successful/);
});

test('Linux ARM64 is built, tested, and packaged on a native runner', async()=>{
  const {readFileSync} = require('node:fs');
  const {parse} = require('../../tools/node_modules/yaml');
  const {targets} = await import('../../bindings/node/scripts/release.mts');
  const release=parse(readFileSync('.github/workflows/npm-release.yml','utf8'));
  const ci=parse(readFileSync('.github/workflows/ci.yml','utf8'));
  const manifest=JSON.parse(readFileSync('bindings/node/package.json','utf8'));
  assert.deepEqual(targets.find(target=>target.suffix==='linux-arm64-gnu'),
    {suffix:'linux-arm64-gnu',triple:'aarch64-unknown-linux-gnu',os:'linux',cpu:'arm64',libc:'glibc'});
  assert.deepEqual(new Set(manifest.napi.targets),new Set(targets.map(target=>target.triple)));
  assert.ok(release.jobs.build.strategy.matrix.include.some(row=>row.os==='ubuntu-24.04-arm' && row.target==='linux-arm64-gnu'));
  assert.ok(release.jobs.smoke.strategy.matrix.os.includes('ubuntu-24.04-arm'));
  assert.ok(ci.jobs['reference-and-rust'].strategy.matrix.os.includes('ubuntu-24.04-arm'));
  const acceptance=ci.jobs['reference-and-rust'].steps.find(step=>step.name==='Run Linux acceptance');
  assert.equal(acceptance.if,"runner.os == 'Linux'");
  assert.match(acceptance.run,/tools\/verify\.py --model-exports-only/);
});

test('pull requests build the exact merge commit without checking a release or permitting publication',async()=>{
  const f=fixture();f.state.context.eventName='pull_request';f.state.context.ref='refs/pull/42/merge';
  f.state.tag=undefined;f.state.release=null;f.state.run=null;
  assert.deepEqual(await f.run(),{sha:'a'.repeat(40),version:'0.2.0'});
  f.state.context.ref='refs/heads/trunk';await assert.rejects(f.run(),/merge ref/);
});

test('pull request validation rejects invalid package versions and missing manifests',async()=>{
  const f=fixture();f.state.context.eventName='pull_request';f.state.context.ref='refs/pull/42/merge';
  for(const version of ['bad','0.2.0-beta.1',2,null]) {
    f.state.version=version;
    await assert.rejects(f.run(),/Invalid Node package version/);
  }
  await assert.rejects(validate({github:{rest:{repos:{getContent:async()=>({data:[]})}}},context:f.state.context}),/Missing Node manifest/);
});
