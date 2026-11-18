//! Wires the Client Log to the Run timer and persists the results.

use std::path::PathBuf;

use poe_core::{
    clock::Millis,
    run::{Command, Run, RunTimer, TimerEvent, TimerSettings},
    tail::LogTail,
    view::TimerView,
};

use crate::storage::{Session, Settings, Storage};

/// Local wall-clock milliseconds, the same scale as Client Log stamps.
pub fn now_ms() -> Millis {
    chrono::Local::now()
        .naive_local()
        .and_utc()
        .timestamp_millis()
}

pub struct Tracker {
    storage: Storage,
    timer: RunTimer,
    tail: Option<LogTail>,
    history: Vec<Run>,
    saved_offset: Option<u64>,
}

impl Tracker {
    pub fn new(storage: Storage, settings: &Settings) -> Self {
        let session = storage.load_session();
        let history = storage.load_history();
        let mut timer = RunTimer::new(TimerSettings {
            pause_in_hideout: settings.pause_in_hideout,
        });
        timer.category = Some(settings.category.clone());
        if let Some(run) = session.run.clone() {
            timer.restore(run);
        }
        // Catch up on anything logged while the app was closed.
        let tail = session
            .log_path
            .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() >= session.log_offset))
            .map(|p| LogTail::new(p, session.log_offset));
        Self {
            storage,
            timer,
            tail,
            history,
            saved_offset: None,
        }
    }

    pub fn apply_settings(&mut self, settings: &Settings) {
        self.timer.settings.pause_in_hideout = settings.pause_in_hideout;
        self.timer.category = Some(settings.category.clone());
    }

    pub fn log_path(&self) -> Option<PathBuf> {
        self.tail.as_ref().map(|t| t.path().to_owned())
    }

    /// Follow `path`, unless already following it.
    pub fn follow(&mut self, path: PathBuf) {
        if self.tail.as_ref().is_some_and(|t| t.path() == path) {
            return;
        }
        if let Ok(tail) = LogTail::from_end(&path) {
            self.tail = Some(tail);
            self.save_session();
        }
    }

    /// Process new log lines. Returns whether anything was read.
    pub fn poll(&mut self) -> bool {
        let Some(tail) = &mut self.tail else {
            return false;
        };
        let Ok(records) = tail.poll() else {
            return false;
        };
        if records.is_empty() {
            return false;
        }
        let events: Vec<_> = records
            .iter()
            .flat_map(|r| self.timer.on_record(r))
            .collect();
        if !events.is_empty() {
            self.handle(events);
            self.save_session();
        }
        true
    }

    pub fn command(&mut self, command: Command) {
        let events = self.timer.command(command, now_ms());
        self.handle(events);
        self.save_session();
    }

    pub fn view(&self) -> TimerView {
        let category = self
            .timer
            .run()
            .and_then(|r| r.category.clone())
            .or_else(|| self.timer.category.clone());
        let history = self.history.iter().filter(|r| r.category == category);
        TimerView::build(self.timer.run(), history, now_ms())
    }

    pub fn history(&self) -> &[Run] {
        &self.history
    }

    fn handle(&mut self, events: Vec<TimerEvent>) {
        let mut changed = false;
        for event in events {
            if let TimerEvent::Finished(run) | TimerEvent::Abandoned(run) = event {
                self.history.push(run);
                changed = true;
            }
        }
        if changed {
            self.storage.save_history(&self.history);
        }
    }

    /// Save if the log has advanced since the last save (called periodically).
    pub fn save_if_stale(&mut self) {
        if self.saved_offset != self.tail.as_ref().map(LogTail::offset) {
            self.save_session();
        }
    }

    pub fn save_session(&mut self) {
        let offset = self.tail.as_ref().map_or(0, LogTail::offset);
        self.storage.save_session(&Session {
            log_path: self.log_path(),
            log_offset: offset,
            run: self.timer.save(now_ms()),
        });
        self.saved_offset = Some(offset);
    }
}
