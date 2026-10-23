import { handbuch, type Kapitel } from "./handbuch.ts";

const html = (s: string) => s.replaceAll("&", "&amp;").replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
const nummer = (k: Kapitel) => String(handbuch.kapitel.indexOf(k) + 1).padStart(2, "0");
const titel = (k: Kapitel) => `${nummer(k)}. ${k.titel}`;
const anker = (k: Kapitel) => titel(k).toLocaleLowerCase("de-DE")
  .replace(/[^\p{L}\p{N}_\- ]/gu, "").replaceAll(" ", "-");
const md = (s: string) => s.replaceAll("\\", "\\\\").replaceAll("|", "\\|")
  .replaceAll("<", "&lt;").replaceAll(">", "&gt;");

export function handbuchMarkdown(): string {
  const out = [`# ${handbuch.titel}`, handbuch.untertitel,
    `Stand: ${handbuch.stand}`, "## Inhaltsverzeichnis",
    ...handbuch.kapitel.map((k) => `- [${titel(k)}](#${anker(k)})`)];
  for (const k of handbuch.kapitel) {
    out.push(`## ${titel(k)}`, k.kurztext);
    for (const a of k.abschnitte) {
      if (a.titel) out.push(`### ${a.titel}`);
      out.push(...(a.text ?? []).map(md));
      if (a.schritte) out.push(a.schritte.map((s, i) => `${i + 1}. ${md(s)}`).join("\n"));
      if (a.liste) out.push(a.liste.map((s) => `- ${md(s)}`).join("\n"));
      if (a.tabelle) out.push(`| ${a.tabelle.spalten.map(md).join(" | ")} |\n| ${a.tabelle.spalten.map(() => "---").join(" | ")} |\n${a.tabelle.zeilen.map((r) => `| ${r.map(md).join(" | ")} |`).join("\n")}`);
      if (a.hinweis) out.push(`> ${md(a.hinweis)}`);
      if (a.abbildung) out.push(`![${md(a.abbildung.alt)}](../../apps/desktop/public/handbuch/${a.abbildung.datei})`, md(a.abbildung.beschreibung));
    }
    if (k.siehe.length) out.push(`Weiterlesen: ${k.siehe.map((id) => {
      const ziel = handbuch.kapitel.find((v) => v.id === id)!;
      return `[${ziel.titel}](#${anker(ziel)})`;
    }).join(" · ")}`);
  }
  return out.join("\n\n") + "\n";
}

export const handbuchDruckCss = `
  @page { size: A4; margin: 20mm 17mm 20mm 19mm; }
  .handbuch-ausgabe { color:#182b39; background:white; font:11pt/1.55 Arial,sans-serif; }
  .handbuch-ausgabe * { box-sizing:border-box; }
  .handbuch-ausgabe h1 { font-size:27pt; line-height:1.2; }
  .handbuch-ausgabe h2 { font-size:18pt; line-height:1.25; margin-top:1.7em; }
  .handbuch-ausgabe h3 { font-size:12pt; margin-top:1.5em; }
  .handbuch-ausgabe h1,.handbuch-ausgabe h2,.handbuch-ausgabe h3 { break-after:avoid; color:#182b39; }
  .handbuch-ausgabe .kapitel { border:0; border-radius:0; padding:0; margin:0; background:white; }
  .handbuch-ausgabe p,.handbuch-ausgabe li { orphans:3; widows:3; }
  .handbuch-ausgabe ol { list-style:decimal outside; padding-left:1.5em; }
  .handbuch-ausgabe ul { list-style:disc outside; padding-left:1.5em; }
  .handbuch-ausgabe li { display:list-item; border:0; padding:0; margin-bottom:.55em; }
  .handbuch-ausgabe table { border-collapse:collapse; width:100%; font-size:10pt; margin:1em 0; table-layout:fixed; }
  .handbuch-ausgabe th,.handbuch-ausgabe td { border:1px solid #bbc9d1; padding:7pt; text-align:left; vertical-align:top; overflow-wrap:anywhere; }
  .handbuch-ausgabe th { background:#edf2f5; }
  .handbuch-ausgabe thead { display:table-header-group; }
  .handbuch-ausgabe tr { break-inside:avoid; }
  .handbuch-ausgabe .hinweis { padding:10pt 12pt; border-left:3pt solid #486575; background:#f2f5f7; }
  .handbuch-ausgabe a { color:inherit; text-decoration:underline; }
  .handbuch-ausgabe .weiter { margin-top:1.5em; font-size:10pt; }
  .handbuch-ausgabe .titelblatt { display:block; border:0; border-bottom:2pt solid #35566b; padding:38mm 0 2em; min-height:230mm; margin:0; background:white; }
  .handbuch-ausgabe .abbildungsabschnitt { break-inside:avoid; }
  .handbuch-ausgabe figure { margin:16pt 0; break-inside:avoid; }
  .handbuch-ausgabe figure img { display:block; width:100%; height:auto; border:1px solid #bbc9d1; }
  .handbuch-ausgabe figcaption { margin-top:7pt; font-size:9pt; line-height:1.45; color:#425563; }
  .handbuch-ausgabe .inhaltsverzeichnis { columns:2; column-gap:20pt; }
  .handbuch-ausgabe .toc-zeile { display:flex; gap:8pt; justify-content:space-between; text-decoration:none; }
  .handbuch-ausgabe .toc-zeile > span:first-child { min-width:0; flex:1; }
  .handbuch-ausgabe .kapitelseite { flex:0 0 20pt; min-width:20pt; text-align:right; font-variant-numeric:tabular-nums; }
  .handbuch-ausgabe .inhaltsverzeichnis li { break-inside:avoid; font-size:10pt; }
  @media screen { .handbuch-ausgabe { max-width:210mm; margin:2rem auto; padding:20mm; box-shadow:0 1px 12px #0002; } }
  @media print { .handbuch-ausgabe .kapitel { break-before:page; } .handbuch-ausgabe .inhalt { break-before:page; } }
`;

export function handbuchInhaltHtml(bildQuelle: (datei: string) => string = (datei) => `/handbuch/${datei}`): string {
  return `<header class="titelblatt"><h1>${html(handbuch.titel)}</h1><p>${html(handbuch.untertitel)}</p><p>Stand: ${html(handbuch.stand)}</p><p>Von der Projektanlage bis zur Einreichung, Nachbearbeitung und dokumentierten Genehmigung.</p><p>Ausführliche Ausgabe mit bebilderten Arbeitsabläufen, Kontrolltabellen und Praxisbeispielen.</p><p>Abbildungen: fiktive Beispielprojekte. Grundlage: ${html(handbuch.basis)}.</p></header>
  <nav class="inhalt" aria-label="Inhaltsverzeichnis"><h2>Inhaltsverzeichnis</h2><ol class="inhaltsverzeichnis">${handbuch.kapitel.map((k) => `<li><a class="toc-zeile" href="#${k.id}"><span>${html(k.titel)}</span><span class="kapitelseite" data-kapitel="${k.id}"></span></a></li>`).join("")}</ol></nav>
  ${handbuch.kapitel.map((k) => `<section class="kapitel" id="${k.id}"><h2>${html(titel(k))}</h2><p>${html(k.kurztext)}</p>${k.abschnitte.map((a) => {
    let out = a.titel ? `<h3>${html(a.titel)}</h3>` : "";
    out += (a.text ?? []).map((s) => `<p>${html(s)}</p>`).join("");
    for (const [tag, list] of [["ol", a.schritte], ["ul", a.liste]] as const)
      if (list) out += `<${tag}>${list.map((s) => `<li>${html(s)}</li>`).join("")}</${tag}>`;
    if (a.tabelle) out += `<table><thead><tr>${a.tabelle.spalten.map((s) => `<th scope="col">${html(s)}</th>`).join("")}</tr></thead><tbody>${a.tabelle.zeilen.map((r) => `<tr>${r.map((s) => `<td>${html(s)}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
    if (a.hinweis) out += `<aside class="hinweis">${html(a.hinweis)}</aside>`;
    if (a.abbildung) out += `<figure><img src="${html(bildQuelle(a.abbildung.datei))}" alt="${html(a.abbildung.alt)}"><figcaption>${html(a.abbildung.beschreibung)}</figcaption></figure>`;
    return a.abbildung ? `<div class="abbildungsabschnitt">${out}</div>` : out;
  }).join("")}<p class="weiter">Weiterlesen: ${k.siehe.map((id) => `<a href="#${id}">${html(handbuch.kapitel.find((v) => v.id === id)!.titel)}</a>`).join(" · ")}</p></section>`).join("")}`;
}
export function handbuchHtml(bildQuelle?: (datei: string) => string): string {
  return `<!doctype html><html lang="de"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${html(handbuch.titel)}</title><style>${handbuchDruckCss}</style></head><body><main class="handbuch-ausgabe">${handbuchInhaltHtml(bildQuelle)}</main></body></html>\n`;
}
