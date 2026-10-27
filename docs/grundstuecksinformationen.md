# Flurstücksfakten und Baugrundstücksermittlung

## Bedienung

Unter **Grundstücksinformationen → Flurstücke und Grundbuch** werden Gemarkung,
Flur, Flurstücksnummer mit Zähler und Nenner sowie tatsächliche Flurstücksfläche
erfasst. Der Zähler hat höchstens fünf, der Nenner höchstens vier Ziffern; die
Nummer wird als „Zähler/Nenner“ gespeichert. Ältere Nummern in anderer Form
bleiben unverändert speicherbar, der Bauantrag weist auf sie hin. Die optionale
Gemarkungsnummer (höchstens vier Ziffern, ohne Landesschlüssel) übermittelt XBau
statt des Gemarkungsnamens. Grundbuchbezirk,
Blatt, laufende Nummer, Eigentümer, Bemerkung und ein Dokumentbezug auf eine
konkrete Version des Grundbuchauszugs sind optional. Eigentümer sind Kunden aus
der Kundendatenbank: bei Vorhabenflurstücken Bauherren oder Beteiligte mit der
Rolle Grundstückseigentümer bzw. Erbbauberechtigter, bei Nachbar- und sonstigen
Flurstücken beliebige Kunden. Eine Eigentümerzuordnung ändert keine Fläche; die
Baugrundstücksermittlung bleibt aktuell; das gilt auch für die Gemarkungsnummer.

Flurstücke mit der Rolle „Sonstiges“ (Zuwegung, Baulastfläche) zeigen im
Bauantrag einen Hinweis auf die Baulastklärung. Jede Verfahrensakte erhält dazu
eine ungeklärte Anforderung „Baulast für Flurstück … klären“; ändert sich die
Rolle später, nennt der nächste Abgabestand die Anforderung zur Prüfung. Auszüge werden zunächst als
Projektdatei importiert und anschließend ausgewählt. Die Flurstücke lassen sich
ohne Grundbuch und ohne fertige Baugrundstücksberechnung speichern. Im Dialog
zum Anlegen/Bearbeiten eines Projekts gibt es deshalb keine zweite Flächeneingabe.
Eine im **Aufmaß** gezeichnete Flurstücksgrenze (Flurkarte, Liegenschafts-
karte, Lageplan; Markierung „Flurstück“ mit zugeordnetem Flurstück) wird über
„Aufgaben → Übernehmen“ mit dem Flurstück verknüpft. Die gemessene Fläche steht
als *gezeichnete Fläche* neben der Katasterfläche und zeigt die Abweichung; sie
überschreibt keine Kataster- oder Baugrundstücksfläche. Liegt kein amtlicher
Wert vor, kann „Gezeichnete Fläche als Flurstücksfläche verwenden“ gewählt
werden; dann setzt der Core die Flurstücksfläche aus der Zeichnung und das Feld
ist nicht von Hand änderbar. Eine geänderte Kontur im Aufmaß wird nicht still
übernommen: Speichern und Projektübersicht verlangen „Aktuellen Aufmaßstand
übernehmen“. „Verknüpfung lösen“ behält den Wert als manuelle Angabe.

Jedes Flurstück hat einen **Bezug zum Vorhaben**: Vorhabengrundstück (Normalfall),
Nachbarflurstück oder Sonstiges (etwa Zuwegung oder Baulastfläche). Nur
Vorhabenflurstücke bilden Baugrundstücke, stehen im Bauantrag und in der XBau-Abgabe
und ergeben die Grundstücksfläche des Projekts (Summe ihrer Flurstücksflächen).
Die übrigen bleiben als Fakten erfasst, etwa für Abstandsflächen. Ein
Buchgrundstück gehört nur ganz zum Vorhaben, und mindestens ein Flurstück muss
dazugehören. Ein Wechsel des Bezugs ändert den Quellenstand; die
Baugrundstücksermittlung ist danach erneut zu bestätigen. Ohne erfasste
Flurstücke bleibt die Flächenangabe eines Bestandsprojekts stehen.

Unter **Berechnungen → Baugrundstück** stehen diese gespeicherten Fakten lesend
bereit. Dort werden die Grundstückszuordnung, einzelne begründete Flächenabzüge
und zugeordnete Gemeinschaftsanteile angesetzt. Die Flurstücksfläche wird dabei
nicht verändert. GRZ, GFZ und Formulare erhalten die maßgebenden Bezugsflächen je
Baugrundstück, niemals eine stillschweigend zusammengezählte Katasterfläche.

Grundbuchangaben sind auch für die Berechnung optional. Ohne vollständige
Grundbuchzuordnung wird jedes Flurstück als eigene Berechnungseinheit angesetzt.
Ein einzelnes Flurstück kann damit unmittelbar berechnet werden. Mehrere
Flurstücke werden niemals allein wegen gleicher fehlender Angaben summiert.
Vollständige Angaben mit gleichem Bezirk, Blatt und laufender Nummer bilden eine
gemeinsame Einheit; unterschiedliche Zuordnungen ergeben getrennte Einheiten.
Eine ausdrückliche gemeinsame Zuordnung ist über dieselbe besondere Kennung mit
Grundlage möglich. Die Grundbuchzuordnung beeinflusst damit die Gruppierung und
die Anzahl der nachgelagerten Nutzungsberechnungen, nicht die grundsätzliche
Berechenbarkeit. Ein einzelner Ansatz ohne Grundbuch ist keine Bestätigung einer
grundbuchrechtlichen Einheit.

Nach Änderungen an Grundstücksinformationen muss die Ermittlung erneut geprüft
und mit **Eingaben speichern** bestätigt werden. Bis dahin wird keine alte
Bezugsfläche automatisch in Formulare übernommen. Bereits archivierte Nachweise
bleiben mit ihren ursprünglichen Daten nachrechenbar. Ein offener, abweichender
Berechnungsentwurf bleibt bei Konflikten erhalten; das Laden aktueller Quellen
mit Verwerfen des Entwurfs ist eine ausdrücklich beschriftete Aktion.

## Modulverträge und Speicherung

- `grundstuecksinformationen::Grundstuecksinformationen` besitzt die Fakten und
  validiert sie unabhängig von Planfestsetzungen und Nutzungsnachweisen.
- `FlurstuecksInfo.planbindung`, `planflaeche_m2` und `flaeche_aus_plan`
  (alle optional, nicht geschrieben solange leer) tragen die Herkunft einer
  gezeichneten Fläche aus dem Aufmaß. Die Werte setzt ausschließlich
  `CoreServices::planaufmass_flurstuecke`; die Validierung weist von Hand
  eingetragene oder abweichende Werte zurück. `flaeche_m2` bleibt der
  Rechenwert; der Quellen-Fingerprint der Ermittlung ist unverändert.
- `FlaechenAbzug.id`/`planbindung` und `flaechen::Bauteil.id`/`planbindung`
  (optional, nicht geschrieben solange leer) tragen Zeilenkennung und Herkunft
  einer aus dem Aufmaß übernommenen Fläche (Ziele `abzuege.<id>.flaeche_m2`
  bzw. `bauteile.<id>.flaeche_m2`). Die Werte setzen ausschließlich
  `CoreServices::planaufmass_abzuege` und `planaufmass_bauteile`; Abzugsart,
  rechtliche Grundlage, Bauteiltyp und Anrechnung bleiben Eingaben und Regeln
  dieses Moduls. Beim Speichern (Projektgrenze, Einzelspeicherung eines
  Baugrundstücks, Nutzungsnachweise) müssen gebundene Werte dem Stand ihrer
  Quelle entsprechen (`grundstueck_planquellen_pruefen`). Altdaten ohne
  Kennung bleiben gültig, können aber nicht gebunden werden.
- `BaugrundstuecksErmittlung` speichert nur flurstücksbezogene Ansätze,
  Nutzungsberechnungseingaben und den vom Core gesetzten Quellenhash. Kataster-
  und Grundbuchdaten werden nicht als zweite Faktenkopie darin gespeichert.
- Die beiden Felder liegen getrennt im bestehenden Projekt-JSON. Die
  Schreibbereiche `grundstuecksinformationen` und `grundstuecke` verwenden den
  Drei-Wege-Abgleich und übernehmen ausschließlich ihre verantworteten Felder.
- `grundstuecksinformationen::plan` verbindet Fakten und Ansätze zu dem bestehenden
  `BaugrundstuecksPlan` für Berechnungsaufrufe und unveränderliche Archive.
  Die bestehende Ergebnisliste der Baugrundstücksermittlung bleibt erhalten.
- `grundstueckskontext::ermitteln` prüft den Quellenstand und liefert weiterhin
  getrennte Einheiten, Identitäten und Bezugsflächen an Formulare und andere
  Verbraucher. Eine alte Projektgesamtfläche ersetzt keine unbestätigte Ermittlung.
- Berechnungsaufrufe und neue Nachweisstände prüfen, dass ihre Flurstücksfakten
  mit den gespeicherten Projektquellen übereinstimmen. Neue Nutzungsnachweise
  verlangen zusätzlich die bestätigte Zuordnung mit ihren gespeicherten Abzügen
  und Gemeinschaftsanteilen. Abweichende Flächen können
  nicht im Berechnungsdialog an der Faktenverwaltung vorbei verwendet werden.
- Neue Frontendtypen für Fakten und Ansätze werden aus Rust generiert. Der
  Frontendadapter verbindet ausschließlich Daten; Gruppierung, Anrechnung und
  Flächenberechnung bleiben im Core.

## Bestandsdaten

Das bisherige Feld `baugrundstuecke` bleibt als lesbarer Bestandsvertrag erhalten.
Bei der nächsten tatsächlichen Bereichsspeicherung übernimmt der Core daraus
Fakten und Berechnungsansätze und entfernt die bisherige Live-Doppelstruktur.
Flurstücksidentitäten, Abzüge, besondere Zuordnungen und Nutzungseingaben bleiben
erhalten. Eine reine Formatumstellung ändert keine Grundstücksidentitäten oder
Nachweisaktualität. Änderungen an den Quellen erfordern dagegen eine erneute
Bestätigung. Historische Nachweise werden nicht umgeschrieben. Es ist keine
SQL-Migration erforderlich.

## Validierung

Core-Tests prüfen optionale Grundbuchdaten, fehlende Zuordnungen, getrennte
Grundbuchblätter und verlustfreie Formatübernahme. SQLite-/Storage-Integration
prüft unabhängige Speicherbereiche, Quellenkonflikte, Faktenfläche gegenüber
Bezugsfläche, GRZ-/Formularübergabe und historische Nachweiswiederholung.

Die Browserregression prüft den neuen Ablauf ohne Grundbuchpflicht, Fehlererhalt
beim Speichern, die lesende Faktenansicht in der Berechnung, begründete Zuordnung,
getrennte Speicherpayloads, Projektwechsel und schmale Fenster. Die Screenshots
verwenden Testdaten und einen Testtransport; sie ersetzen keine native Abnahme.

![Flurstücksinformationen](grundstuecksinformationen/flurstuecke.png)

![Baugrundstücksermittlung](grundstuecksinformationen/baugrundstueck.png)
