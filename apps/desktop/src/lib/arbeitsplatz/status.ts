/** Anzeigezustände; keine fachliche Freigabeentscheidung im Frontend. */
export const STATUS = {
  entwurf: {
    titel: "Ungespeicherte Änderungen",
    text: "Noch nicht Bestandteil des gespeicherten Projektstands.",
    art: "warnung",
    symbol: "✎",
  },
  gespeichert: {
    titel: "Gespeichert",
    text: "Eingaben gespeichert. Berechnung und fachliche Freigabe sind eigene Schritte.",
    art: "neutral",
    symbol: "▣",
  },
  berechnet: {
    titel: "Berechnet",
    text: "Rechenergebnis für den angezeigten Eingabestand; noch keine fachliche Freigabe.",
    art: "neutral",
    symbol: "≙",
  },
  archiviert: {
    titel: "Archiviert",
    text: "Unveränderlicher Nachweisstand mit seinen damaligen Eingaben und Regeln.",
    art: "neutral",
    symbol: "▣",
  },
  veraltet: {
    titel: "Grundlagen geändert",
    text: "Aktuelle Grundlagen prüfen und einen neuen Nachweis erstellen. Der Archivstand bleibt erhalten.",
    art: "warnung",
    symbol: "↻",
  },
  offen: {
    titel: "Zu prüfen",
    text: "Offene Angaben oder eine fachliche Prüfung sind erforderlich.",
    art: "warnung",
    symbol: "!",
  },
  freigegeben: {
    titel: "Fachlich freigegeben",
    text: "Bürofreigabe für diesen Nachweisstand. Eine Einreichung oder Genehmigung ist damit nicht verbunden.",
    art: "erfolg",
    symbol: "✓",
  },
  exportiert: {
    titel: "Exportiert",
    text: "Dateien bereitgestellt; Versand und Eingang separat dokumentieren.",
    art: "neutral",
    symbol: "↗",
  },
  eingereicht: {
    titel: "Eingereicht",
    text: "Dokumentierter Versand bestimmter Dateiversionen an den Empfänger.",
    art: "neutral",
    symbol: "↗",
  },
  eingegangen: {
    titel: "Eingang belegt",
    text: "Eingangsbeleg und tatsächlich übermittelte Dateiversionen abgeglichen.",
    art: "erfolg",
    symbol: "✓",
  },
} as const;
export type Arbeitsstatus = keyof typeof STATUS;
