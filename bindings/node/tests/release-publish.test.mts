import assert from 'node:assert/strict';
import {test} from 'node:test';
import {publishPackages, type ReleasePackage, type Registry} from '../scripts/release-publish.mts';
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
  }),/not yet confirmed/);
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
      execFileSync('tar',['-czf',join(root,'tarballs',file),'-C',join(root,'source'),'package']);
    }
    // Deliberately put main first by filename; publish order must follow package identity.
    tarball('a.tgz','@spars/node');tarball('b.tgz','@spars/node-darwin-arm64');tarball('c.tgz','@spars/node-linux-x64-gnu');
    const ordered=releasePackages(join(root,'tarballs'));
    assert.deepEqual(ordered.map(pkg=>pkg.name),['@spars/node-linux-x64-gnu','@spars/node-darwin-arm64','@spars/node']);
    for(const pkg of ordered) assert.equal(pkg.integrity,'sha512-'+createHash('sha512').update(readFileSync(pkg.path)).digest('base64'));
    tarball('b.tgz','@spars/node-darwin-arm64','0.1.0');assert.throws(()=>releasePackages(join(root,'tarballs')),/mismatched/);
    tarball('b.tgz','@spars/node-linux-x64-gnu');assert.throws(()=>releasePackages(join(root,'tarballs')),/mismatched/);
    tarball('b.tgz',false);assert.throws(()=>releasePackages(join(root,'tarballs')),/Invalid tarball/);
    rmSync(join(root,'tarballs/b.tgz'));assert.throws(()=>releasePackages(join(root,'tarballs')),/exactly three/);
  } finally {rmSync(root,{recursive:true,force:true});}
});
