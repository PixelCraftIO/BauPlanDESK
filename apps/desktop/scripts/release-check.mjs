/** Fail before expensive release builds; never print key material. */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
export function releaseFehler(env, config, pkg) {
  const fehler = [];
  if (!/^\d+\.\d+\.\d+$/.test(config.version)) fehler.push('Für den stabilen Updatekanal eine Version X.Y.Z verwenden.');
  if (config.version !== pkg.version) fehler.push('App- und npm-Version stimmen nicht überein. npm run version:app -- X.Y.Z verwenden.');
  if (env.GITHUB_REF_TYPE === 'tag' && env.GITHUB_REF_NAME !== `v${config.version}`) fehler.push('Release-Tag passt nicht zur App-Version.');
  if (!env.TAURI_SIGNING_PRIVATE_KEY?.trim()) fehler.push('Secret TAURI_SIGNING_PRIVATE_KEY fehlt.');
  try {
    const text = Buffer.from(env.BAUPROJEKT_UPDATER_PUBKEY?.trim() ?? '', 'base64').toString('utf8').split(/\r?\n/);
    const key = Buffer.from(text[1] ?? '', 'base64');
    if (!text[0].startsWith('untrusted comment:') || key.length !== 42 || key.subarray(0,2).toString() !== 'Ed') throw Error();
  } catch { fehler.push('Repository-Variable BAUPROJEKT_UPDATER_PUBKEY fehlt oder hat kein Tauri-Public-Key-Format.'); }
  const source = env.BAUPROJEKT_UPDATE_URL?.trim() || (env.BAUPROJEKT_LIZENZSERVER?.trim() ? `${env.BAUPROJEKT_LIZENZSERVER.trim().replace(/\/$/,'')}/updates/latest.json` : 'https://github.com/PixelCraftIO/BauProjektIO/releases/latest/download/latest.json');
  try { const url = new URL(source); if (url.protocol !== 'https:' || url.username || url.password || url.hash) throw Error(); }
  catch { fehler.push('Updatequelle muss HTTPS ohne Zugangsdaten oder Fragment verwenden.'); }
  return fehler;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const config=JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json',import.meta.url)));
  const pkg=JSON.parse(readFileSync(new URL('../package.json',import.meta.url)));
  const errors=releaseFehler(process.env,config,pkg);
  for(const error of errors) console.error(`::error::${error}`);
  if(errors.length) process.exitCode=1;
  else console.log(`Release v${config.version}: Version, HTTPS-Quelle und Signierkonfiguration vorhanden.`);
}
