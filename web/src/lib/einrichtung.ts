import { getMigrations } from "better-auth/db/migration";
import { authOptionen, getAuth } from "./auth";

// Läuft einmal beim Serverstart (instrumentation.ts): Tabellen anlegen bzw.
// ergänzen und das Startkonto aus den Umgebungsvariablen sicherstellen.
// Das Passwort aus BAUPLANIO_ADMIN_PASSWORT gilt immer: Wer es dort ändert und
// neu startet, hat danach das neue Passwort.
export async function einrichten() {
  // Vor dem Erzeugen der Auth-Instanz migrieren, sonst meldet ihre
  // Schemaprüfung beim ersten Start fehlende Tabellen.
  const { runMigrations } = await getMigrations(authOptionen());
  await runMigrations();
  const auth = getAuth();

  const email = process.env.BAUPLANIO_ADMIN_EMAIL?.trim().toLowerCase();
  const passwort = process.env.BAUPLANIO_ADMIN_PASSWORT;
  const name = process.env.BAUPLANIO_ADMIN_NAME?.trim() || "Administrator";
  if (!email || !passwort) {
    console.warn("[bauplanio] BAUPLANIO_ADMIN_EMAIL/BAUPLANIO_ADMIN_PASSWORT fehlen – kein Startkonto angelegt.");
    return;
  }
  if (passwort.length < 12) {
    throw new Error("BAUPLANIO_ADMIN_PASSWORT muss mindestens 12 Zeichen lang sein.");
  }

  const ctx = await auth.$context;
  const vorhanden = await ctx.internalAdapter.findUserByEmail(email, { includeAccounts: true });
  if (!vorhanden) {
    await auth.api.createUser({ body: { email, password: passwort, name, role: "admin" } });
    console.info(`[bauplanio] Startkonto ${email} angelegt.`);
    return;
  }

  const konto = vorhanden.accounts.find((a) => a.providerId === "credential");
  const passt = konto?.password ? await ctx.password.verify({ hash: konto.password, password: passwort }) : false;
  if (!passt) {
    const hash = await ctx.password.hash(passwort);
    if (konto) {
      await ctx.internalAdapter.updatePassword(vorhanden.user.id, hash);
    } else {
      await ctx.internalAdapter.linkAccount({
        userId: vorhanden.user.id,
        providerId: "credential",
        accountId: vorhanden.user.id,
        password: hash,
      });
    }
    // Alte Sitzungen nach einem Passwortwechsel beenden.
    await ctx.internalAdapter.deleteUserSessions(vorhanden.user.id);
    console.info(`[bauplanio] Passwort für ${email} aus der Umgebung übernommen.`);
  }
}
