# Oberfläche: Konflikte, Dialogschutz, Projektrechte und Aktualität

Korrekturen aus der dritten UI-Prüfung (Befunde 1–11 und 13–16). Die Fachregeln
bleiben im Core; die Oberfläche übernimmt Ergebnisse und Rechte von dort.

## Verhalten

### Versionskonflikte erhalten Eingaben

- **Kunden:** Bei einem Konflikt wird die aktuelle Fassung zur Grundlage (neue
  Version). Die eigenen Eingaben bleiben stehen. Felder, die nur am anderen
  Arbeitsplatz geändert wurden, werden übernommen. Felder, die beide Seiten
  unterschiedlich geändert haben, erscheinen als Abweichung mit dem aktuellen
  Wert und dem Knopf „Aktuellen Wert übernehmen“. Ist der Kunde inzwischen
  gelöscht, bleiben die Eingaben sichtbar und werden nicht gespeichert.
  „Alle speichern und beenden“ und Strg+S melden einen Konflikt. Die Anwendung
  bleibt dann geöffnet.
- **Projektdaten:** Dasselbe Verfahren gilt im Formular. Die neue Fassung kommt
  über `start` herein. Ein Hinweis im Dialog nennt sie; der irreführende Text
  „bitte Änderungen erneut eintragen“ entfällt. Schlägt das Neuladen fehl,
  bleibt die bisherige Meldung mit erhaltenen Eingaben.
- **Bauherren / Weitere Beteiligte:** Die Auswahl bleibt erhalten. Der Dialog
  lädt Projektversion und gespeicherte Zuordnung nach und nennt diese. Erneutes
  „Speichern“ ist ohne Schließen möglich. Ein Fehler beim Nachladen wird
  angezeigt. Bisher setzte eine neue Projektversion die Auswahl über einen
  reaktiven Ladeeffekt zurück; der Dialog lädt jetzt nur beim Öffnen.

Der Abgleich (`apps/desktop/src/lib/abgleich.ts`) ist reiner
Oberflächenzustand. Optimistic Locking bleibt im Core: Erneutes Speichern
sendet die neue Version und wird dort wieder geprüft.

### Dialoge fragen vor dem Verwerfen

Bauherren, Weitere Beteiligte, Büroangaben, Projektdaten und Neues Projekt
bieten wie „Projektmitglieder“ `hatAenderungen()`, `istBeschaeftigt()` und
`schliessenErlaubt()`. Escape, „Abbrechen“ und Menübefehle (Neu, Öffnen,
zuletzt geöffnet, Kunden, Benutzer, Büro, Projektdaten, Lizenz, Schließen,
Importieren, Ansichten) fragen vor dem Verwerfen nach. Strg+S, Rückgängig,
Wiederholen und die Updateprüfung lassen den Dialog unberührt.

### Strg+S speichert die sichtbare Ansicht

`anwendung.speichern` speichert die sichtbare Projektakte, die Kundendaten
oder die Einstellungen (Druckvorlage, geöffnete Kategorie). Bei offenem Dialog
und in Ansichten ohne Speicherfunktion erscheint ein Hinweis. Eine
ausgeblendete Akte wird nicht mehr im Hintergrund gespeichert.

### Einstellungen: ungespeichert statt „laufende Aktion“

Druckvorlage und Kategorienbearbeitung melden Änderungen über `hatAenderungen()`.
Sie gelten nicht mehr als laufende Aktion. Beim Beenden erscheint deshalb der
Dialog „Anwendung schließen?“ mit „Alle speichern und beenden“ statt der
falschen Meldung „Bitte laufende Aktionen zuerst abschließen“.

### Projektrechte

`CoreServices::eigenes_projektrecht` liefert das wirksame Projektrecht nach
der Regel von `projekt_zugriff`: Administratoren erhalten `leitung`, Mitglieder
ihr Recht. Wer nicht lesen darf, erhält `Forbidden`. Desktop
(`projekt_recht`) und Server (`GET /projekte/{id}/recht`) sind dünne Adapter.

Die Oberfläche lädt das Recht beim Öffnen und bei jeder Aktivierung der Akte.
Bei nur lesendem Projektrecht sieht die Akte den Benutzer mit der Rolle
„Lesen“, wie ohne Schreiblizenz. Formulare und Knöpfe sind dann
schreibgeschützt. „Projektmitglieder …“ erscheint nur bei Projektrecht
`leitung` und Rolle Projektleitung, wie der Core es zum Ändern verlangt.
Geprüft wird weiterhin an der Servicegrenze.

### Ältere Fachstände öffnen ohne Scheinänderung

`CoreServices::fachobjekte` liest bekannte Module über ihren typisierten
Struct und liefert sie in aktueller Form. Dazu gehören Verfahrensakte,
Fachpläne, Abstandsplan, Gebäude, Aufgaben, Unterlagen, Dokumentdaten,
Raumbuch und Projektorganisation. Felder mit `#[serde(default)]` wie
`schritte` und `antragsangaben` sind ergänzt, sodass `original` und `eingabe`
der Oberfläche übereinstimmen. Das gilt für SQLite und PostgreSQL gleich,
weil die Normalisierung im Core und nicht in den DB-Adaptern liegt. Die Ablage
und die Revision bleiben unverändert, bis jemand speichert. Datensätze einer
neueren Schemaversion oder mit unbekannten Feldern bleiben unverändert; die
Speichergrenze meldet sie ausdrücklich.

### Aktuelle Auswahllisten in ausgeblendeten Bereichen

`projektEreignisse.ts` meldet innerhalb einer Akte, dass Dokumente,
Fachobjekte oder Nachweise geändert wurden. Gemeldet wird nach erfolgreichen
Ablagen über die API-Hülle (Import, Versionswiederherstellung,
Formular/Druck/E-Mail-Anhang ablegen, Arbeitskopie übernehmen), nach
Fachobjekt-Speichern und nach Nachweisaktionen. Die Dateiverwaltung meldet
außerdem jede eigene Änderung ihrer Liste und lädt beim Einblenden neu. Der
Bauantrag aktualisiert die Anlagenauswahl. Fachberechnungen laden Gebäude,
Dokumente, Nachweise und Raumbücher neu, Baugrundstücke bei neuer
Projektversion. Nach dem Speichern der Büroangaben lädt der Bauantrag die
Entwurfsverfasser und die fehlenden Angaben neu.

### Prüfbericht und Ergebnisaktualität

Der Prüfbericht einer Fachberechnung wird beim Wechsel des Eintrags
zurückgesetzt. Seine Signatur umfasst neben den Eingaben die Projektversion
und die Revisionen der Gebäude und Raumbücher. Ändern sich diese Quellen,
gilt der Bericht als veraltet („Verknüpfte … Daten haben sich geändert“).

### Faktor für Nichtvollgeschosse aus dem Core

`FlaechenErgebnis.nichtvollgeschoss_faktor` (neu, optional) enthält den
wirksamen Faktor nach Regelwerk und, nur mit Bebauungsplan, Planfestsetzung.
Das Flächenformular zeigt diesen Wert statt einer eigenen Regel. Vor der ersten
Prüfung zeigt es einen neutralen Hinweis und blendet die Teilflächenfelder ein.

### Fehleranzeige und Rückfragen

- Ladefehler bei Beteiligten, Kunden, Büroangaben und Dokumenten (Bauantrag)
  sowie bei Baugrundstücken, Gebäuden, Dokumenten und Nachweisen
  (Fachberechnung) erscheinen als Fehlermeldung statt als leere Liste.
- „Entwurf verwerfen“ in Fachlisten fragt vorher nach; es löscht auch die
  lokale Entwurfssicherung.
- Die Lizenz-Abmeldung verwendet `ask` aus `@tauri-apps/plugin-dialog`.

## Verbraucher und Kompatibilität

- **Vertragsänderungen (additiv):** neue Core-Methode `eigenes_projektrecht`,
  neuer Desktop-Befehl `projekt_recht`, neue Server-Route
  `GET /projekte/{id}/recht` und das neue optionale Feld
  `FlaechenErgebnis.nichtvollgeschoss_faktor`. Das Feld hat
  `#[serde(default)]`; ältere gespeicherte Ergebnisse lesen es als `None`.
- **Lesevertrag `fachobjekte`:** Die Datenform entspricht jetzt der, die eine
  Speicherung ablegen würde. Fehlende Default-Felder sind ergänzt, explizite
  `null` bei `skip_serializing_if` können entfallen. Interne Core-Verbraucher
  lesen typisiert und sind nicht betroffen. Fingerabdrücke und
  Aktualitätsprüfungen verwenden weiterhin die gespeicherten Rohdaten.
- **Keine Datenmigration:** Gespeicherte Projekte, Fachstände und Vorlagen
  bleiben unverändert. Revisionen ändern sich nicht durch Lesen.
- **Regeländerung:** keine. Die Oberfläche übernimmt den bestehenden
  Core-Faktor; Projektrechte folgen der bestehenden Zugriffsregel.

## Validierung

- Storage: `altbestand_lesen` (alte Verfahrensakte ohne `schritte` und
  `antragsangaben`, unveränderte Ablage und Revision, neuere oder unbekannte
  Datensätze unverändert, eigenes Projektrecht für alle Mitgliedschaften).
- Core: `flaechen::wirksamer_nichtvollgeschossfaktor_steht_im_ergebnis`
  (Regelwerk, Planfestsetzung nur mit Plan, Altergebnis ohne Feld).
- Desktop: `command_zugriff_tests` prüft `projekt_recht` für fremde und
  lesende Projekte.
- Browserregression `apps/desktop/tests/fehler-regressionen.py`:
  Projektkonflikt mit erhaltenen Eingaben und übernommener fremder Änderung,
  Bauherrenkonflikt mit erneutem Speichern, Escape- und Menü-Rückfragen,
  Strg+S ohne speicherbare Ansicht, Kundenkonflikt bei Strg+S und Beenden
  sowie das Projektrecht „Lesen“.
