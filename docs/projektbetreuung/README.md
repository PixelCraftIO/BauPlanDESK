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
Netzwerkbetrieb eine persönliche Kalenderadresse auf dem BauPlanDESK-Server.
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

## Planliste

**Ermittlung → Unterlagen → Planliste** führt die Pläne des Projekts mit
Plannummer, Büroindex und Planverteiler.

- **Aufnehmen:** Eine PDF-, DXF- oder IFC-Datei aus den Projektdateien wird mit
  Planart (LP, GR, SN, AN, DT, EW, FF, BS, SO), Geschoss/Bereich, laufender
  Nummer, Titel und Maßstab in die Liste übernommen. Ein Plan entspricht genau
  einem Dokument im DMS.
- **Plannummer:** setzt der Core beim Speichern nach dem Büroschema
  (**Einstellungen → Plannummern**, Vorgabe `{projekt}-{planart}-{geschoss}-{nr:2}`,
  z. B. `2026-014-GR-EG-03`). `{projekt}` ist das Kennzeichen der
  Projektorganisation, ersatzweise der Projektname. Leere Teile entfallen samt
  Trennzeichen. Eine Plannummer ist im Projekt eindeutig.
- **Büroindex:** Revisionen a, b, c … mit Datum, Änderungstext und der
  Dokumentversion, die den Index trägt. Liegt im DMS eine neuere Version ohne
  Index, zeigt die Liste „neuere Version ohne Index“.
- **Behördenindex:** Spalte „Behörde“ zeigt je Verfahren den aus versendeten
  Abgaben abgeleiteten Index A, B, C … und den Einreichungsstatus (Entwurf,
  eingereicht, nachgereicht, genehmigt, ersetzt). Büro- und Behördenindex
  werden nicht ineinander umgerechnet.
- **Planverteiler:** erfasste Weitergaben (Empfänger, Datum, Index, Weg) und
  automatisch alle versendeten oder eingegangenen Abgaben an die Behörde aus
  der Verfahrensakte.

Technik: `core_lib::planliste`, `ObjektArt::Planliste` (Objektkennung =
Dokument-ID), Tauri `planliste`, `plannummernschema[_speichern]`, Server
`GET /api/projekte/{id}/planliste`, `GET/PUT /api/plannummernschema`.

## Einreichungsprüfung

**Nachweis und Einreichung → Verfahren → Einreichungsprüfung** fasst vor dem
Absenden alle Prüfpunkte in einer Liste mit Ampel zusammen, wahlweise für ein
Verfahren:

| Bereich | Quelle |
|---|---|
| Grundlagen und Aufmaß, Bauvorlagen, Nachweise | Punkte der Projektübersicht (Unterlagenregister, Anforderungen der Verfahrensakte, Nachweisstände, Aufmaße) |
| Pläne und Planindex | Planliste: kein Index oder Datei entfernt (rot), neuere Datei ohne Index (rot), Behörde hat älteren Index (gelb) |
| Formulare | Formularinstanzen: Werte weichen von den Projektdaten ab (gelb); noch kein Formular (gelb) |
| Fachplaner | Lieferlisten: überfällig, ohne Termin oder binnen 7 Tagen fällig (rot), später fällig oder geliefert und ungeprüft (gelb) |
| Verfahren | Anforderungen fehlen, offene Nachforderungen (rot), Vorhabenprofil unbestätigt, Antwort eingereicht (gelb) |

Rot heißt „vor dem Einreichen erledigen“, gelb „bewusst prüfen“. Die Prüfung
liest nur und trifft keine Einreichungsentscheidung. Technik:
`core_lib::einreichungspruefung`, Tauri `einreichungspruefung`, Server
`GET /api/projekte/{id}/einreichungspruefung?verfahren=…`.

## Fachplaner und Lieferungen

**Übersicht → Fachplaner / Lieferungen** führt je Fachplaner eine Lieferliste
(`ObjektArt::Fachplanerlieferung`, Modul `fachplanerlieferung`): Disziplin,
Büro, Kontakt, angeforderte Unterlagen (abhakbar), Anfragedatum, zugesagter
Termin, Stand (offen, angefragt, teilweise, geliefert, geprüft, entfallen),
Zuständigkeit, gelieferte Dokumentversionen und optional ein Verfahren.
Ausstehende Lieferungen mit Termin erscheinen in Projektübersicht, Büro-Cockpit
und Kalender-Abo (`cockpit::lieferungstermine`).

## Rohbauwert, Gebühr und Baustatistik

**Formulare und Ausgabe → Rohbauwert, Gebühr, Baustatistik**. Projektwerte
stehen in `ObjektArt::Baukennwerte` (ein Datensatz je Projekt, Objektkennung =
Projekt-ID); Brutto-Rauminhalt, Nutzungsfläche, Wohnfläche, Wohnungen und
Vollgeschosse kommen aus den Berechnungen, eine Eingabe überschreibt sichtbar.

- Regelpaket `workflow-regeln/gebuehren.json`: Rohbauwert je m³ BRI nach
  Gebäudeart, Gebührensatz in Promille und Mindestgebühr je Land. Zunächst nur
  NRW, **`geprueft: false`**: die Werte sind Schätzwerte und vor der Nutzung
  gegen den Runderlass zu den Rohbauwerten und die AVerwGebO NRW zu prüfen; nur
  das Büro setzt `geprueft` auf `true`.
- Baustatistik: Arbeitshilfe für den Erhebungsbogen nach dem
  Hochbaustatistikgesetz mit Quelle je Merkmal; Bauherrenart aus der
  Kundenkategorie des ersten Bauherrn.
- Beide als PDF im Briefbogen (`baukennwerte_pdf`). Server
  `GET /api/projekte/{id}/gebuehrenschaetzung`, `…/baustatistik`.

## Sachstandsbericht

**Projektübersicht → Sachstandsbericht für Bauherrn …** setzt ein Schreiben im
Briefbogen an den ersten Bauherrn: Verfahrensstand je Verfahren, eingereichte
Abgaben, offene Nachforderungen, nächste Behördenfristen (höchstens zwölf) und
Bescheide. Aufgaben, Projekttagebuch und Fachplanerlieferungen sind bürointern
und fehlen bewusst. Technik: `core_lib::sachstandsbericht`, Tauri
`sachstandsbericht_pdf`, Server `GET /api/projekte/{id}/sachstandsbericht`
(liefert das Schreiben als Daten).

## Tägliche E-Mail

**Einstellungen → Tägliche E-Mail** (nur Netzwerkbetrieb): Adresse, Umfang
(nur eigene / alle Fristen), Abwesenheit bis und Vertretung. Gespeichert unter
`tagesmail.<Benutzer>`. Der Server stellt morgens je Empfänger die
überfälligen und in sieben Tagen fälligen Termine aus dem Büro-Cockpit
zusammen; ohne fällige Termine geht keine Nachricht. Während der Abwesenheit
erhält die Vertretung die Termine mit Vermerk, der Abwesende nichts.

Serverkonfiguration (fehlt `BAUPROJEKT_SMTP_URL`, ist der Versand aus):

| Variable | Beispiel |
|---|---|
| `BAUPROJEKT_SMTP_URL` | `smtps://benutzer:kennwort@mail.buero.de:465` oder `smtp://…:587?tls=required` |
| `BAUPROJEKT_MAIL_ABSENDER` | `BauPlanDESK <fristen@buero.de>` |
| `BAUPROJEKT_MAIL_UHRZEIT` | `06:30` (deutsche Ortszeit, Vorgabe) |

Der Versand eines Tages wird in der Datenbank beansprucht
(`tagesmail.versandtag`), damit mehrere Serverinstanzen nicht doppelt senden.
SMTP über `lettre` (MIT) mit rustls; `quoted_printable` (0BSD) ist in
`deny.toml` einzeln freigegeben.
