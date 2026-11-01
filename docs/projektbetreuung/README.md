# Projektbetreuung im Büro

Ergänzungen aus dem Ausbaukonzept vom 01.11.2026: büroweite Sicht auf Fristen
und Verfahren, Kalender-Abo, Projekttagebuch, Planliste, Abgabeprüfung,
Rohbauwert und Sachstandsbericht. Die Verfahrensakte bleibt die einzige Quelle
für Fristen und Verfahrensstände; die Module hier lesen sie nur.

## Büro-Cockpit

Die Startseite (**Projekte**) zeigt oberhalb des Projektverzeichnisses das
**Büro-Cockpit**. Es fasst alle Projekte zusammen, die der angemeldete Benutzer
lesen darf.

- **Fristen und Wiedervorlagen:** alle laufenden Termine aller Verfahrensakten
  (Nachforderungen, Nebenbestimmungen, Anhörung, Zurückstellung, Beteiligung,
  Rechtsbehelfsfrist, Geltungsdauer, Fertigstellung, Bearbeitungsfrist,
  Zahlung) mit Stufe **Überfällig**, **Diese Woche** (bis 7 Tage), **30 Tage**
  und **Später**. Filter: Zuständigkeit (alle / nur meine), Zeitraum, Suche.
  Ein Klick auf den Betreff öffnet das Projekt im passenden Register der Akte.
- **Laufende Verfahren:** je Projekt und Verfahren Stand, Aktenzeichen,
  Behörde, letzte Abgabe (Versand/Eingang oder externe Einreichung), offene
  Forderungspunkte, letzter Bescheid und nächste Frist.
- **Ablehnung ohne Folgeverfahren:** Verfahren, deren jüngster Bescheid eine
  Ablehnung ist und zu denen noch kein Folgeverfahren angelegt ist, werden
  hervorgehoben. Die Verfahrensakte zeigt dort die Handlungsoptionen.

Technik: `core_lib::cockpit` (reines Lesemodell, rechnet keine Fristen selbst,
sondern verwendet `verfahren::termine::termine`), Tauri-Befehl `buero_cockpit`,
Server `GET /api/cockpit`. Projekte, deren Akten nicht lesbar sind, erscheinen
mit Meldung statt still zu fehlen.
