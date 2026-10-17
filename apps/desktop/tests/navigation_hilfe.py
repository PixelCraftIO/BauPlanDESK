"""Gemeinsame Navigation der UI-Tests.

Arbeitsplatzbereiche liegen in der Werkzeugleiste. Fachbereiche einer Projektakte
liegen im Register, dessen Gliederung links angezeigt wird; ist der gesuchte
Bereich nicht in der aktuellen Gliederung, werden die Register der Reihe nach
geöffnet.
"""

ARBEITSPLATZ = {"Projekte", "Kunden / Bauherren", "Formularbibliothek", "Einstellungen"}


def akte(scope):
    return scope.locator(".projekt-sitzung:visible")


def register(scope, titel):
    akte(scope).locator(".register").get_by_role("tab", name=titel, exact=True).click()


def gliederung(scope):
    return akte(scope).locator(".aktennavigation")


def bereich(scope, name):
    if name in ARBEITSPLATZ:
        scope.locator(".app-werkzeuge").get_by_role("button", name=name, exact=True).click()
        return
    knopf = gliederung(scope).get_by_role("button", name=name, exact=True)
    if not knopf.count():
        for reiter in akte(scope).locator(".register").get_by_role("tab").all():
            reiter.click()
            try:
                knopf.wait_for(state="visible", timeout=1000)
                break
            except Exception:
                continue
    knopf.click()
