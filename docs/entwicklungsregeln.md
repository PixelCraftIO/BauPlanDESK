# Entwicklungsregeln: eigenständige Fachmodule und stabile Schnittstellen

Diese Regeln gelten für neue Features, fachliche Erweiterungen und Überarbeitungen
von BauPlanIO. Ziel ist, jedes Fachmodul unabhängig weiterzuentwickeln und seine
Ergebnisse zuverlässig in die bestehende Anwendung einzubinden.

**Leitregel:** Ein Modul besitzt seine Eingaben, Fachlogik und internen Details.
Andere Module verwenden seinen dokumentierten öffentlichen Vertrag. Eine neue
interne Berechnung darf bestehende Verbraucher nicht zu einer Überarbeitung
zwingen, solange sich deren benötigte Daten und deren Bedeutung nicht ändern.

Das ist ein modularer Aufbau innerhalb der vorhandenen Anwendung. Er setzt keine
separaten Dienste, Prozesse oder unabhängig ausgelieferten Anwendungen voraus.

## 1. Verantwortung eines Moduls

Jedes Fachmodul muss eindeutig festlegen:

- Welche fachliche Aufgabe es erfüllt und welche Daten es selbst verantwortet.
- Welche Eingaben es benötigt, mit welchen Einheiten und Validierungsregeln.
- Welche Ergebnisse es anderen Modulen bereitstellt.
- Welche anderen Fachmodule es dazu benötigt.
- Wie fehlende Daten, Fehler, Warnungen und veraltete Ergebnisse behandelt werden.

Interne Hilfsfunktionen, UI-Zustände, Datenbanktabellen und Details einer
verwendeten Bibliothek gehören nicht automatisch zum öffentlichen Vertrag.
Öffentliche Schnittstellen sind klein, typisiert und fachlich benannt.

Ein neues Feature dokumentiert diesen Vertrag beim Modul. Umfangreiche Änderungen
halten die Entscheidung und ihre Auswirkungen in einer kurzen Architektur-Notiz
fest. Eine eigene Crate pro Feature ist nur nötig, wenn die Modulgrenze dies
rechtfertigt; eine klar abgegrenzte Rust-Modulstruktur genügt häufig.

## 2. Bestehende Architektur verwenden

- Gemeinsame Geschäftslogik liegt in `crates/core-lib`. Sie hängt nicht von Svelte,
  Tauri, axum, sqlx oder konkreten Ablage-Backends ab.
- Reine Berechnungen werden von Speicherung, Transport und Benutzeroberfläche
  getrennt. Sie lassen sich mit fachlichen Eingaben ohne geöffnetes Fenster testen.
- `CoreServices` koordiniert Fachmodule, Berechtigungen und Persistenz. Desktop-
  Commands und Server-Handler bleiben dünne Adapter zum selben fachlichen Vertrag.
- Datenbanken und Ablagen implementieren die vorhandenen Repository-/Storage-
  Schnittstellen. Verbraucher greifen nicht direkt in die Tabellen eines anderen
  Moduls oder auf dessen interne Dateistruktur zu.
- Das Frontend erfasst Eingaben, zeigt Ergebnisse und steuert Arbeitsabläufe.
  Verbindliche Fachberechnungen und rechtliche Anrechnungsregeln werden nicht
  zusätzlich in Svelte nachimplementiert. Vorschauen müssen als solche erkennbar
  sein; für gespeicherte und weitergegebene Ergebnisse ist der Core maßgebend.
- Fachlich veränderliche Regeln, BauNVO-Fassungen und Formularzuordnungen werden
  möglichst in den bestehenden versionierten Konfigurations-/Regelpaketen geführt.
  Neue Regeln benötigen Validierung und fachliche Testfälle.

## 3. Abhängigkeiten über Ergebnisse und Verträge

Ein Verbraucher erhält nur die Daten, die er benötigt. Er ruft eine öffentliche
Funktion, einen Service oder eine abstrahierte Schnittstelle des Liefermoduls auf.
Er kennt dessen konkrete Berechnungsschritte nicht.

Die Abhängigkeiten verlaufen gerichtet. Zirkuläre Abhängigkeiten zwischen
Fachmodulen sind zu vermeiden. Gemeinsame Abläufe werden im Service koordiniert;
gemeinsame Typen enthalten nur tatsächlich gemeinsame fachliche Begriffe.
Ein allgemeines „Hilfsmodul“ darf nicht sämtliche Feature-Interna aufnehmen.

Beispiel für den gewünschten Datenfluss:

```text
Flurstücke + Zuordnungen + Planfestsetzungen
                     ↓
          Baugrundstücksermittlung
                     ↓
     typisierte Liste von Baugrundstücken
              ↓                 ↓
      GRZ / GFZ / BMZ      Projektkontext für
      je Grundstück       Formulare / Bauantrag
```

Die nachgelagerten Module dürfen eigene fachliche Ansätze haben. Eine besondere
Bezugsfläche für einen bestimmten Nachweis muss jedoch ausdrücklich benannt und
begründet werden. Eine heimliche zweite Baugrundstücksermittlung ist unzulässig.

## 4. Beispiel: Baugrundstücke als verlässlicher Lieferant

Das Baugrundstücksmodul ermittelt aus Flurstücken, rechtlichen Zuordnungen,
Flächenabzügen und gegebenenfalls zugeordneten Gemeinschaftsflächen die benötigten
Grundstückseinheiten. Es stellt grundsätzlich eine **Liste** bereit. Ein einzelnes
Baugrundstück ist eine Liste mit einem Eintrag; mehrere Grundstücke dürfen nicht
unbemerkt zu einer gemeinsamen Berechnungsfläche zusammengefasst werden.

Der öffentliche Vertrag muss mindestens eindeutig ausdrücken:

| Information | Bedeutung für Verbraucher |
| --- | --- |
| Projekt-ID und Eingabestand | Zu welchem Vorhaben und Datenstand gehört das Ergebnis? |
| Identität der Grundstückseinheit | Welchem Grundstück sind Berechnungen und Ergebnisse zugeordnet? |
| Zugeordnete Flurstück-IDs | Welche Flurstücke bilden die Einheit? |
| Eigentumsfläche in m² | Tatsächliche Eigentumsfläche; getrennt von der Bezugsfläche |
| Maßgebende Bezugsfläche in m² | Fachlich bestimmter Ansatz für den jeweiligen Nachweis |
| Abzüge, Zuschläge und Grundlagen | Nachvollziehbare Herleitung; keine erneute Anrechnung beim Verbraucher |
| Status, Warnungen und Berechnungs-/Regelstand | Darf das Ergebnis verwendet werden und unter welchen Voraussetzungen? |

Diese Tabelle ist eine Vorgabe für den Vertrag, keine Behauptung, dass bereits
jedes Feld in der heutigen Implementierung vorhanden ist. Bestehende Typen wie
`BaugrundstueckGruppe` werden kontrolliert weiterentwickelt oder durch einen
Adapter in einen ausdrücklich dokumentierten Ergebnisvertrag übersetzt.

Eine Erweiterung um weitere Zuordnungsfälle, präzisere Flächengeometrien oder neue
Abzugsregeln erfolgt im Liefermodul. GRZ, GFZ und andere Verbraucher verwenden
weiterhin die bereitgestellten Einheiten und Bezugsflächen. Sie ziehen bereits
berücksichtigte Flächen nicht noch einmal ab und verteilen Zuschläge nicht erneut.

Grundstücksidentitäten werden nicht anhand der Position in einer Liste zugeordnet.
Ändern sich Zuordnung oder Gruppierung, muss eine Aufteilung oder Zusammenführung
explizit behandelt werden. Wenn heutige Schlüssel aus veränderlichen Angaben
abgeleitet sind, benötigt ihre Ablösung eine Migration bzw. Zuordnung alter und
neuer Schlüssel. Berechnungen dürfen dabei nicht still einem anderen Grundstück
zugewiesen werden.

## 5. Beispiel: unabhängig erweiterbares PDF-Modul

Das PDF-/Formularmodul stellt klar getrennte Operationen bereit, beispielsweise
Formular analysieren, Felder beschreiben, Werte validieren und eine neue befüllte
Dokumentversion erzeugen. Seine Verbraucher arbeiten mit Feldkennungen,
Feldtypen, Werten, Dokumentreferenzen und nachvollziehbaren Fehlermeldungen.

Die PDF-Bibliothek, AcroForm-Objektnummern, Darstellungsdetails und interne
Analyseverfahren bleiben innerhalb des Moduls bzw. seines Adapters. Die UI darf
nicht direkt von einem konkreten Parser abhängen. Formularvorlagen ordnen
fachliche Projektwerte über eine klar definierte Kontextschnittstelle Feldern zu.
Neue Vorlagen benötigen keine neue Grundstücks- oder GRZ-Berechnung.

Das Modul kann später etwa bedingte Felder, wiederholbare Gruppen, komplexere
Validierungen oder eine andere PDF-Engine unterstützen. Bestehende Feldtypen und
Aufrufe bleiben dabei nutzbar. Neue Fähigkeiten werden ausdrücklich beschrieben;
unbekannte oder nicht unterstützte Feldtypen werden nicht still falsch befüllt.

DMS und PDF-Bearbeitung haben getrennte Aufgaben: Die Formularlogik erzeugt das
bearbeitete Ergebnis, der Service legt es unter Beachtung von Rechten, Sperren und
Versionen als neue Dokumentversion ab. Das Original wird nicht überschrieben.
PDF-Ausgabe und XBau-Ausgabe sind eigenständige Verbraucher fachlicher Daten;
keines benötigt die internen Datenstrukturen des jeweils anderen Exportmoduls.

## 6. Kompatibilität und Versionierung

- Eingaben und Ergebnisse haben dokumentierte Einheiten, Wertebereiche und eine
  eindeutige Bedeutung für `null`, fehlend und nullwertige Zahlen. Eine fehlende
  Fläche ist keine Fläche von `0 m²` und eine fehlende Festsetzung keine Freigabe.
- Neue optionale Felder erhalten sichere Vorgaben. Sie sind nur kompatibel, wenn
  bestehende Verbraucher sie tatsächlich ignorieren können und das Verhalten ohne
  das neue Feld fachlich korrekt bleibt. Dies wird getestet, nicht angenommen.
- Felder werden nicht still umbenannt, gelöscht oder in ihrer Bedeutung geändert.
  Auch neue Enum-Varianten können bestehende Verbraucher brechen.
- Änderungen an der internen Methode oder am Regelwerk erhalten einen passenden
  Berechnungs-/Regelstand. Änderungen am öffentlichen Vertrag werden zusätzlich
  als Vertragsänderung behandelt. Datenversion, Projektversion und Regelversion
  erfüllen unterschiedliche Aufgaben und dürfen nicht miteinander verwechselt werden.
- Eine notwendige Vertragsänderung benennt alle Verbraucher und umfasst Adapter,
  Migrationen und Prüfungen. Bisherige gespeicherte Projekte und Formularvorlagen
  bleiben lesbar; es gibt keine automatische destruktive Neuberechnung beim Öffnen.
- Eine sachlich geänderte Rechtslage darf trotz gleicher Datenform andere Ergebnisse
  erzeugen. Die Änderung wird dokumentiert und abhängige Ergebnisse werden als
  neu zu prüfen markiert. Kompatibilität bedeutet keinen Anspruch auf unveränderte
  fachliche Zahlen nach einer Regeländerung.

## 7. Aktualität, Fehler und mehrere Projektakten

Jedes weitergegebene Berechnungsergebnis muss seinem Projekt, seiner Einheit,
seinen relevanten Eingaben und seinem Regelstand zugeordnet werden können.
Ändern sich relevante Grundlagen, werden abhängige Ergebnisse gezielt ungültig
bzw. als veraltet markiert. Eine neue Grundstücksfläche darf nicht zusammen mit
unverändert als aktuell dargestellten alten GRZ-/GFZ-Ergebnissen verwendet werden.

Vollständige, teilweise ermittelte, ungültige und veraltete Ergebnisse werden
unterschieden. Verbraucher verwenden ein Teilergebnis nur dann, wenn ihr Vertrag
das fachlich ausdrücklich erlaubt. Ein Fehler wird nicht durch ein leeres oder
scheinbar gültiges Standardergebnis verdeckt. Eine ausfallende PDF-Analyse darf
unabhängige Grundstücksberechnungen weiterhin benutzbar lassen.

Requests, Antworten, Entwürfe und Caches gehören zu einer konkreten Projektakte.
Die aktuell sichtbare Registerkarte ist keine fachliche Datenquelle für einen
bereits gestarteten Request. Verspätete Antworten verändern nur die ursprüngliche
Akte; Cache-Schlüssel enthalten alle relevanten Versionen und Eingaben.
Optimistic Locking, Berechtigungen und Dokumentensperren bleiben an den
Servicegrenzen wirksam. Speicherfehler erhalten den Entwurf.

## 8. Prüfungen für unabhängige Weiterentwicklung

Für geänderte Module sind passende Tests erforderlich:

- **Modultests:** Fachfälle und Grenzfälle der eigenen Berechnung oder Verarbeitung.
- **Vertragstests:** Öffentliche Typen, Serialisierung, Einheiten, Status und Verhalten
  bei fehlenden Angaben; bestehende Verbraucher können das Ergebnis weiter nutzen.
- **Integrationstests an betroffenen Grenzen:** etwa neue Grundstücksermittlung →
  GRZ/GFZ je Einheit oder Formularbefüllung → neue DMS-Version.
- **Kompatibilitätsfälle:** repräsentative gespeicherte Altprojekte, frühere
  Regelpakete und bestehende Formularvorlagen bleiben lesbar und sinnvoll verwendbar.
- **Fehlerfälle:** veraltete Ergebnisse, Regelwechsel, mehrere Grundstücke,
  Hintergrundprojekte, Speicher-/Versionskonflikte und nicht unterstützte PDF-Felder.

Die unabhängige Weiterentwicklung gilt als belegt, wenn interne Änderungen bei
unverändertem Vertrag die Verbraucher ohne Anpassung weiterhin bestehen lassen.
Tests prüfen fachliche Erwartungen und Übergaben, nicht interne Implementierungsdetails.
Für reine Dokumentations- oder kosmetische Änderungen sind keine neuen Fachtests nötig.

## 9. Vorgehen bei Erweiterungen

1. Betroffenes Fachmodul und seine öffentlichen Ein- und Ausgaben identifizieren.
2. Verbraucher und Abhängigkeiten feststellen; den bestehenden Vertrag lesen.
3. Prüfen, ob die Erweiterung intern möglich ist oder den Vertrag wirklich verändert.
4. Intern erweitern und den bestehenden Vertrag erhalten; neue Fähigkeiten bei Bedarf
   über dokumentierte Erweiterungen bzw. Adapter bereitstellen.
5. Regelstand, Datenmigration und Ergebnisaktualität passend zur Änderung behandeln.
6. Modul-, Vertrags- und betroffene Integrationstests ausführen.
7. Im PR benennen: geändertes Modul, Auswirkungen auf den Vertrag, Migrationen,
   betroffene Verbraucher und tatsächliche Validierung.

Diese Regeln beschreiben den Zielaufbau. Bestehende Kopplungen werden bei Arbeiten
am betroffenen Bereich schrittweise abgebaut. Ein neues Feature ist kein Anlass,
die gesamte Anwendung vorsorglich umzubauen. Eine unvermeidliche Änderung mehrerer
Module bleibt zulässig, muss aber durch den fachlichen Vertrag begründet sein.
