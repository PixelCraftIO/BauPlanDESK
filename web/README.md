# BauPlanIO – Webseite

Landingpage und Web-Handbuch für **bauplanio.de**, gebaut mit Next.js 16 und
[Better Auth](https://www.better-auth.com/). Die ganze Seite liegt hinter einer
Anmeldung: Ohne Sitzung ist nur `/login` erreichbar, alles andere (auch Bilder
und das PDF) leitet dorthin um.

- **Handbuch ohne Kopie:** `/handbuch` rendert die Markdown-Dateien aus
  [`docs/handbuch/benutzerhandbuch/`](../docs/handbuch/benutzerhandbuch/) beim Aufruf.
  Wer dort ein Kapitel ändert, ändert nach dem nächsten Deployment auch die
  Webseite. Neue Kapitel (`NN-name.md`) erscheinen automatisch in der Navigation.
- **Zeicheneditor und Abläufe:** `/zeicheneditor` ist ein Funktionsrundgang mit
  Bildern aus dem Handbuch (Inhalte in `src/lib/zeicheneditor.ts`), `/ablaeufe`
  zeigt als Ablaufdiagramme, wie die Funktionen in einem Projekt
  ineinandergreifen. Die Diagramme sind Daten in `src/lib/ablaeufe.ts` (Kästen
  mit Lage, Pfeile mit Beschriftung) und werden von `src/lib/diagramm.tsx` als
  Inline-SVG gezeichnet, hell und dunkel. Neue Editorbilder entstehen mit der
  Bildautomatik (`scripts/handbuch-bilder`, Szenen `editor-…`).
- **Anmeldung:** E-Mail und Passwort, keine Selbstregistrierung
  (`disableSignUp`). Das Startkonto kommt aus den Umgebungsvariablen und wird
  beim Start angelegt; Rollen, Sperren und weitere Konten sind über das
  Admin-Plugin von Better Auth vorbereitet.
- **Daten:** Konten und Sitzungen liegen in einer SQLite-Datei
  (`BAUPLANIO_DATENBANK`). Tabellen werden beim Start automatisch angelegt bzw.
  ergänzt.
- **Nicht öffentlich:** `robots.txt` sperrt alles, jede Antwort trägt
  `X-Robots-Tag: noindex`.

## Lokal starten

```bash
cd web
npm ci
cp .env.example .env.local   # Werte eintragen
npm run dev                  # http://localhost:3000
```

`npm run build` prüft Typen und baut, `npm run licenses` prüft die Lizenzen
der npm-Pakete (wie bei der Desktop-App: kein GPL/AGPL/LGPL). Next.js bringt
`sharp` (libvips, LGPL) als optionale Abhängigkeit mit; es wird per
`overrides` durch das leere Paket `vendor/kein-sharp` ersetzt, die
Bildoptimierung ist abgeschaltet.

## Umgebungsvariablen

| Variable | Pflicht | Bedeutung |
| --- | --- | --- |
| `BETTER_AUTH_URL` | ja | Öffentliche Adresse, z. B. `https://bauplanio.de` |
| `BETTER_AUTH_SECRET` | ja | Zufälliges Geheimnis, mind. 32 Zeichen (`openssl rand -base64 32`). Ändern meldet alle ab. |
| `BAUPLANIO_ADMIN_EMAIL` | ja | E-Mail-Adresse des Startkontos (Anmeldename) |
| `BAUPLANIO_ADMIN_PASSWORT` | ja | Passwort des Startkontos, mind. 12 Zeichen |
| `BAUPLANIO_ADMIN_NAME` | nein | Anzeigename beim Anlegen (Standard „Administrator“) |
| `BAUPLANIO_DATENBANK` | nein | Pfad der SQLite-Datei; im Container `/data/auth.sqlite` |
| `BAUPLANIO_HANDBUCH` | nein | Ordner des Handbuchs; im Container `/app/handbuch` |

**Passwort ändern:** `BAUPLANIO_ADMIN_PASSWORT` ändern und neu starten. Beim
Start wird das Passwort übernommen und alle bestehenden Sitzungen dieses
Kontos werden beendet.

## Deployment mit Coolify (bauplanio.de)

1. **DNS:** A-Record (und ggf. AAAA) für `bauplanio.de` und `www.bauplanio.de`
   auf die IP des VPS setzen.
2. **Ressource anlegen:** In Coolify *+ New → Private Repository (GitHub App)*
   → `PixelCraftIO/BauProjektIO`, Branch `main`.
3. **Build:** Build Pack *Dockerfile*, Base Directory `/`,
   Dockerfile Location `/web/Dockerfile`. (Der Build-Kontext muss die
   Repo-Wurzel sein, weil das Handbuch aus `docs/` kommt.)
4. **Netzwerk:** Ports Exposes `3000`. Domains
   `https://bauplanio.de,https://www.bauplanio.de` – Coolify holt die
   Let’s-Encrypt-Zertifikate selbst. Optional unter *Advanced* die
   Weiterleitung auf eine Domain (ohne `www`) wählen.
5. **Umgebungsvariablen:** die Pflichtwerte aus der Tabelle oben eintragen,
   `BETTER_AUTH_URL=https://bauplanio.de`. Geheimnis und Passwort *nicht* als
   Build-Variable markieren (werden nur zur Laufzeit gebraucht).
6. **Persistenter Speicher:** *Storages → + Add → Volume*, Ziel im Container
   `/data`. Ohne Volume gehen Konten und Sitzungen bei jedem Deployment
   verloren (das Startkonto wird dann zwar neu angelegt, alle sind aber
   abgemeldet).
7. **Healthcheck:** Das Image prüft selbst `GET /api/auth/ok`; in Coolify
   kann derselbe Pfad mit Port 3000 eingetragen werden.
8. *Deploy*. Danach `https://bauplanio.de` öffnen → Anmeldeseite → mit dem
   Startkonto anmelden.

Automatische Deployments: In Coolify *Webhooks/Auto Deploy* aktiv lassen,
dann baut jeder Push auf `main` die Seite neu – auch reine Handbuchänderungen.

Lokal das Image bauen und starten:

```bash
docker build -f web/Dockerfile -t bauplanio-web .
docker run --rm -p 3000:3000 -v bauplanio-daten:/data \
  -e BETTER_AUTH_URL=http://localhost:3000 \
  -e BETTER_AUTH_SECRET="$(openssl rand -base64 32)" \
  -e BAUPLANIO_ADMIN_EMAIL=ich@example.org \
  -e BAUPLANIO_ADMIN_PASSWORT='mindestens-12-zeichen' \
  bauplanio-web
```

## Aufbau

| Pfad | Inhalt |
| --- | --- |
| `src/lib/auth.ts` | Better-Auth-Konfiguration (SQLite, keine Registrierung, Admin-Plugin, Rate-Limit für Anmeldungen) |
| `src/lib/einrichtung.ts` | Migration und Startkonto beim Serverstart (`src/instrumentation.ts`) |
| `src/proxy.ts` | Leitet alles ohne gültige Sitzung auf `/login` um |
| `src/lib/handbuch.ts` | Markdown → HTML, schreibt Kapitel- und Bildlinks auf Web-Adressen um |
| `src/app/(geschuetzt)/` | Landingpage, Handbuch, Bilder- und PDF-Auslieferung (prüfen die Sitzung erneut) |
| `src/app/login/` | Anmeldeseite |

## Ausbau

- Weitere Konten: `auth.api.createUser` (Admin-Plugin) aus einer
  Verwaltungsseite oder einem Skript; Rollen über `role`.
- Öffentlich schalten: Landingpage-Route aus `config.matcher` in
  `src/proxy.ts` und aus dem Layout `(geschuetzt)` herausnehmen, `robots.ts`
  und `X-Robots-Tag` in `next.config.ts` anpassen.
- Mehr Last oder mehrere Instanzen: `database` in `auth.ts` auf PostgreSQL
  umstellen (`pg`-Pool), Rate-Limit-Speicher auf `"database"`.
