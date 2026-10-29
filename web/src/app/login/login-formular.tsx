"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { authClient } from "@/lib/auth-client";

export function LoginFormular({ ziel }: { ziel: string }) {
  const router = useRouter();
  const [fehler, setFehler] = useState<string | null>(null);
  const [laeuft, setLaeuft] = useState(false);

  async function anmelden(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const daten = new FormData(event.currentTarget);
    setFehler(null);
    setLaeuft(true);
    const { error } = await authClient.signIn.email({
      email: String(daten.get("email")),
      password: String(daten.get("passwort")),
      rememberMe: daten.get("merken") === "on",
    });
    if (error) {
      setLaeuft(false);
      setFehler(
        error.status === 429
          ? "Zu viele Versuche. Bitte warten Sie eine Minute."
          : "E-Mail-Adresse oder Passwort ist nicht korrekt.",
      );
      return;
    }
    router.replace(ziel);
    router.refresh();
  }

  return (
    <form onSubmit={anmelden} className="formular">
      <label>
        E-Mail-Adresse
        <input name="email" type="email" autoComplete="username" required autoFocus />
      </label>
      <label>
        Passwort
        <input name="passwort" type="password" autoComplete="current-password" required />
      </label>
      <label className="auswahl">
        <input name="merken" type="checkbox" defaultChecked /> Angemeldet bleiben
      </label>
      {fehler && (
        <p className="meldung fehler" role="alert">
          {fehler}
        </p>
      )}
      <button type="submit" className="knopf primaer" disabled={laeuft}>
        {laeuft ? "Anmelden …" : "Anmelden"}
      </button>
    </form>
  );
}
