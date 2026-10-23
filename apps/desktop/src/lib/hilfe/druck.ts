import { handbuchDruckCss, handbuchInhaltHtml } from "./export";

/** Isolierter Handbuchdruck; übernimmt niemals Projektinhalte oder offene Formulare. */
export function handbuchDrucken(onFehler?: (fehler: Error) => void): () => void {
  if (document.getElementById("bau-druckbereich"))
    throw new Error("Bitte die andere Druckausgabe zuerst abschließen.");
  const root = document.createElement("div");
  root.id = "bau-handbuch-druck";
  root.className = "handbuch-ausgabe";
  root.style.display = "none";
  // Ausschließlich mit HTML-escaping erzeugte, mitgelieferte Handbuchinhalte.
  root.innerHTML = handbuchInhaltHtml();
  const css = document.createElement("style");
  css.textContent = handbuchDruckCss + `@media print {
    html,body { height:auto!important; overflow:visible!important; margin:0!important; padding:0!important; background:white!important; }
    body > :not(#bau-handbuch-druck) { display:none!important; }
    #bau-handbuch-druck { display:block!important; }
  }`;
  let aktiv = true;
  const clean = () => {
    aktiv = false;
    root.remove(); css.remove(); window.removeEventListener("afterprint", clean);
  };
  document.head.append(css);
  document.body.append(root);
  window.addEventListener("afterprint", clean);
  // Vor dem Druck müssen auch noch nicht in der Hilfe geöffnete Abbildungen geladen sein.
  void Promise.all([...root.querySelectorAll("img")].map((img) => img.decode()))
    .then(() => { if (aktiv) window.print(); })
    .catch((e) => {
      if (!aktiv) return;
      clean();
      onFehler?.(e instanceof Error ? e : new Error("Handbuchabbildungen konnten nicht geladen werden."));
    });
  return clean;
}
