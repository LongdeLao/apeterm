use crate::app::*;
use crate::{
    db,
    features::sec::{self, EntityKind},
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// UI + runtime state owned by the SEC feature.
#[derive(Debug)]
pub struct SecFeature {
    pub tab: SecTab,
    pub institutional_selection: usize,
    pub ceo_selection: usize,
    pub congress_selection: usize,
    pub status: Option<String>,
    pub loading: bool,
    pub(crate) receiver: Option<mpsc::Receiver<SecEvent>>,
    pub(crate) last_refresh: Option<Instant>,
}

impl Default for SecFeature {
    fn default() -> Self {
        Self {
            tab: SecTab::Institutional,
            institutional_selection: 0,
            ceo_selection: 0,
            congress_selection: 0,
            status: None,
            loading: false,
            receiver: None,
            last_refresh: None,
        }
    }
}

impl App {
    pub fn sec_refresh_interval(&self) -> Duration {
        Duration::from_secs(self.config.sec.refresh_interval_seconds.max(1))
    }
    pub fn refresh_sec(&mut self) {
        if self.sec.loading {
            return;
        }

        let db_path = self.ticker_db_path.clone();
        let config = self.config.sec.clone();
        self.sec.loading = true;
        self.sec.last_refresh = Some(Instant::now());
        self.sec.status = Some("SEC sync running".to_string());

        let (sender, receiver) = mpsc::channel();
        self.sec.receiver = Some(receiver);
        thread::spawn(move || match sec::sync::sync_all(&db_path, &config) {
            Ok(count) => {
                let _ = sender.send(SecEvent::Done(format!("SEC synced {count} entities")));
            }
            Err(error) => {
                let _ = sender.send(SecEvent::Error(error));
            }
        });
    }
    pub fn refresh_selected_sec_entity(&mut self) {
        if self.sec.loading {
            return;
        }
        let Some(entity_id) = self.selected_sec_entity_id() else {
            return;
        };

        let db_path = self.ticker_db_path.clone();
        let config = self.config.sec.clone();
        self.sec.loading = true;
        self.sec.status = Some("SEC entity sync running".to_string());

        let (sender, receiver) = mpsc::channel();
        self.sec.receiver = Some(receiver);
        thread::spawn(
            move || match sec::sync::sync_entity(&db_path, &config, entity_id) {
                Ok(_) => {
                    let _ = sender.send(SecEvent::Done("SEC entity synced".to_string()));
                }
                Err(error) => {
                    let _ = sender.send(SecEvent::Error(error));
                }
            },
        );
    }
    pub fn poll_sec(&mut self) {
        if let Some(receiver) = &self.sec.receiver {
            match receiver.try_recv() {
                Ok(SecEvent::Done(status)) => {
                    self.sec.loading = false;
                    self.sec.receiver = None;
                    self.sec.status = Some(status);
                }
                Ok(SecEvent::Error(error)) => {
                    self.sec.loading = false;
                    self.sec.receiver = None;
                    self.sec.status = Some(format!("SEC sync error: {error}"));
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.sec.loading = false;
                    self.sec.receiver = None;
                }
            }
        }

        self.maybe_auto_refresh_sec();
    }
    pub(crate) fn maybe_auto_refresh_sec(&mut self) {
        if self.sec.loading || !self.onboarding_complete {
            return;
        }

        let Some(last_refresh) = self.sec.last_refresh else {
            self.refresh_sec();
            return;
        };

        if last_refresh.elapsed() >= self.sec_refresh_interval() {
            self.refresh_sec();
        }
    }
    pub fn cycle_sec_tab(&mut self, direction: SelectionDirection) {
        self.sec.tab = match (self.sec.tab, direction) {
            (SecTab::Institutional, SelectionDirection::Previous) => SecTab::Congress,
            (SecTab::Institutional, SelectionDirection::Next) => SecTab::Ceos,
            (SecTab::Ceos, SelectionDirection::Previous) => SecTab::Institutional,
            (SecTab::Ceos, SelectionDirection::Next) => SecTab::Congress,
            (SecTab::Congress, SelectionDirection::Previous) => SecTab::Ceos,
            (SecTab::Congress, SelectionDirection::Next) => SecTab::Institutional,
        };
    }
    pub fn move_sec_selection(&mut self, direction: SelectionDirection) {
        let max_index = self.sec_entity_count().saturating_sub(1);
        let selection = match self.sec.tab {
            SecTab::Institutional => &mut self.sec.institutional_selection,
            SecTab::Ceos => &mut self.sec.ceo_selection,
            SecTab::Congress => &mut self.sec.congress_selection,
        };
        match direction {
            SelectionDirection::Previous => {
                *selection = selection.saturating_sub(1);
            }
            SelectionDirection::Next => {
                *selection = selection.saturating_add(1).min(max_index);
            }
        }
    }
    pub fn active_sec_selection(&self) -> usize {
        match self.sec.tab {
            SecTab::Institutional => self.sec.institutional_selection,
            SecTab::Ceos => self.sec.ceo_selection,
            SecTab::Congress => self.sec.congress_selection,
        }
    }
    pub fn selected_sec_entity_id(&self) -> Option<i64> {
        let connection = db::open(&self.ticker_db_path).ok()?;
        let entities = match self.sec.tab {
            SecTab::Institutional => {
                crate::features::sec::repo::list_entities(&connection, EntityKind::Institution)
                    .ok()?
            }
            SecTab::Ceos => {
                crate::features::sec::repo::list_ceo_entities(&connection, false).ok()?
            }
            SecTab::Congress => {
                crate::features::sec::repo::list_ceo_entities(&connection, true).ok()?
            }
        };
        let index = self
            .active_sec_selection()
            .min(entities.len().saturating_sub(1));
        entities.get(index).map(|entity| entity.id)
    }
    pub(crate) fn sec_entity_count(&self) -> usize {
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return 0;
        };
        match self.sec.tab {
            SecTab::Institutional => {
                crate::features::sec::repo::list_entities(&connection, EntityKind::Institution)
            }
            SecTab::Ceos => crate::features::sec::repo::list_ceo_entities(&connection, false),
            SecTab::Congress => crate::features::sec::repo::list_ceo_entities(&connection, true),
        }
        .map(|entities| entities.len())
        .unwrap_or(0)
    }
}
