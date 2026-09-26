import assert from 'node:assert/strict';
import {test} from 'node:test';
import {selectTarget} from '../scripts/release.mts';

test('host selection distinguishes glibc from musl on both Linux architectures',()=>{
  for(const cpu of ['x64','arm64']) {
    assert.equal(selectTarget('linux',cpu,'glibc').suffix,`linux-${cpu}-gnu`);
    assert.equal(selectTarget('linux',cpu,'musl').suffix,`linux-${cpu}-musl`);
  }
  assert.equal(selectTarget('win32','arm64').suffix,'win32-arm64-msvc');
  assert.equal(selectTarget('darwin','x64').suffix,'darwin-x64');
  for(const [os,cpu,libc] of [['linux','x64',undefined],['linux','x64','unknown'],['linux','arm','musl'],['freebsd','x64',undefined]]) {
    assert.throws(()=>selectTarget(os!,cpu!,libc),/Unsupported/);
  }
});
