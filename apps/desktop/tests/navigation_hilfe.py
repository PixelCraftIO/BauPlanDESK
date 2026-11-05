"""Gemeinsame Navigation der UI-Tests.

Arbeitsplatzbereiche liegen in der Werkzeugleiste. Fachbereiche einer Projektakte
liegen im Projektbaum links (Register → Gruppe → Bereich); ist der gesuchte
Bereich zugeklappt, werden die Zweige aufgeklappt.
"""

ARBEITSPLATZ = {"Projekte", "Kunden / Bauherren", "Formularbibliothek", "Einstellungen"}


def akte(scope):
    return scope.locator(".projekt-sitzung:visible")


def navigation_zeigen(scope):
    """Die Arbeitsansicht im Aufmaß klappt Projektnavigation und Register ein."""
    knopf = akte(scope).get_by_role("button", name="Projektnavigation einblenden", exact=True)
    if knopf.count() and knopf.first.is_visible():
        knopf.first.click()


def register(scope, titel):
    """Register = oberster Zweig des Projektbaums; wird aufgeklappt, nie zugeklappt."""
    navigation_zeigen(scope)
    knopf = gliederung(scope).get_by_role("button", name=titel, exact=True)
    if knopf.get_attribute("aria-expanded") == "false":
        knopf.click()


def gliederung(scope):
    return akte(scope).locator(".aktennavigation")


def bereich(scope, name):
    if name in ARBEITSPLATZ:
        scope.locator(".app-werkzeuge").get_by_role("button", name=name, exact=True).click()
        return
    navigation_zeigen(scope)
    knopf = gliederung(scope).get_by_role("button", name=name, exact=True)
    if not knopf.count():
        for z in gliederung(scope).locator('.zweigkopf[aria-expanded="false"]').all():
            z.click()
        knopf.first.wait_for(state="visible", timeout=2000)
    knopf.click()


def verfahrensbereich(scope, name):
    groups = {
        "Vorbereitung": ["Verfahren", "Vorhabenprofil", "Anforderungen", "Abweichungen / Befreiungen", "Unterschriften", "Unterlagen und Arbeitsaufträge"],
        "Laufendes Verfahren": ["Kommunikation", "Nachforderungen", "Behördliche Schritte", "Schreiben", "Abgabestände"],
        "Entscheidung und Ausführung": ["Bescheide / Auflagen", "Bauausführung", "Korrekturen", "Verfahrenschronik"],
    }
    for phase, names in groups.items():
        if name in names:
            akte(scope).get_by_role("navigation", name="Verfahrensphasen", exact=True).get_by_role("button",name=phase,exact=True).click()
            break
    akte(scope).get_by_role("navigation", name="Verfahrensbereiche", exact=True).get_by_role("button",name=name,exact=True).click()
