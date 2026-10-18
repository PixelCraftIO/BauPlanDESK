//! Begrenzung gleichzeitig erzeugter großer Exporte (Aktenexport,
//! Einreichungspakete) je Prozess.
//!
//! Große Exporte belegen Arbeitsspeicher (Einzeldateien, Manifest, ZIP-Puffer)
//! und Rechenzeit. Ohne Grenze können viele parallele Anfragen ein NAS oder einen
//! Server per Speichermangel anhalten. Weitere Anfragen warten, bis ein Platz frei
//! wird; sie werden nicht abgewiesen.
use crate::{CoreError, CoreResult};
use tokio::sync::{Semaphore, SemaphorePermit};

/// Höchstzahl gleichzeitig laufender großer Exporte je Prozess.
pub const MAX_GLEICHZEITIGE_EXPORTE: usize = 2;

static EXPORTE: Semaphore = Semaphore::const_new(MAX_GLEICHZEITIGE_EXPORTE);

/// Wartet auf einen freien Exportplatz. Der Platz gilt, bis die Rückgabe fällt.
pub(crate) async fn platz() -> CoreResult<SemaphorePermit<'static>> {
    EXPORTE
        .acquire()
        .await
        .map_err(|_| CoreError::Storage("Exportbegrenzung nicht verfügbar".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn weitere_exporte_warten_bis_ein_platz_frei_wird() {
        let alle = EXPORTE
            .acquire_many(MAX_GLEICHZEITIGE_EXPORTE as u32)
            .await
            .unwrap();
        let mut naechster = std::pin::pin!(platz());
        tokio::select! {
            biased;
            _ = &mut naechster => panic!("bei voller Belegung darf kein Export starten"),
            _ = std::future::ready(()) => {}
        }
        drop(alle);
        drop(naechster.await.unwrap());
    }
}
