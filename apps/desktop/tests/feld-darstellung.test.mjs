import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {feldFehler,feldSichtbar,mehrzeilig} from '../src/lib/berechnungen/feldDarstellung.ts';
const feld=(name,typ='number')=>({name,typ,titel:name,optionen:[],readonly:false,gruppe:'daten',referenz:null,groesse:null});
test('Fehlende Pflichtzahl wird nicht mit ausdrücklicher Null verwechselt',()=>{
 assert.notEqual(feldFehler(feld('aufmass_m2'),null),'');
 assert.equal(feldFehler(feld('aufmass_m2'),0),'');
 assert.notEqual(feldFehler(feld('aufmass_m2'),NaN),'');
 assert.equal(feldFehler(feld('lichte_hoehe_m','optional_number'),null),'');
});
test('Aufmaßsichtbarkeit erhält Daten und lässt unbekannte Formen prüfbar',()=>{
 const row={form:'aufmass',laenge_m:8,breite_m:4,breite2_m:3,aufmass_m2:40}, vorher=structuredClone(row);
 assert.equal(feldSichtbar('wohnflaechen','positionen',feld('laenge_m'),row),false);
 assert.equal(feldSichtbar('wohnflaechen','positionen',feld('aufmass_m2'),row),true);
 assert.equal(feldSichtbar('wohnflaechen','positionen',feld('breite2_m'),{...row,form:'trapez'}),true);
 assert.equal(feldSichtbar('wohnflaechen','positionen',feld('laenge_m'),{...row,form:'alte_sonderform'}),true);
 assert.deepEqual(row,vorher);
});
test('Darstellungskonfiguration passt zu den tatsächlichen Formularverträgen',()=>{
 const config=JSON.parse(readFileSync(new URL('../src/lib/berechnungen/feldDarstellung.json',import.meta.url)));
 for(const [key,felder] of Object.entries(config.aufmass)){
  const [modul,pfad]=key.split('.');
  const vorlage=JSON.parse(readFileSync(new URL(`../../../regelwerke/fachberechnungen/${modul}.json`,import.meta.url)));
  const t=vorlage.tabellen.find(t=>t.pfad===pfad);assert.ok(t,key);
  for(const f of Object.keys(felder))assert.ok(t.felder.some(v=>v.name===f),`${key}.${f}`);
 }
 assert.equal(mehrzeilig(feld('wohnung','text')),false);
 assert.equal(mehrzeilig(feld('sondergrundlage','text')),true);
});

test('Neue Module werden automatisch in die passende Navigation aufgenommen',async()=>{
 const {navigationMitModulen}=await import('../src/lib/navigation.ts');
 const nav=navigationMitModulen([{id:'neuer_nachweis',gruppe:'Berechnungen'},{id:'neue_unterlage',gruppe:'Unterlagen'}]);
 assert.ok(nav.find(z=>z.id==='berechnungen').kinder.find(k=>k.id==='weitere').module.includes('neuer_nachweis'));
 assert.ok(nav.find(z=>z.id==='unterlagen').module.includes('neue_unterlage'));
});
