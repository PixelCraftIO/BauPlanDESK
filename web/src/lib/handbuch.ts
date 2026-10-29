import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import type { Element, Root } from "hast";
import rehypeSlug from "rehype-slug";
import rehypeStringify from "rehype-stringify";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { visit } from "unist-util-visit";

// Das Web-Handbuch rendert die Markdown-Dateien aus
// docs/handbuch/benutzerhandbuch/ direkt – es gibt keine zweite Fassung.
// Links zwischen Kapiteln und auf Bilder werden auf die Web-Adressen umgeschrieben.

export const HANDBUCH_ORDNER = path.resolve(
  /*turbopackIgnore: true*/ process.env.BAUPLANIO_HANDBUCH ??
    path.join(/*turbopackIgnore: true*/ process.cwd(), "..", "docs", "handbuch", "benutzerhandbuch"),
);

export const PDF_DATEI = "Benutzerhandbuch.pdf";

export type Kapitel = {
  slug: string;
  nummer: number;
  titel: string;
};

export type Seite = Kapitel & {
  html: string;
  abschnitte: { id: string; titel: string }[];
};

const KAPITEL_MUSTER = /^(\d{2})-([a-z0-9-]+)\.md$/;

export async function kapitelListe(): Promise<Kapitel[]> {
  const dateien = (await readdir(HANDBUCH_ORDNER)).filter((d) => KAPITEL_MUSTER.test(d)).sort();
  return Promise.all(
    dateien.map(async (datei) => {
      const text = await readFile(path.join(/*turbopackIgnore: true*/ HANDBUCH_ORDNER, datei), "utf8");
      const nummer = Number(datei.slice(0, 2));
      return { slug: datei.replace(/\.md$/, ""), nummer, titel: ueberschrift(text).replace(/^\d+\s+/, "") };
    }),
  );
}

export async function startseite(): Promise<Seite> {
  // Die Pflegehinweise im README richten sich an die Autoren, nicht an Leser.
  return seite("README.md", { slug: "", nummer: 0, titel: "Benutzerhandbuch" }, (text) =>
    text.replace(/^## Pflege dieses Handbuchs\n[\s\S]*?(?=^## )/m, ""),
  );
}

export async function kapitel(slug: string): Promise<Seite | null> {
  const eintrag = (await kapitelListe()).find((k) => k.slug === slug);
  return eintrag ? seite(`${slug}.md`, eintrag) : null;
}

function ueberschrift(text: string) {
  return text.match(/^#\s+(.+)$/m)?.[1].trim() ?? "";
}

async function seite(datei: string, eintrag: Kapitel, anpassen = (text: string) => text): Promise<Seite> {
  const text = anpassen(await readFile(path.join(/*turbopackIgnore: true*/ HANDBUCH_ORDNER, datei), "utf8"));
  const abschnitte: { id: string; titel: string }[] = [];
  const html = String(
    await unified()
      .use(remarkParse)
      .use(remarkGfm)
      // Rohes HTML aus dem Markdown wird nicht übernommen (Standard von remark-rehype).
      .use(remarkRehype)
      .use(rehypeSlug)
      .use(() => (baum: Root) => webAdressen(baum, abschnitte))
      .use(rehypeStringify)
      .process(text),
  );
  return { ...eintrag, html, abschnitte };
}

function webAdressen(baum: Root, abschnitte: { id: string; titel: string }[]) {
  visit(baum, "element", (knoten: Element) => {
    if (knoten.tagName === "h2" && typeof knoten.properties.id === "string") {
      abschnitte.push({ id: knoten.properties.id, titel: textInhalt(knoten) });
    }
    if (knoten.tagName === "img" && typeof knoten.properties.src === "string") {
      knoten.properties.src = bildAdresse(knoten.properties.src);
      knoten.properties.loading = "lazy";
    }
    if (knoten.tagName === "a" && typeof knoten.properties.href === "string") {
      const ziel = linkAdresse(knoten.properties.href);
      if (ziel === null) {
        // Verweise auf Dateien außerhalb des Handbuchs (Skripte im Repository)
        // gibt es im Web nicht – nur den Text stehen lassen.
        knoten.tagName = "span";
        delete knoten.properties.href;
      } else {
        knoten.properties.href = ziel;
        if (/^https?:/.test(ziel)) {
          knoten.properties.target = "_blank";
          knoten.properties.rel = ["noopener", "noreferrer"];
        }
      }
    }
  });
}

function bildAdresse(src: string) {
  return src.startsWith("bilder/") ? `/handbuch/bilder/${src.slice("bilder/".length)}` : src;
}

export function linkAdresse(href: string): string | null {
  if (/^(https?:|mailto:|#)/.test(href)) return href;
  const [datei, anker] = href.split("#", 2);
  const marke = anker ? `#${anker}` : "";
  if (datei === "README.md") return `/handbuch${marke}`;
  if (datei === PDF_DATEI) return "/handbuch/pdf";
  if (datei.startsWith("bilder/")) return bildAdresse(datei);
  const treffer = datei.match(KAPITEL_MUSTER);
  if (treffer) return `/handbuch/${datei.replace(/\.md$/, "")}${marke}`;
  return null;
}

function textInhalt(knoten: Element | Root): string {
  return knoten.children
    .map((kind) => (kind.type === "text" ? kind.value : kind.type === "element" ? textInhalt(kind) : ""))
    .join("");
}
