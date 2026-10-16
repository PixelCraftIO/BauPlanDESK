import { test } from 'node:test';
import assert from 'node:assert/strict';
import { releaseFehler } from '../scripts/release-check.mjs';
const bytes=Buffer.alloc(42);bytes.write('Ed');
const pub=Buffer.from(`untrusted comment: test public key\n${bytes.toString('base64')}\n`).toString('base64');
const env={BAUPROJEKT_UPDATER_PUBKEY:pub,TAURI_SIGNING_PRIVATE_KEY:'presence-only-fixture',GITHUB_REF_TYPE:'tag',GITHUB_REF_NAME:'v0.1.1'};
const config={version:'0.1.1'};
test('Release verlangt passende Version, Tag, Schlüssel und HTTPS',()=>{
 assert.deepEqual(releaseFehler(env,config,config),[]);
 for(const patch of [{TAURI_SIGNING_PRIVATE_KEY:''},{BAUPROJEKT_UPDATER_PUBKEY:'placeholder'},{GITHUB_REF_NAME:'v0.1.0'},{BAUPROJEKT_UPDATE_URL:'http://updates.test/latest.json'},{BAUPROJEKT_UPDATE_URL:'https://user:password@updates.test/latest.json'}]) {
  assert.ok(releaseFehler({...env,...patch},config,config).length);
 }
 assert.ok(releaseFehler(env,config,{version:'0.1.0'}).length);
});
