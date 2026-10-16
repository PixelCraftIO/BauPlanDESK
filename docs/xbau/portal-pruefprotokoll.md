# Prüfprotokoll: Portal-Anbindung NRW

Prüfdatum: 16.10.2026. Dieses Protokoll trennt tatsächlich geprüften Code,
Abrufversuche und noch zu prüfende externe Aussagen.

## Durchgeführt

- `origin/main` abgerufen: `4953dcc` (Merge PR #28, Projektübersicht).
- Arbeitsbranch `codex/xbau-nachrichtenaustausch`, Implementierungsstand
  `7b3a5a6`, gelesen; Arbeitsverzeichnis vor Analyse sauber.
- Projekt-/Fachmodulregeln in `AGENTS.md` und `docs/entwicklungsregeln.md` gelesen.
- Verträge und Abläufe für DMS-Versionen, Abgabestand, Einreichung,
  Dokumentkategorien, PDF-Ausgabe, XBau-Journal und Verfahrensübergabe gelesen.
- Bestehende Dokumentation zu Verfahrensakte und XBau-Grenzen abgeglichen.
- Keine neuen Anwendungsfunktionen programmiert; nur Analyse/Planung ergänzt.
- Keine Portalzugänge eingerichtet, keine Behörden/Betreiber kontaktiert,
  keine Test- oder Produktivanträge versendet.

## Externe Abrufprüfung

| Ziel | Ergebnis in dieser Umgebung | Erkenntnisgrenze |
| --- | --- | --- |
| <https://bauportal.nrw/> | Proxy verweigert HTTPS-Tunnel mit HTTP 403 | Seiteninhalt, Verfahren, Schnittstellen und Dateiannahme nicht geprüft |
| <https://docs.fitko.de/fit-connect/> | Proxy verweigert HTTPS-Tunnel mit HTTP 403 | Keine aktuelle FIT-Connect-Spezifikation geprüft; insbesondere kein Beleg für NRW-Nutzung |
| GitHub-Organisation `FITKO` und öffentliche Repository-Suchen | HTML technisch abrufbar | Kein belastbarer NRW-Schnittstellenvertrag oder bestätigter Portaladapter daraus gewonnen |

Die 403-Antwort stammt beim Aufbau des Proxy-Tunnels aus der Cloud-Umgebung;
sie ist **keine Ablehnung durch den Portalbetreiber** und kein Hinweis auf
fehlende persönliche NRW-Anmeldedaten. Allgemeine Paketmanager-/GitHub-Freigaben
bestehen, die betroffenen amtlichen Domains sind im laufenden Netzwerkprofil
nicht freigegeben. Es wurde kein alternativer Proxy/Umweg verwendet.

## Gespeicherte Netzkonfiguration

Als Konfigurationsentwurf wurden diese konkreten Recherche-Domains gespeichert:

- `bauportal.nrw`, `www.bauportal.nrw`
- `mhkbd.nrw`, `www.mhkbd.nrw`
- `docs.fitko.de`, `fitko.de`, `www.fitko.de`
- `www.xleitstelle.de`
- `xrepository.de`, `www.xrepository.de`

Die bestehende Liste expliziter zusätzlicher Domains war laut Runtime leer;
Paketmanager-Vorgaben wurden durch das partielle Update nicht ersetzt.
Setupskripte, Startanweisungen, Repositoryzuordnungen und Geheimnisse wurden
nicht verändert. Der Entwurf wurde als `saved`, `requires_publish: true`
bestätigt. Speichern des Entwurfs hat die laufende Netzrichtlinie nicht geändert.
Die Änderungen müssen in den Umgebungseinstellungen geprüft/gespeichert und die
Umgebung veröffentlicht werden; danach Zugriff und ggf. notwendige offizielle
Weiterleitungs-/Dokumentationshosts erneut prüfen.

## Offizielle Quellen für die noch offene Prüfung

Die folgenden Links sind **Prüfziele**, keine in dieser Sitzung gelesenen Belege:

- [Bauportal NRW](https://bauportal.nrw/): tatsächlich verfügbare Verfahren,
  Zuständigkeiten, Antragszugänge, Upload-/Importhinweise, technische Dokumentation
  und Ansprechpartner prüfen.
- [MHKBD NRW](https://www.mhkbd.nrw/): amtliche Hinweise zum digitalen
  Baugenehmigungsverfahren und zur Portalzuständigkeit suchen; konkrete Unterseite
  nach erfolgreichem Abruf dokumentieren.
- [FITKO-Dokumentation FIT-Connect](https://docs.fitko.de/fit-connect/): nur
  bei belegtem Bezug zum konkreten Ziel dessen technische Verträge prüfen.
- [XLeitstelle Planen und Bauen](https://www.xleitstelle.de/) und
  [XRepository](https://www.xrepository.de/): veröffentlichte Standards,
  Nachrichten- und Codelistenstände prüfen. Standardunterstützung ersetzt keinen
  Portal-Nutzungsvertrag.

Nach erfolgreichem Zugriff je Feststellung genaue Dokument-URL, Dokumenttitel,
Version/Datum, Abrufdatum und relevante Aussage protokollieren. Öffentlich nicht
belegte Fähigkeiten bleiben bis zu einer Betreiberbestätigung offen. Eine
Betreiberanfrage wird erst bei ausdrücklichem Auftrag versendet.

## Weiterhin offen

- Eine für externe Planungssoftware nutzbare NRW-Schnittstelle und deren
  Zugangs-/Nutzungsbedingungen.
- Konkrete Pilotkommune/Stelle und Verfahrensart mit akzeptiertem Dateiprofil.
- Tatsächlich zugelassene XBau-Versionen, Importpakete und veröffentlichte
  dynamische Codelisten; kein Produktivversprechen für die ausgelieferte Entwurfsliste.
- Rückkanal, Originalquittungen, Vorgangs-/Dateiidentitäten und fachliche Bedeutung.
- Testzugang, Hersteller-Onboarding und bestätigte Ende-zu-Ende-Abnahme.

Die Architektur-/Ablaufplanung bleibt trotz dieser offenen Punkte verwendbar.
Eine Aussage „NRW-API geprüft und direkt anbindbar“ wäre derzeit nicht belegt.
