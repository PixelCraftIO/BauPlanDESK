# UI-Regressionen für Projekttabs

Die Tests laufen gegen den **Produktionsbuild der echten Svelte-Anwendung**. Ein vor dem Start injizierter Tauri-Testtransport kontrolliert Antworten und simuliert insbesondere fehlgeschlagenes und verspätetes Speichern. Der Testtransport wird nicht in die Anwendung eingebunden.

Voraussetzungen: Repository-Frontendabhängigkeiten, Python mit Playwright und Chromium unter `/usr/bin/chromium`. Keine zusätzlichen Laufzeitabhängigkeiten der Anwendung.

Im Repository:

```sh
npm run check --prefix apps/desktop
npm run build --prefix apps/desktop
python -m http.server 1430 --bind 127.0.0.1 --directory apps/desktop/dist
```

In einer zweiten Shell:

```sh
python apps/desktop/tests/projekttabs.py
```

Die Tests prüfen getrennte Entwürfe, Dirty-Markierungen, Hintergrundtab schließen, verzögertes Speichern, Bauantragsdaten, Beenden, native Menüauslöser, leere Suchergebnisse, Leserrechte und schmale Fenster. Aufnahmen und Prüfergebnis werden unter `docs/ui-umsetzung` geschrieben. Die JSON-Fixture enthält ausschließlich lokale Beispieldaten.

Diese Suite überprüft die UI und ihre IPC-Zuordnung. Backend-Berechnungen und Persistenz werden durch die vorhandenen Rust-Tests überprüft; eine native Desktop-Probe ergänzt die UI-Tests.
