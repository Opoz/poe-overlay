//! Wires the Client Log to the Run timer and persists the results.

use std::path::PathBuf;

use poe_core::{
    clock::Millis,
    gems::{GemData, GemsView, decorate_view},
    guide::{GuideBook, GuideCommand, GuideOptions, GuideView},
    log::LogEvent,
    pob::Build,
    profile::Profile,
    run::{Command, Run, RunTimer, TimerEvent, TimerSettings},
    tail::LogTail,
    tree::{PassiveTree, TreeOverlay, TreeView},
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
    tree: PassiveTree,
    tree_overlay: TreeOverlay,
    guides: GuideBook,
    gem_data: GemData,
}

impl Tracker {
    pub fn new(storage: Storage, settings: &Settings) -> Self {
        let session = storage.load_session();
        let history = storage.load_history();
        let mut timer = RunTimer::new(TimerSettings {
            pause_in_hideout: settings.pause_in_hideout,
        });
        timer.category = Some(settings.category.clone());
        timer.restore(session.timer.clone());
        // Catch up on anything logged while the app was closed.
        let tail = session
            .log_path
            .clone()
            .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() >= session.log_offset))
            .map(|p| LogTail::new(p, session.log_offset));
        Self {
            storage,
            timer,
            tail,
            history,
            saved_offset: None,
            tree: PassiveTree::bundled(),
            tree_overlay: session.tree,
            guides: GuideBook::bundled(),
            gem_data: GemData::bundled(),
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
        let mut events = Vec::new();
        for record in &records {
            events.extend(self.timer.on_record(record));
            if let LogEvent::AreaGenerated { area_id, .. } = &record.event
                && let Some(profile) = self.timer.active_profile_mut()
            {
                self.guides.on_area(profile, area_id);
            }
        }
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

    pub fn select_character(&mut self, name: &str) {
        self.timer.select(name, now_ms());
        self.save_session();
    }

    pub fn profiles(&self) -> Vec<Profile> {
        self.timer.profiles().cloned().collect()
    }

    pub fn active_character(&self) -> Option<String> {
        self.timer.active().map(str::to_owned)
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

    pub fn tree(&self) -> &PassiveTree {
        &self.tree
    }

    pub fn tree_overlay(&self) -> &TreeOverlay {
        &self.tree_overlay
    }

    pub fn tree_view(&self) -> TreeView {
        let active = self.timer.active();
        let profile = active.and_then(|n| self.timer.profiles().find(|p| p.name == n));
        let (no_passives, no_masteries) = (Default::default(), Default::default());
        TreeView::build(
            &self.tree,
            &self.tree_overlay,
            active,
            profile.and_then(|p| p.class.as_deref()),
            profile.map_or(&no_passives, |p| &p.passives),
            profile.map_or(&no_masteries, |p| &p.masteries),
            profile.and_then(|p| p.build.as_ref().map(|b| (b, p.level))),
        )
    }

    pub fn toggle_tree(&mut self) {
        self.tree_overlay.toggle_visible();
        self.save_session();
    }

    pub fn toggle_calibration(&mut self) {
        self.tree_overlay.toggle_calibration();
        self.save_session();
    }

    /// A Calibration click at game-window pixels, in a window `game_height` tall.
    pub fn tree_click(&mut self, x: f64, y: f64, game_height: f64) {
        let view = self.tree_view();
        if let Some(start) = view.class_start {
            self.tree_overlay
                .click(&self.tree, start.id, (x, y), game_height);
            self.save_session();
        }
    }

    /// Middle-click on the class start with the tree fully zoomed out.
    /// Only while the tree Overlay is showing. Returns whether it aligned.
    pub fn align_tree(&mut self, x: f64, y: f64, game_height: f64) -> bool {
        if !self.tree_overlay.visible {
            return false;
        }
        let Some(start) = self.tree_view().class_start else {
            return false;
        };
        self.tree_overlay
            .align_on_class_start(&self.tree, start.id, (x, y), game_height);
        self.save_session();
        true
    }

    pub fn set_allocations(&mut self, name: &str, allocations: poe_core::tree::Allocations) {
        self.timer.set_allocations(name, allocations);
        self.save_session();
    }

    pub fn guide_view(&self) -> GuideView {
        let active = self.timer.active();
        let profile = active.and_then(|n| self.timer.profiles().find(|p| p.name == n));
        let mut view = GuideView::build(&self.guides, profile);
        if let Some(p) = profile
            && let Some(build) = &p.build
        {
            let guide = self.guides.guide(p.guide_options);
            decorate_view(&mut view, &self.gem_data, guide, build);
        }
        view
    }

    pub fn gems_view(&self) -> GemsView {
        let active = self.timer.active();
        let profile = active.and_then(|n| self.timer.profiles().find(|p| p.name == n));
        GemsView::build(&self.gem_data, profile)
    }

    /// Set (or with `reset`, clear) a hand-picked level range for a Skill Group.
    pub fn set_group_levels(
        &mut self,
        name: &str,
        index: usize,
        levels: Option<poe_core::pob::LevelRange>,
        reset: bool,
    ) {
        if let Some(profile) = self.timer.profile_mut(name) {
            if reset {
                profile.group_levels.remove(&index);
            } else {
                profile.group_levels.insert(index, levels);
            }
            self.save_session();
        }
    }

    /// Attach an imported Build to a Character.
    pub fn set_build(&mut self, name: &str, build: Build, source: String) -> bool {
        let Some(profile) = self.timer.profile_mut(name) else {
            return false;
        };
        profile.apply_build(build, source);
        self.save_session();
        true
    }

    pub fn guide_command(&mut self, command: GuideCommand) {
        if let Some(profile) = self.timer.active_profile_mut() {
            self.guides.command(profile, command);
            self.save_session();
        }
    }

    pub fn set_guide_options(&mut self, name: &str, options: GuideOptions) {
        if let Some(profile) = self.timer.profile_mut(name) {
            if profile.guide_options.bandit != options.bandit {
                profile.bandit_set_by_hand = true;
            }
            profile.guide_options = options;
            self.save_session();
        }
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
            timer: self.timer.save(now_ms()),
            tree: self.tree_overlay.clone(),
        });
        self.saved_offset = Some(offset);
    }
}
