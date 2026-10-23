//! Begrenzte synchrone Datei-/PDF-Arbeit außerhalb asynchroner Executor-Threads.
//! Der Permit bleibt bei Abbruch des wartenden Futures bis zum Arbeitsende im
//! Thread. Grenzen werden somit auch nach einem abgebrochenen Request eingehalten.
use crate::{CoreError, CoreResult};
use tokio::sync::{Semaphore, oneshot};
static ARBEIT: Semaphore = Semaphore::const_new(2);
pub async fn ausfuehren<T, F>(name: &'static str, arbeit: F) -> CoreResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> CoreResult<T> + Send + 'static,
{
    let platz = ARBEIT
        .acquire()
        .await
        .map_err(|_| CoreError::Storage("Arbeitswarteschlange geschlossen".into()))?;
    let (tx, rx) = oneshot::channel();
    std::thread::Builder::new()
        .name(format!("bauplan-{name}"))
        .spawn(move || {
            let _platz = platz;
            let start = std::time::Instant::now();
            let ergebnis = std::panic::catch_unwind(std::panic::AssertUnwindSafe(arbeit))
                .unwrap_or_else(|_| {
                    Err(CoreError::Storage(
                        "Dateiverarbeitung unerwartet beendet".into(),
                    ))
                });
            tracing::debug!(
                vorgang = name,
                dauer_ms = start.elapsed().as_millis() as u64,
                erfolgreich = ergebnis.is_ok()
            );
            let _ = tx.send(ergebnis);
        })
        .map_err(|e| CoreError::Storage(format!("Dateiverarbeitung starten: {e}")))?;
    rx.await
        .map_err(|_| CoreError::Storage("Dateiverarbeitung unterbrochen".into()))?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn panik_und_fehler_beenden_nicht_die_warteschlange() {
        assert!(
            ausfuehren::<(), _>("test", || panic!("Parser"))
                .await
                .is_err()
        );
        assert_eq!(ausfuehren("test", || Ok(42)).await.unwrap(), 42);
    }
}
