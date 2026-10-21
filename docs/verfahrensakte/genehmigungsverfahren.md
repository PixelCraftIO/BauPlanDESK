# Genehmigungsverfahren begleiten

Ergänzt die [Verfahrensakte](README.md) um die Schritte zwischen Antrag und
Nutzungsaufnahme: Verfahrenswahl, Abweichungen, Unterschriften, behördliche
Nachrichten, Schreiben an die Behörde, Ablehnung und Rechtsbehelf sowie die
Phase nach der Genehmigung. Die Fachlogik liegt in `core-lib::verfahren`
(`abweichungen`, `landesrecht`, `eingang`, `schreiben`, `optionen`,
`ausfuehrung`) und in `core-lib::xbau` (`eingang`, `folge`). Tauri und HTTP sind
dünne Adapter.

Die Anwendung **entscheidet keine Rechtsfrage**. Sie nennt Bedingungen,
Fundstellen und fehlende Angaben, sperrt Abgaben bei erkennbaren Lücken und
überlässt die Entscheidung den verantwortlichen Personen. Ein verwendetes
Regelpaket bleibt so lange ungeprüft, bis es fachlich abgenommen ist. Die
Oberfläche zeigt diesen Stand an.

## Lage und Handlungsoptionen

Über allen Registern der Akte steht die **Lage**: ein Satz zum Verfahrensstand
und die daraus folgenden Optionen. Jede Option hat eine Frist, eine Fundstelle
und, wenn möglich, eine direkte Aktion:

| Lage | Optionen |
|---|---|
| Anhörung offen | Stellungnahme abgeben, Planung ändern und nachreichen, Abweichung oder Befreiung beantragen, Fristverlängerung beantragen, Antrag zurücknehmen |
| Abgelehnt | Rechtsbehelf prüfen (Frist ab Zustellung), Geänderten Antrag stellen (Folgeverfahren), Vorbescheid zu den strittigen Fragen, Ablehnungsgründe erfassen |
| Entscheidung ausstehend | Genehmigungsfiktion (voraussichtliches Datum, falls die Fassung eine kennt); nach überschrittener Bearbeitungsfrist oder drei Monaten Sachstand erfragen und Untätigkeitsklage prüfen (§ 75 VwGO) |
| Genehmigt | Bekanntgabe an Nachbarn, Anfechtung durch Nachbarn, Baubeginn vorbereiten |

Die Genehmigungsfiktion richtet sich nach der Fassung, die am Tag des
Antragseingangs gilt. Für NRW ist ab 01.09.2026 eine Fiktion von drei Monaten
(regulär) bzw. sechs Wochen (vereinfacht) ab Vollständigkeit hinterlegt.
Fristhemmungen durch Nachforderungen werden nicht geschätzt. Ohne
Vollständigkeitsdatum oder Prüfverfahren erscheint keine Fiktion.

## Verfahrenswahl und Landesrecht

`workflow-regeln/landesrecht.json` enthält je Land und Fassung (`gilt_ab`,
`gilt_bis`) die Verfahren mit Bedingungen und Fundstellen, die
Genehmigungsfiktion und die Unterschriftsregeln. Hinterlegt sind die BauO NRW
bis 31.08.2026 und ab 01.09.2026; beide Fassungen tragen `geprueft: false`.

**Stammdaten → Verfahrenswahl** wertet Wohngebäude, Gebäudeklasse,
Bebauungsplan, Erschließung, Sonderbau und geplante Abweichungen aus. Jedes
Verfahren wird als geeignet, ungeeignet oder ungeklärt angezeigt, mit der
Bedingung, die das Ergebnis bestimmt. Erst „Als Verfahrensart der Akte
übernehmen“ setzt Art und Prüfverfahren der Akte.

**Bauvorlagen:** `workflow-regeln/nrw-bauvorlagen.json` ist ein NRW-Katalog
(`bundesland: "NW"`) nach BauPrüfVO, darunter Lageplan (§ 3), Bauzeichnungen
(§ 4), Baubeschreibung (§ 5), Standsicherheit (§ 8), Brandschutz (§ 9),
vereinfachtes Verfahren (§ 10), Freistellung (§ 13), Beseitigung (§ 15) und
Vorbescheid (§ 16). Profilregeln können jetzt auf Verfahrensarten (`arten`)
beschränkt werden und Dokumentkategorien (`kategorien`) nennen. Beim Anwenden
schlägt die Akte passende Projektunterlagen vor. Im Profilbereich wird der
Katalog aus den mitgelieferten Katalogen ausgewählt
(`GET /api/verfahrenskataloge`).

## Abweichungen, Ausnahmen und Befreiungen

Register **Abweichungen / Befreiungen**. Jeder Eintrag hat:

- Art (Abweichung, Ausnahme, Befreiung, Erleichterung) und Rechtsgrundlage,
  z. B. § 69 BauO NRW, § 31 Abs. 1 bzw. 2 BauGB;
- betroffene Vorschrift, Gegenstand, Soll und Ist;
- Begründung, beteiligte Nachbarn mit Zustimmung sowie zugehörige Unterlagen.

**Abgleich mit der Planung:** Unterschreitungen aus aktuellen Nachweisen
(Nutzungsmaß wie GRZ/GFZ und Abstandsflächen) erscheinen als Bedarf. Abgeglichen
wird über einen stabilen Schlüssel. Ein Abgabestand für Bauantrag, Änderung,
Nutzungsänderung, Teilbaugenehmigung oder Freistellung ist gesperrt, solange:

- eine Unterschreitung ohne Eintrag ist;
- einem Eintrag Vorschrift, Rechtsgrundlage oder Begründung fehlt;
- eine Freistellung eine planungsrechtliche Ausnahme oder Befreiung bräuchte.

Hinweise entstehen, wenn ein Nachbar nicht zugestimmt hat, eine Unterschreitung
entfallen ist oder sich Soll bzw. Ist geändert haben. Ein Eintrag, den es nicht
mehr braucht, wird als „entfallen“ markiert und nicht gelöscht.

Gespeicherte Einträge bleiben unverändert (Wortlautschutz). Die Entscheidung der
Behörde steht am Bescheid (`abweichungsentscheidungen`). Im XBau-Antrag gehen die
Einträge mit Code aus der Codeliste `abweichungart` bzw.
`erleichterung.gegenstand` mit. Fehlt die Codeliste, gibt die Abgabe einen
Hinweis.

## Unterschriften

Register **Unterschriften**. Aus der Dokumentkategorie und den Regeln des
Landespakets entsteht der Bedarf je Unterlage (§ 70 Abs. 3 BauO NRW):

- Antrag: Bauherrschaft und Entwurfsverfasser;
- Lageplan, Bauzeichnungen, Baubeschreibung, Berechnungen: Entwurfsverfasser
  (bauvorlageberechtigt nach § 67 BauO NRW);
- Statik, Gutachten, Entwässerung: Fachplaner.

Jede Unterschrift nennt Person, Rolle, Art (handschriftlich, qualifizierte
elektronische Signatur, Portal), Datum und die konkrete Dokumentversion. Eine
neue Version braucht eine neue Unterschrift. Fehlende Unterschriften stehen als
Hinweis am Abgabestand und sperren ihn nicht, weil Portale die Unterschrift oft
selbst einholen.

## Behördliche Nachrichten (XBau-Eingang)

**Unterlagen → XBau-Nachrichten → Auswerten** liest eine im Journal erfasste
Behördennachricht fachlich aus und zeigt, was übernommen würde:

| Nachricht | Wirkung in der Akte |
|---|---|
| 1120 Eingangsbestätigung | Versandereignis „Eingegangen“ an der Abgabe, Status eingereicht |
| 1121 Aktenzeichen | Aktenzeichen, wenn die Akte noch keins hat |
| 1140 Befundliste | Nachforderung mit einem Punkt je Befund und der Frist; in NRW mit Hinweis auf die Rücknahmefiktion nach § 71 BauO NRW. Ist die Dokumentation vollständig, wird das Vollständigkeitsdatum gesetzt |
| 0203/0213/0233/0263 Anhörung | Verfahrensschritt Anhörung mit Gründen und Frist |
| 0205/0215/0235/0265 Bescheid | Bescheid mit Entscheidung, Auflagen und Abweichungsentscheidungen; Status entschieden; genehmigte Unterlagen werden vorgeschlagen |
| 1160 Gebührenbescheid | Gebühreneintrag mit Betrag und Zahlungsfrist |

Übernommen wird erst nach ausdrücklicher Bestätigung. Dabei werden das
Zugangsdatum und die genehmigten Unterlagen bestätigt. Vorher läuft ein Prüflauf
gegen die Akte, damit eine abgelehnte Übernahme keine Teilstände hinterlässt.
Ist die Nachricht schon als Kommunikation erfasst, gilt deren Datum als Zugang.
Die erzeugten Einträge haben aus der Nachrichten-ID abgeleitete Kennungen, so
dass eine wiederholte Übernahme wirkungslos bleibt. Nicht ausgewertete
Nachrichten bleiben Kommunikation.

## Folgenachrichten (XBau-Ausgang)

Beim Festhalten eines Abgabestands lässt sich statt des Antrags eine
Folgenachricht wählen (`xbau_folge`):

- `baugenehmigung.stellungnahme.0204` – Stellungnahme zur Anhörung, Punkte aus deren Gründen;
- `abweichung.antrag.0220` – gesonderter Abweichungsantrag;
- `verlaengerung.antrag.0260` – Verlängerung der Geltungsdauer;
- `anzeige.baubeginn.0900` und `anzeige.nutzungsaufnahme.0910`;
- `prozessnachrichten.ruecknahme.1130` – Rücknahme.

Alle Nachrichten werden gegen das XSD XBau 2.7 geprüft. Folgenachrichten
brauchen das Aktenzeichen der Behörde. Die Abweichungssperre gilt bei
Folgenachrichten nur für den Abweichungsantrag.

## Schreiben an die Behörde

Register **Schreiben**. Erzeugt ein PDF im Briefbogen des Büros aus der Akte:

- Antwort auf eine Nachforderung (Punkte mit Antwort und Anlagen);
- Fristverlängerung (Bezug, beantragtes Datum, Grund);
- Stellungnahme zur Anhörung;
- Antrag auf Abweichung bzw. Befreiung;
- Sachstandsanfrage;
- Rücknahme.

Die Vorschau nennt Lücken, z. B. Punkte ohne Antwort. Auf Wunsch wird das PDF
als Projektunterlage in der Kategorie „Schriftverkehr“ abgelegt und kann dann
einem Abgabestand beigefügt werden. Versand und Zugang dokumentiert weiterhin
die Kommunikation.

Die Wiedervorlage einer Nachforderung in NRW weist darauf hin, dass ein
unvollständiger Antrag nach Fristablauf als zurückgenommen gilt (§ 71 BauO NRW).
Die Abgabe weist auf Nachweise hin, die seit ihrer Freigabe veraltet sind.

## Ablehnung, Rechtsbehelf und Folgeverfahren

- **Ablehnungsgründe** stehen am Bescheid: Wortlaut, Grundlage, Reaktion und
  Verweis auf eine Abweichung oder ein Folgeverfahren. Gespeicherter Wortlaut
  bleibt unverändert.
- **Rechtsbehelfsfrist:** ein Monat ab Zustellung, als Termin; für Nachbarn ab
  deren eigener Bekanntgabe (`nachbarbekanntgaben`).
- **Folgeverfahren** (`POST …/folgeverfahren`) legt z. B. einen geänderten
  Antrag, Vorbescheid oder Widerspruch mit Vorgänger und Behörde an und
  verknüpft die Gründe. Das Aktenzeichen vergibt die Behörde neu. Die erste Abgabe des Folgeverfahrens nennt die Gründe
  des Vorgängers als Hinweis, damit die Änderungen sie beantworten.

## Nach der Genehmigung

Register **Bauausführung** mit Checklisten für **Baubeginn** und
**Nutzungsaufnahme**:

- Genehmigung zugegangen (§ 74 BauO NRW) bzw. Wartefrist der
  Genehmigungsfreistellung abgelaufen (§ 63 BauO NRW);
- Geltungsdauer nicht abgelaufen (§ 75 BauO NRW);
- Bauleitung benannt, Grundrissfläche und Höhenlage abgesteckt (§ 74 BauO NRW);
- Baubeginn angezeigt bzw. Fertigstellung oder Nutzungsaufnahme angezeigt;
- bautechnische Nachweise (Standsicherheit, Brandschutz, Wärme- und
  Schallschutz) mit Prüfer und Vorlagedatum;
- Auflagen, die vor Baubeginn bzw. vor Nutzungsaufnahme fällig sind
  (`Auflage.faelligkeit`, Nebenbestimmung nach § 36 VwVfG).

Eine nach der Genehmigung geänderte Version einer genehmigten Unterlage führt
zum Hinweis auf einen Änderungsantrag (Tektur). Sind alle Voraussetzungen
erfüllt, lässt sich die Baubeginns- bzw. Nutzungsaufnahmeanzeige als
Abgabestand mit XBau-Folgenachricht festhalten.

## Verträge und Kompatibilität

Alle neuen Felder der Verfahrensakte sind optional (`serde(default)`) und werden
ohne Inhalt nicht geschrieben: `abweichungen`, `gebuehren`, `unterschriften`,
`ablehnungsgruende`, `nachbarbekanntgaben`, `bautechnische_nachweise` und
`bauvorbereitung`. Dazu kommen `Bescheid.abweichungsentscheidungen`,
`Verfahrensschritt.gruende`, `Auflage.faelligkeit`,
`Abgabewunsch.xbau_folge`, `Profilregel.arten` und `Profilregel.kategorien`.
Bestehende Akten und Kataloge bleiben lesbar; eine Datenbankmigration ist nicht
nötig. Ältere Programmstände lesen Akten ohne neue Einträge weiter. Akten mit
neuen Einträgen weisen sie wegen unbekannter Felder ab, statt sie beim Speichern
zu kürzen.

Neue HTTP-Wege (Projektzugriff wie die übrigen Verfahrenswege):

- `GET /api/verfahrenskataloge`, `POST /api/projekte/{id}/verfahrenswahl`
- `GET /api/projekte/{id}/verfahren/{v}/abweichungen`
- `GET /api/projekte/{id}/verfahren/{v}/unterschriften`
- `POST /api/projekte/{id}/verfahren/{v}/schreiben` (liefert `pdf_base64`)
- `GET /api/projekte/{id}/verfahren/{v}/optionen`
- `POST /api/projekte/{id}/verfahren/{v}/folgeverfahren`
- `GET /api/projekte/{id}/verfahren/{v}/checkliste`
- `GET|POST /api/projekte/{p}/xbau/nachrichten/{id}/auswertung`

Tests: `crates/storage/tests/{abweichungen,xbau_eingang,landesrecht,schreiben,optionen}.rs`
sowie die Moduletests in `core-lib`. Das Beispielprojekt verwendet Verfahrenswahl,
Abweichungsabgleich, Unterschriften, Lage, Antwortschreiben und XBau-Auswertung.
