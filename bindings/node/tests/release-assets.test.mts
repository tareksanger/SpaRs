import {test} from 'node:test';
import assert from 'node:assert/strict';
import {validateManifest} from '../scripts/release-assets.mts';
test('manifest validates each boundary independently',()=>{
  const valid={schema:1,tag:'v0.2.0',sha:'a'.repeat(40),files:[{name:'package.tgz',integrity:'sha512-'+'A'.repeat(86)+'=='}]};
  assert.deepEqual(validateManifest(valid,'v0.2.0'),valid);
  for (const value of [null, {}, {...valid,schema:2}, {...valid,sha:'main'}, {...valid,tag:'v0.3.0'},
    {...valid,files:[{...valid.files[0],name:'../evil.tgz'}]},
    {...valid,files:[{...valid.files[0],name:'--flag.tgz'}]},
    {...valid,files:[{...valid.files[0],integrity:'bad'}]}]) assert.throws(()=>validateManifest(value,'v0.2.0'));
});

import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, readdirSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {basename, join} from 'node:path';
import {execFileSync} from 'node:child_process';
import {targets} from '../scripts/release.mts';
import {seal, verify, upload, download, runAssets, manifestName, type Gh} from '../scripts/release-assets.mts';
function fixture(): {root:string; directory:string; stored:Map<string,Buffer>; calls:string[][]; invoke:Gh} {
  const root=mkdtempSync(join(tmpdir(),'spars-assets-'));
  const directory=join(root,'tarballs');
  mkdirSync(directory);mkdirSync(join(root,'source/package'),{recursive:true});
  for(const suffix of ['',...targets.map(target=>'-'+target.suffix)]) {
    writeFileSync(join(root,'source/package/package.json'),JSON.stringify({name:'@spars/node'+suffix,version:'0.2.0'}));
    execFileSync('tar',['-czf',join(directory,'spars-node'+suffix+'.tgz'),'package'],{cwd:join(root,'source')});
  }
  seal(directory,'v0.2.0','a'.repeat(40));
  const stored=new Map<string,Buffer>();const calls:string[][]=[];
  const invoke:Gh=args=>{
    calls.push(args);
    if(args[0]==='api') return JSON.stringify({tag_name:'v0.2.0',draft:false,prerelease:false,immutable:false,assets:[...stored.keys()].map(name=>({name}))});
    if(args[1]==='upload') {const file=args[3]!;stored.set(basename(file),readFileSync(file));return '';}
    if(args[1]==='download') {writeFileSync(join(args[6]!,args[4]!),stored.get(args[4]!)!);return '';}
    throw new Error('Unexpected gh call');
  };
  return {root,directory,stored,calls,invoke};
}
test('retain complete set, retry without writes, and recover into missing output directories',()=>{
  const f=fixture();try {
    upload(f.directory,'v0.2.0',f.invoke);
    assert.equal(f.stored.size,targets.length+2);
    assert.equal([...f.stored.keys()].at(-1),manifestName);
    f.calls.length=0;upload(f.directory,'v0.2.0',f.invoke);
    assert.equal(f.calls.filter(call=>call[1]==='upload').length,0);
    const output=join(f.root,'new/nested/recovery');download(output,'v0.2.0',f.invoke);
    assert.deepEqual(verify(output,'v0.2.0'),verify(f.directory,'v0.2.0'));
    assert.throws(()=>download(output,'v0.2.0',f.invoke),/empty/);
  } finally {rmSync(f.root,{recursive:true,force:true});}
});
test('partial upload resumes without overwrites; a later conflicting asset prevents every write',()=>{
  const f=fixture();try {
    let writes=0;
    assert.throws(()=>upload(f.directory,'v0.2.0',args=>{
      if(args[1]==='upload' && ++writes===3) throw new Error('network failure');
      return f.invoke(args);
    }),/network failure/);
    assert.equal(f.stored.size,2);assert.equal(f.stored.has(manifestName),false);
    upload(f.directory,'v0.2.0',f.invoke);assert.equal(f.stored.size,targets.length+2);
    const keys=[...f.stored.keys()];f.stored.delete(keys[0]!);f.stored.set(keys[1]!,Buffer.from('different'));
    f.calls.length=0;assert.throws(()=>upload(f.directory,'v0.2.0',f.invoke),/differs/);
    assert.equal(f.calls.filter(call=>call[1]==='upload').length,0);
  } finally {rmSync(f.root,{recursive:true,force:true});}
});
test('immutable releases, wrong tags, and missing completed manifests fail before upload/download',()=>{
  const f=fixture();try {
    assert.throws(()=>upload(f.directory,'v0.2.0',args=>{
      const result=f.invoke(args);return args[0]==='api'?JSON.stringify({...JSON.parse(result),immutable:true}):result;
    }),/Immutable/);
    assert.equal(f.calls.filter(call=>call[1]==='upload').length,0);
    assert.throws(()=>upload(f.directory,'v0.1.0',f.invoke),/manifest/);
    assert.throws(()=>download(join(f.root,'out'),'v0.2.0',f.invoke),/completed/);
    upload(f.directory,'v0.2.0',f.invoke);
    f.stored.delete([...f.stored.keys()][0]!);
    assert.throws(()=>download(join(f.root,'out'),'v0.2.0',f.invoke),/Missing release asset/);
  } finally {rmSync(f.root,{recursive:true,force:true});}
});
test('checksum corruption and incomplete or duplicate manifests cannot be recovered',()=>{
  const f=fixture();try {
    upload(f.directory,'v0.2.0',f.invoke);
    const name=readdirSync(f.directory).find(name=>name.endsWith('.tgz'))!;
    // Corrupt bytes must be rejected by the checksum before any platform-specific tar parser runs.
    f.stored.set(name,Buffer.from('not a tar archive'));
    assert.throws(()=>download(join(f.root,'out'),'v0.2.0',f.invoke),/checksum/);
    const manifest=verify(f.directory,'v0.2.0');
    assert.throws(()=>validateManifest({...manifest,files:[...manifest.files,manifest.files[0]]},'v0.2.0'),/Duplicate/);
    writeFileSync(join(f.directory,manifestName),JSON.stringify({...manifest,files:manifest.files.slice(1)}));
    assert.throws(()=>verify(f.directory,'v0.2.0'),/Incomplete/);
    f.calls.length=0;assert.throws(()=>upload(f.directory,'v0.2.0',f.invoke));assert.equal(f.calls.length,0);
  } finally {rmSync(f.root,{recursive:true,force:true});}
});

test('recovery command publishes only verified files and reports the directory before publication fails',()=>{
  const f=fixture();try {
    upload(f.directory,'v0.2.0',f.invoke);
    const output=join(f.root,'recovered');const events:string[]=[];
    assert.throws(()=>runAssets(['recover','v0.2.0',output],f.invoke,directory=>{
      assert.deepEqual(verify(directory,'v0.2.0'),verify(f.directory,'v0.2.0'));
      events.push('publish');throw new Error('npm unavailable');
    },message=>events.push(message)),/npm unavailable/);
    assert.deepEqual(events,[`Verified npm release files: ${output}`,'publish']);
    runAssets(['download','v0.2.0',join(f.root,'only')],f.invoke,()=>assert.fail('download published'),()=>{});
    const name=[...f.stored.keys()].find(name=>name.endsWith('.tgz'))!;
    f.stored.set(name,Buffer.from('not a tar archive'));
    assert.throws(()=>runAssets(['recover','v0.2.0',join(f.root,'bad')],f.invoke,()=>assert.fail('corrupt recovery published'),()=>{}),/checksum/);
  } finally {rmSync(f.root,{recursive:true,force:true});}
});
