// Funktionsrundgang der Seite /zeicheneditor. Bilder stammen aus dem
// Benutzerhandbuch (docs/handbuch/benutzerhandbuch/bilder, erzeugt mit
// scripts/handbuch-bilder aus dem Beispielprojekt EFH Mustermann).

export type Funktion = {
  id: string;
  titel: string;
  bild: string;
  alt: string;
  text: string;
  punkte: string[];
  /** Weiterlesen im Handbuch (Kapitel#Anker) oder auf der Seite Abläufe. */
  link: string;
  linkText: string;
};

export const FUNKTIONEN: Funktion[] = [
  {
    id: "arbeitsplatz",
    titel: "Arbeitsplatz wie im CAD",
    bild: "ermittlung-aufmass-vollbild",
    alt: "Zeicheneditor im Vollbild mit Werkzeugleiste links, Plan in der Mitte und Paletten rechts",
    text: "Der Plan bekommt die ganze Fläche. Werkzeuge stehen als Symbole links, Paletten für Aufgaben, Objekte und Eigenschaften rechts.",
    punkte: [
      "Mehrere Aufmaße und Zeichnungen als Tabs, auf Wunsch in einem eigenen Fenster auf dem zweiten Bildschirm",
      "PDF-Pläne mit Maßstab oder Zwei-Punkt-Kalibrierung, DXF mit Zeichnungseinheit und Ebenen",
      "Ausschnitte mit eigenem Maßstab für Sammelblätter, Bezugslinie für Höhen im Schnitt",
      "Vollbild mit F11, alle Kurzbefehle mit F1",
    ],
    link: "/handbuch/05-ermittlung#arbeitsplatz-des-editors",
    linkText: "Arbeitsplatz im Handbuch",
  },
  {
    id: "aufgaben",
    titel: "Aufgaben: was noch gezeichnet werden muss",
    bild: "editor-aufgaben",
    alt: "Palette Aufgaben mit Räumen des Raumbuchs und den Knöpfen Zeichnen und Übernehmen",
    text: "Die Palette Aufgaben listet jedes Ziel im Projekt, das einen Messwert braucht: Flurstücke, Abzugsflächen, Bauteile der GRZ, Gebäude, Geschosse, Räume und Positionen der Fachberechnungen.",
    punkte: [
      "„Zeichnen“ stellt Werkzeug, Bedeutung und Zuordnung passend ein",
      "„Übernehmen“ setzt den Messwert ins Zielfeld und verknüpft es mit der Markierung",
      "Von Hand erfasste Werte sind gekennzeichnet, passende Markierungen werden vorgeschlagen",
      "„Verknüpfte Werte gesammelt prüfen“ zeigt alle Übernahmen mit altem und neuem Wert",
    ],
    link: "/ablaeufe#aufmass",
    linkText: "Ablauf der Übernahme",
  },
  {
    id: "eigenschaften",
    titel: "Markierungen mit Bedeutung",
    bild: "editor-eigenschaften",
    alt: "Ausgewählte Dachprojektion im Grundriss, rechts die Eigenschaften mit Bedeutung „Versiegelte Fläche“, Fläche 116 m² und Verwendung in der Regenwasserberechnung",
    text: "Eine Fläche wird erst durch ihre Bedeutung zum Messwert: Raum nach DIN 277, Gebäudegrundfläche, Abzugsfläche, Stellplatz oder Höhe. Die Eigenschaften zeigen Fläche, Umfang und wo der Wert verwendet wird – im Bild speist die Dachfläche die Regenwasserberechnung.",
    punkte: [
      "Zuordnung zu Flurstück, Baugrundstück, Gebäude, Geschoss oder Raumbuch",
      "Stützpunkte, Kantenmaße und Drehung direkt eingeben",
      "Verwendung: welche Felder diese Markierung speisen",
      "Vor dem Löschen nennt die App alle betroffenen Werte",
    ],
    link: "/handbuch/05-ermittlung#zeichnen-und-zuordnen",
    linkText: "Zeichnen und zuordnen",
  },
  {
    id: "konstruieren",
    titel: "Präzise konstruieren",
    bild: "editor-konstruieren",
    alt: "Fangoptionen, Ortho und Winkelraster über dem Plan, Werkzeug Versatz mit Abstand in der Optionsleiste für die gewählte Wand",
    text: "Die gewohnten Handgriffe aus dem Büro-CAD sind da, ohne dass der Editor ein CAD ersetzen will.",
    punkte: [
      "Befehlszeile (F2) mit Kürzeln wie L, PL, R, W, KO, SP, ST, VS 0,5 und RH; Enter wiederholt",
      "Punktfang auf End-, Mittel- und Schnittpunkte, Kanten, eigene Konturen, Lot und Verlängerung; Ortho (F8) und Winkelraster 15/30/45/90°",
      "Koordinaten mit @: 5,20<90 für Länge und Winkel, @3;4 relativ, beim Rechteck Breite;Höhe",
      "Fensterauswahl, Kopieren, Spiegeln, Stutzen/Dehnen, Versatz und Reihe",
    ],
    link: "/handbuch/05-ermittlung#konstruieren-wie-im-cad",
    linkText: "Konstruieren im Handbuch",
  },
  {
    id: "zeichnung",
    titel: "Eigene Zeichnung in Metern",
    bild: "editor-zeichnung",
    alt: "Eigene Zeichnung eines Erdgeschosses mit Wänden, Fenstern, Türen, Maßketten, Raumstempeln und Planzeichen; rechts die Öffnungen der gewählten Außenwand",
    text: "Ohne Plan vom Architekten wird direkt in Metern gezeichnet. PDF-Seiten oder DXF-Dateien lassen sich als Hintergrund einpassen; der Fang greift auf ihre Linien.",
    punkte: [
      "Wände mit Fenstern und Türen (Breite, Höhe, Brüstung, Anschlag)",
      "Maßketten, Texte und Raumstempel mit Nummer, Name und Fläche aus dem Raumbuch",
      "Planzeichen für Lageplan: Bäume, Stellplatz, Zufahrt, Hauseingang, Grenzpunkt",
      "Flächen, Strecken und Höhen als Markierungen wie auf jedem Plan",
    ],
    link: "/handbuch/05-ermittlung#eigene-zeichnung-in-metern",
    linkText: "Eigene Zeichnung im Handbuch",
  },
  {
    id: "darstellung",
    titel: "Darstellung und Bauzustand",
    bild: "editor-darstellung",
    alt: "Ebene Wände gewählt: Stiftstärke, Linienart, Farbe, Bauzustand Neubau und Mauerwerksschraffur unter Eigenschaften",
    text: "Stift, Linienart, Farbe und Schraffur gelten je Ebene oder je Element. Der Bauzustand färbt nach den Bauvorlagenregeln: Bestand grau, Abbruch gelb, Neubau rot.",
    punkte: [
      "Stiftstärken von 0,1 bis 2,5 mm, Schraffuren für Mauerwerk, Beton, Dämmung und Erdreich",
      "Zeichenebenen im Baum mit Auge und Schloss, aktive Ebene per Klick",
      "„Lageplan-Ebenen anlegen“ richtet die üblichen Lageplan-Ebenen ein",
    ],
    link: "/handbuch/05-ermittlung#darstellung-öffnungen-und-planzeichen",
    linkText: "Darstellung im Handbuch",
  },
  {
    id: "werkzeugkiste",
    titel: "Werkzeugkiste mit Vorlagen",
    bild: "editor-werkzeugkiste",
    alt: "Ausklappliste am Polygon mit Vorlagen für Grundstück, GRZ-Bauteile, Gebäude, Räume und DIN 277",
    text: "Vorlagen stellen Werkzeug, Bedeutung, Namen und Zeichenebene mit einem Klick ein, etwa für DIN 277 NUF, versiegelte Fläche, Stellplatz oder Traufhöhe.",
    punkte: [
      "Gruppen für Grundstück, GRZ-Bauteile, Gebäude, Räume, DIN 277 und Höhen",
      "Ausklappliste am Werkzeug: kleines Dreieck oder rechte Maustaste",
      "Gebäude und Geschoss kommen aus der Optionsleiste über dem Plan",
      "Einheitliche Bedeutungen, weniger Nacharbeit in den Eigenschaften",
    ],
    link: "/handbuch/05-ermittlung#zeichnen-und-zuordnen",
    linkText: "Werkzeugkiste im Handbuch",
  },
  {
    id: "planlayout",
    titel: "Planlayout und DXF fürs CAD",
    bild: "editor-planlayout",
    alt: "Einstellungen für das Planlayout mit Format, Maßstab, Plankopf, Nordpfeil und Legende",
    text: "Die eigene Zeichnung wird maßstäblich auf A4 bis A0 ausgegeben, mit Plankopf aus Projekt und Planliste. Für das Büro-CAD gibt es den DXF-Export in Metern.",
    punkte: [
      "Plankopf mit Bauherr, Vorhaben, Plannummer und Index aus der Planliste",
      "Nordpfeil, Maßstabsleiste und Legende der Ebenen",
      "Auf Wunsch direkt als neue Version des Plans ablegen",
      "DXF mit einer Ebene je Zeichenebene, Farben und Strichstärken",
    ],
    link: "/handbuch/05-ermittlung#planlayout-und-dxf",
    linkText: "Planlayout im Handbuch",
  },
  {
    id: "objekte",
    titel: "Ausschnitte, Ebenen, Markierungen",
    bild: "ermittlung-aufmass-objekte",
    alt: "Reiter Objekte mit Ausschnitten und dem Baum aus Zeichenebenen und ihren Objekten",
    text: "Der Reiter Objekte führt die Ausschnitte und einen Baum aus Zeichenebenen mit ihren Markierungen und Zeichenelementen, mit Suche und Filtern nach Bedeutung und Gebäude.",
    punkte: ["„Öffnen“ zeigt nur den gewählten Ausschnitt", "Auge und Schloss je Ebene, Darstellung unter Eigenschaften"],
    link: "/handbuch/05-ermittlung#maßstab-und-ausschnitte",
    linkText: "Ausschnitte im Handbuch",
  },
  {
    id: "messwerte",
    titel: "Messwerte im Blick",
    bild: "ermittlung-messwerte",
    alt: "Liste der Messwerte aller Aufmaße mit Zielfeld und Stand",
    text: "Die Messwerte-Liste zeigt alle Markierungen aller Aufmaße mit Zielfeld und Stand. Veraltete Verknüpfungen nach einer Planänderung fallen sofort auf; „Im Editor zeigen“ springt zur Stelle im Plan.",
    punkte: ["Filter nach Aufmaß und Stand", "„mit älterem Aufmaßstand“ zeigt, was neu zu übernehmen ist"],
    link: "/ablaeufe#planaenderung",
    linkText: "Ablauf bei Planänderung",
  },
];
