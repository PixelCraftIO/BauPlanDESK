import { readFileSync, writeFileSync } from 'node:fs';
const version=process.argv[2];
if(!/^\d+\.\d+\.\d+$/.test(version ?? '')) throw Error('Aufruf: npm run version:app -- X.Y.Z');
for(const name of ['package.json','package-lock.json','src-tauri/tauri.conf.json']) {
 const path=new URL(`../${name}`,import.meta.url), value=JSON.parse(readFileSync(path));
 value.version=version;
 if(value.packages?.['']) value.packages[''].version=version;
 writeFileSync(path,JSON.stringify(value,null,2)+'\n');
}
console.log(`App-Version ${version} gesetzt. Änderungen prüfen und als v${version} taggen.`);
