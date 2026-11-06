// BETTER_AUTH_URL wird auch als Coolify-Liste oder ohne Schema angenommen.
import assert from "node:assert/strict";
import { test } from "node:test";
import { oeffentlicheAdressen } from "../src/lib/adresse.ts";

test("einzelne Adresse", () => {
  assert.deepEqual(oeffentlicheAdressen("https://bauplanio.de"), {
    baseURL: "https://bauplanio.de",
    trustedOrigins: ["https://bauplanio.de"],
  });
  assert.deepEqual(oeffentlicheAdressen("http://localhost:3000/"), {
    baseURL: "http://localhost:3000",
    trustedOrigins: ["http://localhost:3000"],
  });
});

test("Liste wie COOLIFY_URL, Anführungszeichen, Leerzeichen und Adressen ohne Schema", () => {
  assert.deepEqual(oeffentlicheAdressen("https://bauplanio.de,https://www.bauplanio.de"), {
    baseURL: "https://bauplanio.de",
    trustedOrigins: ["https://bauplanio.de", "https://www.bauplanio.de"],
  });
  assert.deepEqual(oeffentlicheAdressen('"https://bauplanio.de" '), {
    baseURL: "https://bauplanio.de",
    trustedOrigins: ["https://bauplanio.de"],
  });
  assert.deepEqual(oeffentlicheAdressen(" bauplanio.de, www.bauplanio.de "), {
    baseURL: "https://bauplanio.de",
    trustedOrigins: ["https://bauplanio.de", "https://www.bauplanio.de"],
  });
});

test("leer bleibt offen, Unsinn meldet BETTER_AUTH_URL", () => {
  assert.deepEqual(oeffentlicheAdressen(undefined), { trustedOrigins: [] });
  assert.deepEqual(oeffentlicheAdressen(""), { trustedOrigins: [] });
  assert.throws(() => oeffentlicheAdressen("https://"), /BETTER_AUTH_URL/);
  assert.throws(() => oeffentlicheAdressen("ftp://bauplanio.de"), /BETTER_AUTH_URL/);
});
