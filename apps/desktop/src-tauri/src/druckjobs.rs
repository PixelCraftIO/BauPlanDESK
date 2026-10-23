//! Abbruchkennungen gehören genau zu Benutzer, Projekt und laufendem Auftrag.
use core_lib::{CoreError, CoreResult, domain::ProjectId, druckausgabe::Drucksteuerung};
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;
#[derive(Default)]
pub struct Druckjobs(Mutex<HashMap<Uuid, (ProjectId, Uuid, Drucksteuerung)>>);
pub struct Auftrag<'a> {
    pub steuerung: Drucksteuerung,
    id: Uuid,
    jobs: &'a Druckjobs,
}
impl Druckjobs {
    pub fn starten(
        &self,
        id: Uuid,
        projekt: ProjectId,
        benutzer: Uuid,
        steuerung: Drucksteuerung,
    ) -> CoreResult<Auftrag<'_>> {
        let mut jobs = self
            .0
            .lock()
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        if jobs.len() >= 8 || jobs.contains_key(&id) {
            return Err(CoreError::InvalidInput(
                "Druckauftrag bereits vorhanden oder Warteschlange voll".into(),
            ));
        }
        jobs.insert(id, (projekt, benutzer, steuerung.clone()));
        Ok(Auftrag {
            steuerung,
            id,
            jobs: self,
        })
    }
    pub fn benutzer_abbrechen(&self, benutzer: Uuid) -> CoreResult<()> {
        for (_, u, s) in self
            .0
            .lock()
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .values()
        {
            if *u == benutzer {
                s.abbrechen();
            }
        }
        Ok(())
    }
    pub fn abbrechen(&self, id: Uuid, projekt: ProjectId, benutzer: Uuid) -> CoreResult<()> {
        if let Some((p, u, s)) = self
            .0
            .lock()
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .get(&id)
            && *p == projekt
            && *u == benutzer
        {
            s.abbrechen();
        }
        Ok(())
    }
}
impl Drop for Auftrag<'_> {
    fn drop(&mut self) {
        if let Ok(mut jobs) = self.jobs.0.lock() {
            jobs.remove(&self.id);
        }
    }
}
