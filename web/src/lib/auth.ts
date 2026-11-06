import { mkdirSync } from "node:fs";
import path from "node:path";
import Database from "better-sqlite3";
import { betterAuth, type BetterAuthOptions } from "better-auth";
import { nextCookies } from "better-auth/next-js";
import { admin } from "better-auth/plugins";
import { oeffentlicheAdressen } from "./adresse";

// Anmeldung mit Better Auth. Konten liegen in einer SQLite-Datei
// (BAUPLANIO_DATENBANK, im Container auf einem Volume unter /data).
//
// Bewusst geschlossen: keine Selbstregistrierung. Konten entstehen nur
// serverseitig (Startkonto aus den Umgebungsvariablen, siehe einrichtung.ts,
// später über das Admin-Plugin). Für den Ausbau stehen Rollen, Sperren und
// Sitzungsverwaltung des Admin-Plugins bereit.
function erzeugeOptionen() {
  const datei = path.resolve(/*turbopackIgnore: true*/ process.env.BAUPLANIO_DATENBANK ?? "./daten/auth.sqlite");
  mkdirSync(path.dirname(datei), { recursive: true });
  const datenbank = new Database(datei);
  datenbank.pragma("journal_mode = WAL");

  const { baseURL, trustedOrigins } = oeffentlicheAdressen(process.env.BETTER_AUTH_URL);

  return {
    appName: "BauPlanIO",
    ...(baseURL ? { baseURL, trustedOrigins } : {}),
    database: datenbank,
    emailAndPassword: {
      enabled: true,
      disableSignUp: true,
      minPasswordLength: 12,
    },
    session: {
      expiresIn: 60 * 60 * 24 * 7,
      updateAge: 60 * 60 * 24,
    },
    rateLimit: {
      enabled: true,
      window: 60,
      max: 100,
      customRules: {
        "/sign-in/email": { window: 60, max: 5 },
      },
    },
    plugins: [admin(), nextCookies()],
  } satisfies BetterAuthOptions;
}

type Optionen = ReturnType<typeof erzeugeOptionen>;
export type Auth = ReturnType<typeof betterAuth<Optionen>>;

// Erst beim ersten Zugriff erzeugen, damit `next build` weder Geheimnis noch
// Datenbank braucht.
const ablage = globalThis as unknown as { bauplanioOptionen?: Optionen; bauplanioAuth?: Auth };

/** Optionen ohne Auth-Instanz – für die Migration vor dem ersten Zugriff. */
export function authOptionen(): Optionen {
  ablage.bauplanioOptionen ??= erzeugeOptionen();
  return ablage.bauplanioOptionen;
}

export function getAuth(): Auth {
  ablage.bauplanioAuth ??= betterAuth(authOptionen());
  return ablage.bauplanioAuth;
}

export async function aktuelleSitzung(headers: Headers) {
  return getAuth().api.getSession({ headers });
}
