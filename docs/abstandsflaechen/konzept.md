# Ausbauplan: prüfbare Abstandsflächennachweise

Status: Fachlicher Ausbauplan. Implementierten Funktionsumfang, manuelle
Sondernachweise und ausstehende externe Rechts-/Praxisabnahmen beschreibt die
[Bedienungsdokumentation](README.md).
Technischer Ausgangsstand: Main `a5c6fbb`. Nordrhein-Westfalen wird als erstes
Pilotland vorgeschlagen, entsprechend dem zuletzt gezeigten Projekt.

## 1. Ziel und fachliche Grenze

Die Anwendung soll ein Vorhaben wandweise erfassen, die abstandsflächenrechtlich
relevanten Höhen herleiten, erforderliche Flächen geometrisch darstellen und
Konflikte nachvollziehbar dokumentieren. Sie muss einen prüffähigen Nachweis
liefern, der sich mit seinen damaligen Grundlagen reproduzieren lässt.

„Rechtssicher“ ist keine Eigenschaft einer einzigen Formel. Verlässlichkeit
entsteht aus passender Gesetzesfassung, geprüften örtlichen Regeln, belastbaren
Eingaben, transparenten Annahmen, fachlicher Kontrolle und dokumentierten
behördlichen Entscheidungen. Die Software darf keine Baugenehmigung oder
rechtliche Zulässigkeit allein aus einem rechnerischen Ergebnis ableiten.

Zwei klar bezeichnete Arbeitsweisen vorsehen:

- **Tiefenberechnung / Vorprüfung:** manuelle Höhen und bestätigter Rechtsansatz,
  ohne vollständige Grundstücksgeometrie nutzbar. Ergebnis ist eine erforderliche
  Tiefe, keine Aussage über deren Einhaltung auf dem Grundstück.
- **Projektbezogener Nachweis:** alle relevanten Wände, Gelände-, Dach- und
  Grenzverhältnisse, Sicherungen und Sonderfälle sowie deren Prüfstand.

Entwürfe und unvollständige Berechnungen bleiben speicher- und druckbar. Ihre
Ausgaben kennzeichnen fehlende Prüfungen. Eine vollständige fachliche Freigabe
ist bei offenen notwendigen Grundlagen ausgeschlossen.

## 2. Bestand und konkrete Lücken

Vorhanden sind:

- `core-lib::lbo`: Rhai-Regelpakete mit Metadaten und einfachen Tiefenformeln.
- `AbstandsflaecheInput`: nur `wandhoehe_m` und `gebiet`.
- `Abstandsflaeche.svelte`: eine Höhe, ein Baugebiet, ein Ergebnis; keine
  Gebäudewandliste und kein zugehöriger Grundstücksplan.
- `plattform::nachweise`: Eingabe-/Regelarchiv, historische Nachrechnung,
  Aktualitätsprüfung und fachliche Freigabe.
- `druckausgabe` und `verfahren`: Druckberichte, DMS-Versionen,
  Einreichungsstände und Bescheidbezüge.
- `cad::baugrenzen`: Einheiten, Geometrievalidierung und Polygonoperationen.

Die Pakete für NRW, Bayern und Baden-Württemberg sind ausdrücklich beispielhaft.
Sie berücksichtigen nicht alle Sonderfälle und sind kein geprüfter Länderbestand.
Die vorhandene Höhe wird als H behandelt; eine getrennte Herleitung aus Wand,
Dach/Giebel und Gelände fehlt. Das aktuelle zweistellige Aufrunden darf nicht
ungeprüft als allgemeine rechtliche Rundungsregel fortgeschrieben werden.

Zusätzlich bewertet der Nachweispfad den einfachen Abstandsansatz derzeit
technisch als `vollstaendig = true`. Beim Ausbau muss Vollständigkeit ausdrücklich
zum Leistungsumfang gehören: vollständige Tiefenberechnung ist kein vollständiger
Abstandsflächennachweis.

## 3. Welche Rechtsgrundlagen festgehalten werden

Ein eingefrorener Rechtskontext enthält mindestens:

| Angabe | Verwendung |
| --- | --- |
| Bundesland und konkrete LBO-Fassung | maßgebendes Landesrecht |
| Gültigkeitszeitraum und Übergangsvorschriften | rechtlich einschlägige Fassung ausdrücklich bestimmen |
| Gemeinde, ggf. Geltungsbereich und Satzung | örtlich abweichende Vorschriften prüfen |
| Bebauungsplan mit einschlägiger Festsetzung | Einfluss auf Grenzbebauung und Abstände prüfen |
| Baugebiet sowie tatsächliche/rechtliche Grundlage | begründeter Ansatz statt ungeprüfter Auswahl |
| Vorhabensart und Bestands-/Änderungssituation | sachlichen Prüfbereich festlegen |
| Regelpaket, Inhaltshash, Quelle, Prüfer und Freigabe | nachvollziehbare Regelanwendung |

Das Antragsdatum allein entscheidet nicht automatisch über das anzuwendende
Recht. Übergangsrecht und Bestandssituation brauchen eine nachvollziehbare
Entscheidung. Ebenso bestimmt nicht das BauNVO-Jahr allein die Abstandsfläche.

Örtliche Satzungen und Planfestsetzungen werden als eigene versionierte
Regelergänzungen geführt. Eine Satzung ersetzt nicht pauschal die ganze LBO.
Ihr Anwendungsbereich und die jeweils betroffene Regel müssen geprüft werden.
Widersprüche zwischen Regelquellen führen zu einem offenen Prüffall.

Amtliche Prüfreferenzen vor Umsetzung eines Länderpakets:

- NRW: [Recht.NRW](https://recht.nrw.de/), BauO NRW 2018, insbesondere § 6,
  einschließlich einschlägiger Übergangs-, Satzungs- und Verfahrensbestimmungen.
- Bayern: [BayBO Art. 6](https://www.gesetze-bayern.de/Content/Document/BayBO-6).
- Baden-Württemberg: [Landesrecht BW](https://www.landesrecht-bw.de/), LBO,
  insbesondere §§ 5 und 6.
- Je Projekt zusätzlich amtlicher Bebauungsplan, örtliche Satzungen,
  Sicherungsunterlagen und konkrete Behördenentscheidungen.

Die amtlichen Portale waren während dieser Planung aus der Cloud nicht abrufbar
(HTTP-Proxy 403). Daher enthält dieser Plan bewusst keinen als aktuell verifiziert
behaupteten vollständigen Zahlen-/Ausnahmekatalog. Die bisherigen Beispielwerte
sind vor fachlicher Freigabe anhand der dann einschlägigen Originalfassungen
zu prüfen. Auslegungshinweise und Rechtsprechung benötigen eigene Quellen und
klar dokumentierte, fachlich geprüfte Interpretationen.

## 4. Datenmodell für den Planungsalltag

### Grundstücke und Grenzabschnitte

Das Grundstücksmodul liefert stabile Identitäten, zugehörige Flurstücke und
maßgebliche Grenzen. Das Abstandsmodul ergänzt einen eigenen fachlichen Bezug,
falls das einschlägige Bauordnungsrecht eine andere Grundstückseinheit verlangt.
Dieser Bezug wird mit seiner Herleitung dokumentiert; keine heimliche neue
Gruppierung oder Summe von Flurstücksflächen.

**Die für GRZ/GFZ berechnete Bezugsfläche ist keine Abstandsflächengeometrie.**
Abzüge wie private Grünflächen verkleinern nicht automatisch die maßgebliche
Grundstücksgrenze. Umgekehrt sind ein Baugrundstücksflächenwert oder eine
Grundbuchnummer kein geometrischer Grenznachweis.

Grenzabschnitte unterscheiden Nachbargrundstück, öffentliche Fläche,
interne Flurstücksgrenze und ungeklärten Verlauf. Ob eine interne Grenze entfällt,
benötigt die passende Grundstückszuordnung. Bei öffentlichen Flächen darf die
Nutzung bis zur Mitte nur angewandt werden, soweit die konkrete Vorschrift und
Situation dies zulassen; dafür ist die maßgebliche Flächengeometrie erforderlich.

Grundbuchnummern und Auszugsdateien werden keine allgemeine Berechnungspflicht.
Fehlen sie, bleibt eine Vorprüfung möglich. Soweit eine gemeinsame Einheit oder
Fremdflächenanrechnung rechtlich noch nicht belegt ist, bleibt genau diese
Einordnungsfrage offen. Falsche Zusammenfassungen werden nicht automatisch erzeugt.

### Gebäude, Wände und Gelände

Ein Projekt kann mehrere Gebäude und Bauabschnitte enthalten. Jede relevante
Außenwand bzw. jeder Wandabschnitt erhält eine feste ID, Bezeichnung, Lage,
Länge und nach außen gerichtete Seite. Gebäudeecken und verspringende Fassaden
sind eigene geometrische Verhältnisse; kein einzelnes pauschales H je Gebäude.

Zu jedem Wandabschnitt erfassen:

- Geländehöhen mit Verlauf, Quelle und Höhenbezug; vorhandenes und geplantes
  Gelände getrennt. Welche Geländeoberfläche maßgebend ist, wird rechtlich
  entschieden. Ein geplanter Geländeauftrag darf den Ansatz nicht ungeprüft ändern.
- Wandabschluss, Dachanschnitt, Dachform, Dachneigung, Giebel-/Dachanteile und
  sonstige relevante Aufbauten als Ausgangsgeometrie.
- Bestands-/Neubaustatus, ggf. betroffene Teiländerung, Öffnungen und besondere
  Nutzung, soweit diese für eine Regelentscheidung benötigt werden.
- Datenherkunft: Plan-Dokument mit Version, Vermessung, manuelle Erfassung oder
  bestätigte Übernahme. Alle Höhen verwenden denselben festgelegten Bezug.

Das Regelpaket leitet H und gegebenenfalls abschnittsweise unterschiedliche
Ansätze her. Mittelung, Dachanteile und Abschnittsbildung nur dann verwenden,
wenn die konkrete Regel dies erlaubt. Ein manuell übernommenes H bleibt als
fachlich bestätigter Eingabewert mit Quelle erkennbar.

### Bauteile und besondere Fälle

Separat modellieren: Balkone, Erker, Dachüberstände, Außentreppen, Gauben,
Dachaufbauten, Garagen, Carports, Nebenanlagen und vergleichbare Anlagen.

Eine mögliche Privilegierung wird aus allen einschlägigen Bedingungen geprüft:
z.B. Höhe, Länge, Grenzlage, Nutzung und grundstücksbezogene Summen. Nicht allein
das Etikett „Garage“ oder „Balkon“ entscheidet. Bedingungen und Grenzwerte werden
länder- und fassungsbezogen konfiguriert; fehlende Daten führen zu „offen“.

Weitere Prüffallgruppen: zulässige/gebotene Grenzbebauung, Überdeckungen,
Winkel-/Ecksituationen, bestehende Bebauung, Nutzungsänderungen, Anbauten,
Sonderbauten sowie ggf. genehmigte Abweichungen. Nicht jeder Bestand genießt
pauschal eine abstandsflächenrechtliche Ausnahme.

### Sicherungen und Entscheidungen

Sicherungsflächen sind eigene Polygone mit begünstigter/belasteter Einheit,
Umfang, Wirksamkeitsstatus, Originaldokument und geprüftem Verwendungszweck.
Nachbarzustimmung, privatrechtliche Vereinbarung, Dienstbarkeit, Baulast und
behördliche Abweichung sind unterschiedliche Sachverhalte. Ihre Wirkung wird
landesbezogen geprüft. Ein hochgeladenes Dokument ist noch keine wirksame Sicherung.

Eine beantragte Abweichung bleibt offen. Erst eine geeignete Entscheidung mit
passendem Geltungsumfang wird als solche berücksichtigt. Sonderentscheidungen
ändern nicht rückwirkend die allgemeine Formel oder frühere Nachweisstände.

## 5. Unabhängige technische Bausteine

```text
Rechtskontext + Wand-/Dach-/Geländedaten
                    ↓
           Abstands-Regelauswertung
      H-Herleitung, Tiefe, Prüfschritte
                    ↓
        Abstandsflächen-Geometrie
                    ↓
Grenzen + Sicherungsflächen + übrige Baukörper
                    ↓
      Konflikt- und Vollständigkeitsprüfung
                    ↓
        Nachweisstand / PDF / Verfahrensakte
```

Neues Fachmodul `core-lib::abstandsflaechen` mit getrennten Verantwortlichkeiten:

1. `modell`: versionierte Eingaben, feste IDs und Quellenbezüge.
2. `regeln`: Rechtsanwendung, Höhenherleitung, Privilegierungen, Ausnahmen.
3. `geometrie`: robuste Polygonbildung, Verschnitt und Abstandsbestimmung;
   unabhängig vom Bundesland und ohne eingebauten Landesfaktor.
4. `pruefung`: Zusammenführen von Geometrie, Rechtskontext, Sicherungen und
   offenen Punkten; begründete Zustände statt pauschalem Boolean.
5. Nachweis-/Exportadapter nutzen vorhandene Archiv-, Druck- und DMS-Dienste.

Parameter, Bedingungen und Entscheidungstabellen möglichst in validierten,
versionierten Konfigurationsdateien führen. Komplexe rechtliche Auswertungen
können die vorhandene sandboxierte Rhai-Engine verwenden. Eigene Textfelder mit
beliebigen Formeln für Nutzer sind keine tragfähige Regelverwaltung.
Ein Regelpaket deklariert die unterstützten Fälle; unbekannte Fälle erhalten
keinen stillen Standardfaktor. Gesetzesquelle, Implementierungsstand und
fachlicher Freigabestand sind getrennte Metadaten.

Der vorhandene Vertrag `NachweisEingabe::Abstand` bleibt für historische einfache
Berechnungen lesbar. Ein neuer versionierter Vertrag für den vollständigen
Nachweis ersetzt ihn nicht stillschweigend. Alte Stände bleiben Tiefenberechnungen;
sie werden nicht nachträglich als umfassend geprüft dargestellt.

## 6. Geometrie und Ergebnisvertrag

Zunächst manuelle Wand-/Grenzerfassung und tabellarische Koordinateneingabe
unterstützen; CAD darf keine Voraussetzung sein. Anschließend DXF-Übernahme mit
Bestätigung von Maßstab, Einheit, Lage, Referenzsystem und Höhenbezug.
PDF-Zeichnungen dürfen als kalibrierte Erfassungshilfe dienen, jedoch nicht als
ungeprüfte Vermessungsgrundlage. DWG benötigt einen ausdrücklich unterstützten
Import-/Konvertierungsweg; die bestehende Dateiverwaltung ist noch kein DWG-Parser.

Keine pauschale Pufferung des gesamten Gebäudegrundrisses. Die Geometrie wird aus
Wandabschnitten, Außenseiten und ihren jeweiligen Tiefen gebildet. Unterschiedliche
Höhen, Innenhöfe, Versprünge und Dachanteile sind zu berücksichtigen. Welche
Eckverbindungen und Überdeckungen rechtlich zulässig sind, bestimmt die geprüfte
Regelauswertung. Unklare Höhenverläufe nicht eigenmächtig linear interpolieren.

Ergebnis je Wand/Abschnitt:

- Eingangsmaße und vollständige H-Herleitung, angewandte Regelbedingungen,
  erforderliche Tiefe und Mindesttiefe mit Rechtsquellen;
- erzeugtes Polygon und maßgebliche Grundstücks-/Sicherungsflächen;
- soweit sinnvoll definierbar: kleinster vorhandener Abstand, maßgebende Stelle,
  maximale lokale Tiefenunterschreitung und Konfliktfläche in m²;
- Konfliktpolygone statt nur eines Prozentwerts oder eines Eckpunktabstands;
- offene Angaben, verwendete Annahmen, Sonderentscheidungen und Prüfergebnisse.

Bei schrägen Grenzen ist die Einhaltung entlang der relevanten Wand zu prüfen.
Die aktuelle Baugrenzenfunktion liefert nur einen ausdrücklich so bezeichneten
Eckpunktüberstand; dieser ist kein Ersatz für eine vollständige Abstandsprüfung.

Einheiten und Rechengenauigkeit sind festgelegt. Angezeigte Rundung,
rechtlich vorgeschriebene Rundung und numerische Geometrietoleranz werden getrennt.
Toleranzen dürfen eine materielle Unterschreitung nicht automatisch zulässig machen.
Unsichere Messdaten und grenznahe Ergebnisse werden zur Prüfung markiert.

## 7. Status, Freigabe und Bedienung

Drei unabhängige Statusachsen vorsehen:

| Achse | Beispiele |
| --- | --- |
| Fachprüfung | offen / geometrisch eingehalten / Konflikt / durch konkrete Entscheidung gedeckt |
| Aktualität | aktuell / veraltet |
| Bürofreigabe | Entwurf / fachlich geprüft und freigegeben |

„Geometrisch eingehalten“ setzt vollständige für diese Aussage notwendige Angaben
voraus und bestätigt keine gesamte baurechtliche Zulässigkeit. Abweichungsanträge
und fehlende Sicherungen bleiben offene Prüfpunkte. Teilweise vollständige Wände
können bereits Ergebnisse zeigen, ohne das Gesamtvorhaben als vollständig zu werten.

UI im bestehenden Bereich **Berechnungen → Abstandsflächen**:

1. **Grundlagen:** Landesrecht, örtliche Regeln, Grundstücks- und Planbezug.
2. **Wände / Bauteile:** tabellarische Eingabe mit Querschnitt/Höhenskizze.
3. **Lageplan:** Wandnummern, Grenzen, Abstandsflächen und Konflikte; Auswahl
   einer Tabellenzeile hebt dieselbe Wand im Plan hervor.
4. **Sonderfälle / Sicherungen:** geführte fachliche Fragen und Dokumentbelege.
5. **Prüfbericht:** offene Punkte, begründete Ergebnisse, Fachprüfung und Ausgabe.

Ein Grundstücksplan mit Fehlern darf eine unabhängige Tiefenvorprüfung nicht
sperren. Fachwerte werden allein im Core berechnet. Ungeklärte Angaben werden
nicht mit Null oder „Nein“ vorbelegt. Fehlermeldungen nennen Wand, fehlende Angabe
und nächsten Arbeitsschritt. Die Eingaben bleiben bei Fehlern und Tabwechseln erhalten.

## 8. Nachweis, Dokumentation und Verfahren

PDF-Nachweis mit Deckblatt, Rechts- und Datenstand, Quellenliste, nummerierter
Wandübersicht, maßstäblich nachvollziehbarer Planskizze, Höhenherleitungen,
Rechenschritten, Ergebnissen und offenen Punkten. Druckmaßstab nur dann als
maßstäblich zusagen, wenn Ausgabeformat und Druckeinstellungen das gewährleisten;
Maßketten und grafischer Maßstab bleiben enthalten. Keine bloße Screenshot-Ausgabe.

Nachweise speichern Geometrie, Eingaben, Quellversionen, Regelinhalt samt Hash,
Algorithmusversion und maßgebliche Entscheidungen. Regelinhalt allein genügt
nicht zur Reproduzierbarkeit, wenn die Geometrie-Engine später geändert wird.
Ein Nachweis wird unveränderlich abgelegt und in die bestehenden Abgabestände
übernommen. Spätere Änderungen verändern weder eingereichte PDFs noch genehmigte
Versionen. Betroffene neue Nachweise müssen neu berechnet/geprüft werden.

Der Aktualitätsfingerprint berücksichtigt nur relevante Abhängigkeiten, darunter
Wände, Gelände, Grenzen, Sicherungen, Satzung und Quellen-/Regelstände. Eine
GRZ-Eingabeänderung allein darf einen unabhängigen Abstandsflächennachweis nicht
sachlich verändern. Die heutige breite Nachweisgrundlage ist dafür zu präzisieren.

Offene Konflikte lassen sich als Aufgaben/Nachforderungen verknüpfen.
Abweichungsentscheidungen und Nebenbestimmungen werden über die Verfahrensakte
referenziert, ohne deren Fachlogik oder Dokumente im Rechenmodul zu duplizieren.

## 9. Umsetzungsreihenfolge mit Abnahme

### A. Fachlicher NRW-Pilot und tragfähige Verträge

Fallkatalog und konkrete Landesfassung festlegen, amtliche Quellen prüfen und
Regeltabellen fachlich abnehmen. Leistungsumfang und nicht unterstützte Fälle
benennen. Neue Vertragsversion, Teilstatus und Archivstrategie einführen.
Den bestehenden Rechner ausdrücklich als Tiefenvorprüfung kennzeichnen.

Abnahme: Jede umgesetzte Regel hat eine Quelle, alle Bedingungen und mindestens
einen unabhängigen Kontrollfall. Fehlende Regeln oder Grundlagen sind sichtbar.

### B. Wandweiser Rechner ohne CAD-Zwang

Mehrere Wände, nachvollziehbare H-Herleitung aus Gelände/Wand/Dach,
Nutzungskontext, Tabellenansicht und vollständiger Rechenausdruck. Zunächst
fachlich festgelegte einfache Gebäudeformen; ununterstützte Formen bleiben offen.

Abnahme: Manuell unabhängig gerechnete Standardfälle stimmen überein. Einfache
Vorprüfungen funktionieren auch ohne Grundbuchdatei oder fertigen Lageplan.

### C. Grundstücksgeometrie und Konfliktplan

Relevante Grundstückseinheiten, Grenztypen, Wandgeometrie, Polygone,
Überschreitungen und gegenseitige Überdeckungen verbinden; DXF-Adapter ergänzen.

Abnahme: Numerische Grenzfälle und bekannte Referenzpläne werden geometrisch
richtig dargestellt. Nicht nur Eckpunkte, sondern maßgebliche Kantenverläufe
werden geprüft. Falsche Einheiten und ungültige Geometrien werden abgewiesen.

### D. Sonderfälle und rechtliche Sicherungen

Priorisierte Dach-/Bauteilfälle, Grenzanlagen und kumulierte Längen,
öffentliche Flächen, Sicherungen und genehmigte Abweichungen ergänzen.
Jeder Fall durchläuft dieselbe Quellen- und Testabnahme. Eine regulär behandelte
NRW-Fallgruppe ist kein Nachweis für dieselbe Rechtsfolge in einem anderen Land.

### E. Büroabnahme und weitere Länder

Vollständigen realitätsnahen Musterantrag von Erfassung über Nachforderung und
Korrektur bis Genehmigung prüfen. Eigenständige fachliche und geometrische
Vier-Augen-Abnahme, dokumentierte Regression und erst dann Freigabe des Piloten.
Weitere Länder als getrennte, versionierte und geprüfte Regelpakete ausrollen.

Keine belastbare Zeit- oder Kostenfestlegung vor dem Fallkatalog: Der Umfang der
Sonderfälle und der Anspruch an CAD/Geometrie bestimmen den Hauptaufwand.

## 10. Verbindlicher Testkatalog

- Unter-/Überschreitung des Mindesttiefenansatzes und exakt am Grenzwert;
  dimensionsrichtige Rechnung und rechtssichere Trennung der Rundungsarten.
- Unterschiedliche Baugebiete sowie fehlende oder widersprüchliche Einordnung.
- Ebene und geneigte Geländeprofile, wechselnde Wandhöhen und getrennte
  vorhandene/geplante Geländemodelle.
- Flach-, Pult-, Sattel- und Walmdach, Giebel, Staffelung und Aufbauten jeweils
  nur mit fachlich freigegebenen Auswertungsregeln.
- Kurze/verspringende Wände, schräge Grenzen, konkave Grundstücke, Innenhöfe,
  mehrere Baukörper, Überdeckungen und öffentliche Flächen.
- Privilegierte Anlage erfüllt sämtliche Bedingungen, scheitert an einer
  Bedingung oder überschreitet erst gemeinsam mit anderen Anlagen eine Summe.
- Unbekannte Grenzen, noch nicht wirksame Sicherungen und bloß beantragte
  Abweichungen erzeugen keinen positiven Gesamtnachweis.
- Zwei Grundstückseinheiten werden nicht zusammengezählt. GRZ-Flächenabzug
  verändert nicht automatisch die Grundstücksgeometrie der Abstandsprüfung.
- Skalierung/Einheiten, Translation und Drehung verändern die fachlich gleichen
  Ergebnisse nicht. Ungültige Ringe, fehlender Höhenbezug und Selbstüberschneidungen
  werden gezielt abgefangen. Kontrollplan plus unabhängige Rechnung verwenden.
- Regel-, Gelände-, Grenz- oder Planänderung macht genau betroffene Ergebnisse
  veraltet; archivierte PDFs und historische Berechnungen bleiben unverändert.
- Rollen, gleichzeitige Bearbeitung, Entwurfserhalt und Hintergrund-Projekttabs.

Für jeden Referenzfall speichern: Quelle, Skizze, Eingaben, unabhängiger Rechenweg,
Sollgeometrie, Toleranzbegründung, erwartete offenen Punkte und fachliche Abnahme.
Tests, die nur dieselbe Implementierung nachrechnen, gelten nicht als unabhängige
Bestätigung der fachlichen Regel.
