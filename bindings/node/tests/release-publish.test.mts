import assert from 'node:assert/strict';
import {test} from 'node:test';
import {targets} from '../scripts/release.mts';
import {publishPackages, publishTarball, PendingPublicationError, type ReleasePackage, type Registry} from '../scripts/release-publish.mts';
const packages: ReleasePackage[] = ['linux','macos','main'].map(name=>({name,version:'0.2.0',path:name+'.tgz',integrity:'sha512-'+name}));
test('partial publication retries identical bytes and publishes main last',async()=>{
  const stored=new Map<string,string>();const uploaded:string[]=[];
  let fail=true;
  const registry:Registry={
    async integrity(pkg){return stored.get(pkg.name);},
    async publish(pkg){if(pkg.name==='macos' && fail) throw new Error('network failure');uploaded.push(pkg.name);stored.set(pkg.name,pkg.integrity);},
  };
  await assert.rejects(publishPackages(packages,registry),/network failure/);
  assert.deepEqual(uploaded,['linux']);
  fail=false;await publishPackages(packages,registry);
  assert.deepEqual(uploaded,['linux','macos','main']);
  await publishPackages(packages,registry);assert.equal(uploaded.length,3);
});
test('different registry bytes or lookup failure prevent every upload',async()=>{
  for(const lookup of ['different','error']) {
    let uploads=0;
    await assert.rejects(publishPackages(packages,{
      async integrity(pkg){if(lookup==='error') throw new Error('lookup failed');return pkg.name==='main'?'sha512-other':undefined;},
      async publish(){uploads++;},
    }));
    assert.equal(uploads,0);
  }
});
test('unconfirmed platform upload blocks main until retry',async()=>{
  const uploaded:string[]=[];
  await assert.rejects(publishPackages(packages,{
    async integrity(){return undefined;},async publish(pkg){uploaded.push(pkg.name);},
  }, {wait:async () => {}}),/not yet confirmed/);
  assert.deepEqual(uploaded,['linux']);
});

test('real tarball manifests determine ordering, SHA512, and complete versioned inventory', async()=>{
  const {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} = await import('node:fs');
  const {join} = await import('node:path');
  const {tmpdir} = await import('node:os');
  const {execFileSync} = await import('node:child_process');
  const {createHash} = await import('node:crypto');
  const {releasePackages} = await import('../scripts/release-publish.mts');
  const root=mkdtempSync(join(tmpdir(),'spars-publish-inventory-'));
  try {
    mkdirSync(join(root,'source/package'),{recursive:true});mkdirSync(join(root,'tarballs'));
    function tarball(file:string,name:unknown,version:unknown='0.2.0'):void {
      writeFileSync(join(root,'source/package/package.json'),JSON.stringify({name,version}));
      execFileSync('tar',['-czf','../tarballs/'+file,'package'],{cwd:join(root,'source')});
    }
    // Deliberately put main first by filename; publish order must follow package identity.
    tarball('a.tgz','@spars/node');
    for(const [index,target] of targets.entries()) tarball(`platform-${index}.tgz`,`@spars/node-${target.suffix}`);
    const ordered=releasePackages(join(root,'tarballs'));
    assert.deepEqual(ordered.map(pkg=>pkg.name),[...targets.map(target=>`@spars/node-${target.suffix}`),'@spars/node']);
    for(const pkg of ordered) assert.equal(pkg.integrity,'sha512-'+createHash('sha512').update(readFileSync(pkg.path)).digest('base64'));
    tarball('platform-0.tgz',`@spars/node-${targets[0].suffix}`,'0.1.0');assert.throws(()=>releasePackages(join(root,'tarballs')),/mismatched/);
    tarball('platform-0.tgz','@spars/node-darwin-arm64');assert.throws(()=>releasePackages(join(root,'tarballs')),/mismatched/);
    tarball('platform-0.tgz',false);assert.throws(()=>releasePackages(join(root,'tarballs')),/Invalid tarball/);
    rmSync(join(root,'tarballs/platform-0.tgz'));assert.throws(()=>releasePackages(join(root,'tarballs')),new RegExp(`exactly ${targets.length+1} release tarballs`));
  } finally {rmSync(root,{recursive:true,force:true});}
});

test('delayed registry visibility completes all eight targets and main without duplicate uploads', async () => {
  const release = [...targets.map(target => target.suffix), 'main'].map(name => ({name, version:'0.3.0', path:name+'.tgz', integrity:'sha512-'+name}));
  const uploaded: string[] = [];
  const remaining = new Map<string, number>();
  let waits = 0;
  await publishPackages(release, {
    async integrity(pkg) {
      const count = remaining.get(pkg.name);
      if (count === undefined) return undefined;
      if (count > 0) { remaining.set(pkg.name, count - 1); return undefined; }
      return pkg.integrity;
    },
    async publish(pkg) {
      assert.ok(!uploaded.includes(pkg.name));
      for (const previous of uploaded) assert.equal(remaining.get(previous), 0);
      uploaded.push(pkg.name);
      remaining.set(pkg.name, 2);
    },
  }, {wait: async () => { waits++; }});
  assert.deepEqual(uploaded, release.map(pkg => pkg.name));
  assert.equal(waits, 18);
});

test('staged conflict waits for identical bytes and continues without republishing', async () => {
  const uploaded: string[] = [];
  const stored = new Map<string,string>();
  let waits = 0;
  await publishPackages(packages, {
    async integrity(pkg) { return stored.get(pkg.name); },
    async publish(pkg) {
      uploaded.push(pkg.name);
      if (pkg.name === 'linux') throw new PendingPublicationError('already staged');
      stored.set(pkg.name, pkg.integrity);
    },
  }, {wait:async () => { waits++; stored.set(packages[0]!.name, packages[0]!.integrity); }});
  assert.deepEqual(uploaded, ['linux','macos','main']);
  assert.equal(waits, 1);
});

test('confirmation timeout is bounded and prevents later uploads', async () => {
  let waits = 0;
  let uploads = 0;
  await assert.rejects(publishPackages(packages, {
    async integrity() { return undefined; },
    async publish() { uploads++; },
  }, {wait:async milliseconds => { assert.equal(milliseconds,5000); waits++; }}), /after 60 waits/);
  assert.equal(waits,60);
  assert.equal(uploads,1);
});

test('post-upload mismatched bytes and lookup errors stop immediately', async () => {
  for (const failure of ['mismatch','lookup']) {
    let uploaded = false;
    let uploads = 0;
    await assert.rejects(publishPackages(packages, {
      async integrity() {
        if (!uploaded) return undefined;
        if (failure === 'lookup') throw new Error('lookup failed');
        return 'sha512-other';
      },
      async publish() { uploaded = true; uploads++; },
    }, {wait:async () => { assert.fail('must not wait on errors'); }}), failure === 'lookup' ? /lookup failed/ : /integrity mismatch/);
    assert.equal(uploads,1);
  }
});

test('npm subprocess distinguishes accepted upload, staged conflict, and ordinary failure', {skip:process.platform === 'win32'}, async () => {
  const {mkdtempSync,writeFileSync,rmSync} = await import('node:fs');
  const {join} = await import('node:path');
  const {tmpdir} = await import('node:os');
  const root = mkdtempSync(join(tmpdir(),'spars-npm-command-'));
  const previous = process.env.PATH;
  try {
    process.env.PATH = root;
    writeFileSync(join(root,'npm'), '#!/bin/sh\n[ "$#" -eq 5 ] && [ "$1" = publish ] && [ "$2" = linux.tgz ] && [ "$3" = --access ] && [ "$4" = public ] && [ "$5" = --ignore-scripts ] || exit 2\nexit 0\n', {mode:0o755});
    await publishTarball(packages[0]!);
    writeFileSync(join(root,'npm'), '#!/bin/sh\necho \'npm error 409 Cannot publish over previously staged version "0.3.0".\' >&2\nexit 1\n');
    await assert.rejects(publishTarball(packages[0]!), PendingPublicationError);
    writeFileSync(join(root,'npm'), '#!/bin/sh\necho "npm error E401 Unauthorized" >&2\nexit 1\n');
    await assert.rejects(publishTarball(packages[0]!), error => error instanceof Error && !(error instanceof PendingPublicationError) && /npm publish failed/.test(error.message));
  } finally {
    if (previous === undefined) delete process.env.PATH; else process.env.PATH = previous;
    rmSync(root,{recursive:true,force:true});
  }
});

test('staged upload can confirm on the final lookup and never bypasses mismatch or timeout', async () => {
  for (const outcome of ['final','mismatch','timeout']) {
    let uploads = 0;
    let waits = 0;
    const run = publishPackages([packages[0]!], {
      async integrity(pkg) {
        if (uploads === 0) return undefined;
        if (outcome === 'mismatch') return 'sha512-other';
        return outcome === 'final' && waits === 60 ? pkg.integrity : undefined;
      },
      async publish() { uploads++; throw new PendingPublicationError('already staged'); },
    }, {wait:async () => { waits++; }});
    if (outcome === 'final') await run;
    else await assert.rejects(run, outcome === 'mismatch' ? /integrity mismatch/ : /after 60 waits/);
    assert.equal(uploads,1);
    assert.equal(waits,outcome === 'mismatch' ? 0 : 60);
  }
});
