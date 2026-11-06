// Öffentliche Adresse der Webseite aus BETTER_AUTH_URL.
//
// Coolify stellt die Adressen einer Anwendung als Liste bereit
// (COOLIFY_URL = "https://bauplanio.de,https://www.bauplanio.de"). Wird
// BETTER_AUTH_URL darauf gesetzt oder fehlt das Schema, bricht Better Auth
// den Start mit „Invalid base URL“ ab; ebenso bei Leerzeichen oder
// Anführungszeichen um den Wert. Deshalb: Liste, Anführungszeichen und Adressen
// ohne Schema annehmen, die erste Adresse ist die Basis, alle sind vertrauenswürdige
// Herkunft.

export type Adressen = { baseURL?: string; trustedOrigins: string[] };

export function oeffentlicheAdressen(wert: string | undefined): Adressen {
  const eintraege = (wert ?? "")
    .split(/[,\s]+/)
    .map((e) => e.trim().replace(/^["']+|["']+$/g, ""))
    .filter(Boolean);
  if (eintraege.length === 0) return { trustedOrigins: [] };
  const herkuenfte = eintraege.map((e) => {
    const mitSchema = /^[a-z][a-z0-9+.-]*:\/\//i.test(e) ? e : `https://${e}`;
    let url: URL;
    try {
      url = new URL(mitSchema);
    } catch {
      throw new Error(
        `BETTER_AUTH_URL enthält keine gültige Adresse („${e}“). Erwartet wird z. B. https://bauplanio.de`,
      );
    }
    if (url.protocol !== "https:" && url.protocol !== "http:") {
      throw new Error(`BETTER_AUTH_URL muss mit https:// oder http:// beginnen („${e}“).`);
    }
    return url.origin;
  });
  return { baseURL: herkuenfte[0], trustedOrigins: [...new Set(herkuenfte)] };
}
