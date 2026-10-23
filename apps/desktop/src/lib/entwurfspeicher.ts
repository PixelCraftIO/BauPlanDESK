/** Asynchroner Entwurfsspeicher. Der bisherige localStorage-Schlüssel wird beim
 * ersten Lesen übernommen. Transaktionen werden pro Schlüssel geordnet. */
let datenbank: Promise<IDBDatabase> | undefined;
function db() {
  return (datenbank ??= new Promise<IDBDatabase>((resolve, reject) => {
    const req = indexedDB.open("bauplan-entwuerfe", 1);
    req.onupgradeneeded = () => req.result.createObjectStore("entwuerfe");
    req.onsuccess = () => {
      req.result.onversionchange = () => {
        req.result.close();
        datenbank = undefined;
      };
      resolve(req.result);
    };
    req.onerror = () => {
      datenbank = undefined;
      reject(req.error);
    };
    req.onblocked = () => {
      datenbank = undefined;
      reject(
        new Error(
          "Entwurfsspeicher wird aktualisiert. Andere Fenster schließen.",
        ),
      );
    };
  }));
}
async function transaktion<T>(
  key: string,
  mode: IDBTransactionMode,
  fn: (s: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const d = await db();
  return new Promise((resolve, reject) => {
    const tx = d.transaction("entwuerfe", mode),
      req = fn(tx.objectStore("entwuerfe"));
    tx.oncomplete = () => resolve(req.result);
    tx.onabort = () =>
      reject(
        tx.error ?? req.error ?? new Error("Entwurfssicherung abgebrochen"),
      );
    tx.onerror = () => reject(tx.error ?? req.error);
  });
}
const ketten = new Map<string, Promise<unknown>>();
function geordnet<T>(key: string, fn: () => Promise<T>): Promise<T> {
  const p = (ketten.get(key) ?? Promise.resolve()).catch(() => {}).then(fn);
  ketten.set(key, p);
  void p
    .finally(() => {
      if (ketten.get(key) === p) ketten.delete(key);
    })
    .catch(() => {});
  return p;
}
export const entwurfspeicher = {
  lesen: (key: string) =>
    geordnet(key, async () => {
      const wert = await transaktion(key, "readonly", (s) => s.get(key));
      if (wert !== undefined) return wert;
      // Eine fehlgeschlagene Migration löscht den alten Entwurf nicht.
      const alt = localStorage.getItem(key);
      if (!alt) return null;
      const parsed: unknown = JSON.parse(alt);
      await transaktion(key, "readwrite", (s) => s.put(parsed, key));
      try {
        localStorage.removeItem(key);
      } catch {
        /* IDB ist bereits verbindlich. */
      }
      return parsed;
    }),
  schreiben: (key: string, wert: unknown) =>
    geordnet(key, async () => {
      await transaktion(key, "readwrite", (s) => s.put(wert, key));
    }),
  loeschen: (key: string) =>
    geordnet(key, async () => {
      // Altbestand zuerst entfernen: bei Fehler bleibt der Entwurf auffindbar.
      localStorage.removeItem(key);
      await transaktion(key, "readwrite", (s) => s.delete(key));
    }),
};
