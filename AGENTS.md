# Arbeitsregeln für BauProjektIO

Vor fachlichen oder strukturellen Änderungen
[docs/entwicklungsregeln.md](docs/entwicklungsregeln.md) lesen und anwenden.
Die Datei enthält die verbindlichen Regeln für modulare Weiterentwicklung.

- Features besitzen ihre Fachlogik und geben benötigte Daten über dokumentierte,
  typisierte Verträge weiter. Interne Erweiterungen erhalten bestehende Verbraucher.
- Gemeinsame Fachlogik gehört in `crates/core-lib`; UI, Tauri und Server verwenden
  diese Logik über dünne Adapter. Fachregeln nicht im Frontend duplizieren.
- Grundstückseinheiten werden getrennt übergeben und berechnet. Einheiten,
  Identitäten, Herleitung und Ergebnisaktualität ausdrücklich berücksichtigen.
- PDF-/Formularlogik kapselt Parser und Vorlagen; DMS-Persistenz und Exportformate
  verwenden definierte Schnittstellen statt Feature-Interna.
- Vertragsänderungen, Regeländerungen und Datenmigrationen unterscheiden. Bestehende
  Projekte und Vorlagen erhalten; Fehler und veraltete Ergebnisse nicht kaschieren.
- Tests auf das geänderte Modul und die betroffenen Verträge/Integrationen abstimmen.
  Im PR Auswirkungen auf Verbraucher und Kompatibilität dokumentieren.

Die Regeln gelten für den Zielaufbau und schrittweise Verbesserungen bestehender
Module. Sie verlangen keinen pauschalen Umbau oder eigene Dienste pro Feature.
