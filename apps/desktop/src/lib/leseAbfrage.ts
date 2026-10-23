/** Bündelt ausschließlich gleichzeitig laufende Leseanfragen. Kein Cache für
 * abgeschlossene Antworten. Änderungen trennen alte und neue Anfragegruppen. */
const laufend = new Map<string, Promise<unknown>>();
export function leseAbfragenVerwerfen() {
  laufend.clear();
}
export function leseAbfrage<T>(
  name: string,
  argumente: unknown,
  laden: () => Promise<T>,
): Promise<T> {
  const key = JSON.stringify([name, argumente]);
  const alt = laufend.get(key);
  if (alt) return alt as Promise<T>;
  const neu = laden();
  laufend.set(key, neu);
  void neu
    .finally(() => {
      if (laufend.get(key) === neu) laufend.delete(key);
    })
    .catch(() => {});
  return neu;
}
