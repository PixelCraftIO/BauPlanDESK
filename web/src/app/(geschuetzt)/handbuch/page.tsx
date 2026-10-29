import type { Metadata } from "next";
import { startseite } from "@/lib/handbuch";

export const metadata: Metadata = { title: "Benutzerhandbuch" };

export default async function HandbuchStart() {
  const seite = await startseite();
  return <article className="handbuch-text" dangerouslySetInnerHTML={{ __html: seite.html }} />;
}
