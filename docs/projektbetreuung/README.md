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

## Kalender-Abo

Unter **Einstellungen → Kalender-Abo** erzeugt jeder Benutzer im
Netzwerkbetrieb eine persönliche Kalenderadresse auf dem BauPlanIO-Server.
Outlook, Apple- und Google-Kalender abonnieren sie und zeigen die Fristen aller
lesbaren Projekte; der Kalender ist bei jedem Abruf aktuell.

- Umfang **Nur meine Zuständigkeit** (Termine mit dem Benutzer als
  Zuständigem) oder **Alle Fristen**.
- Die Adresse enthält einen zufälligen Schlüssel (256 Bit) und wird nur beim
  Anlegen angezeigt. Gespeichert wird nur sein SHA-256-Wert
  (`kalenderabo.<Benutzer>` in den Einstellungen). **Neue Adresse erzeugen**
  macht die alte ungültig, **Abo widerrufen** entfernt sie.
- Projektrechte werden bei jedem Abruf geprüft; ein deaktiviertes Konto oder
  eine entzogene Mitgliedschaft wirkt sofort.
- Im Einzelplatz gibt es kein Abo; dort bleibt der ICS-Export je Projekt.

Technik: `core_lib::kalenderabo`, Tauri `kalenderabo_*`, Server
`GET/POST/DELETE /api/kalenderabo` (angemeldet) und
`GET /kalender/{benutzer}/{schluessel}.ics` ohne Sitzung (falscher Schlüssel,
fremdes Konto oder ungültige Form: immer 404).

## Projekttagebuch

**Übersicht → Projekttagebuch** hält fest, was nicht als Behördenpost in die
Verfahrensakte gehört: Notizen, Telefonate, Besprechungen, Ortstermine,
Vorfälle/Mängel und Entscheidungen des Bauherrn.

- Ein Eintrag hat Datum (nicht in der Zukunft), Art, Betreff, Inhalt,
  Beteiligte, Anhänge (exakte Dokumentversionen aus den Projektdateien) und
  optional einen Bezug zu einer Verfahrensakte des Projekts.
- **Wiedervorlage:** Datum und Zuständigkeit. Offene Wiedervorlagen erscheinen
  in der Projektübersicht (Bereich Aufgaben), im Büro-Cockpit und im
  Kalender-Abo, bis sie als erledigt markiert sind.
- Jeder Eintrag ist ein eigener Fachdatensatz (`ObjektArt::Projekttagebuch`,
  Modul `projekttagebuch`) mit Revisionen und Protokolleintrag; parallele
  Einträge mehrerer Personen stören sich nicht.

Das Büro-Cockpit und das Kalender-Abo zeigen außerdem offene **Aufgaben** mit
Frist. Kalenderdateien werden einheitlich über
`verfahren::termine::kalender` erzeugt.
