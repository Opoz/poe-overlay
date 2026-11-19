//! Wires the Client Log to the Run timer and persists the results.

use std::path::PathBuf;

use poe_core::{
    clock::Millis,
    gems::{GemData, GemsView, decorate_view},
    guide::{GuideBook, GuideCommand, GuideOptions, GuideView},
    lab::{LabLayout, LabState, LabView},
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
    lab: LabState,
    /// Entered the final lab room since the shell last checked.
    lab_end_entered: bool,
    /// Area ID from the last "Generating level" line, paired with the
    /// following "You have entered" line (which has the room name).
    pending_area: Option<String>,
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
            lab: session.lab,
            pending_area: None,
            lab_end_entered: false,
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
            if let LogEvent::AreaGenerated { area_id, .. } = &record.event {
                self.pending_area = Some(area_id.clone());
                if area_id.contains("Labyrinth") && area_id.contains("_end") {
                    self.lab_end_entered = true;
                }
                if let Some(profile) = self.timer.active_profile_mut() {
                    self.guides.on_area(profile, area_id);
                }
            }
            if let LogEvent::AreaEntered { name } = &record.event
                && let Some(area_id) = self.pending_area.take()
            {
                self.lab.on_area(&area_id, name);
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
        TreeView::build(&self.tree, &self.tree_overlay, profile)
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
    /// A click on the tree Overlay: a Calibration point, or in "Fix points"
    /// mode, adding/removing the node under the cursor.
    pub fn tree_click(&mut self, x: f64, y: f64, game_height: f64) {
        if self.tree_overlay.calibrating.is_none() && self.tree_overlay.editing {
            let Some(transform) = self.tree_overlay.transform else {
                return;
            };
            if let Some(p) = self.timer.active_profile_mut() {
                poe_core::tree::toggle_allocation(
                    &self.tree,
                    &transform,
                    (x, y),
                    &mut p.passives,
                    &mut p.masteries,
                );
                self.save_session();
            }
            return;
        }
        let view = self.tree_view();
        if let Some(start) = view.class_start {
            self.tree_overlay
                .click(&self.tree, start.id, (x, y), game_height);
            self.save_session();
        }
    }

    pub fn toggle_tree_editing(&mut self) {
        self.tree_overlay.toggle_editing();
        self.save_session();
    }

    /// Middle-click on the class start with the tree fully zoomed out.
    /// Only while the tree Overlay is showing. Returns whether it aligned.
    pub fn align_tree(&mut self, x: f64, y: f64, game_height: f64) -> bool {
        if !self.tree_overlay.visible {
            return false;
        }
        let view = self.tree_view();
        if self.tree_overlay.ascendancy {
            let Some(start) = view.ascendancy_start else {
                return false;
            };
            self.tree_overlay
                .align_on_ascendancy_start(&self.tree, start.id, (x, y), game_height);
        } else {
            let Some(start) = view.class_start else {
                return false;
            };
            self.tree_overlay
                .align_on_class_start(&self.tree, start.id, (x, y), game_height);
        }
        self.save_session();
        true
    }

    pub fn toggle_tree_ascendancy(&mut self) {
        self.tree_overlay.toggle_ascendancy();
        self.save_session();
    }

    /// Entered the final Izaro room (where the ascension plaque is) since
    /// last asked. The ascendancy panel opens from the plaque.
    pub fn take_lab_end(&mut self) -> bool {
        let entered = self.lab_end_entered;
        self.lab_end_entered = false;
        if entered {
            // The plaque opens the ascendancy panel: be ready for it.
            self.tree_overlay.ascendancy = true;
        }
        entered
    }

    pub fn set_allocations(&mut self, name: &str, allocations: poe_core::tree::Allocations) {
        self.timer.set_allocations(name, allocations);
        self.save_session();
    }

    /// `browse_act` shows another act in the full-act view.
    pub fn guide_view(&self, browse_act: Option<u8>) -> GuideView {
        let active = self.timer.active();
        let profile = active.and_then(|n| self.timer.profiles().find(|p| p.name == n));
        let mut view = GuideView::build(&self.guides, profile, browse_act);
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

    /// Returns a message for the player when the command couldn't do anything.
    pub fn guide_command(&mut self, command: GuideCommand) -> Option<String> {
        let profile = self.timer.active_profile_mut()?;
        let message = self.guides.command(profile, command);
        self.save_session();
        message
    }

    /// Pick the skill set by hand (`None` = by level).
    pub fn set_skill_set(&mut self, name: &str, set: Option<String>) {
        if let Some(p) = self.timer.profile_mut(name) {
            p.skill_set = set;
            self.save_session();
        }
    }

    /// Pick the Tree Stage by hand (`None` = by level).
    pub fn set_tree_stage(&mut self, name: &str, stage: Option<usize>) {
        if let Some(p) = self.timer.profile_mut(name) {
            p.tree_stage = stage;
            self.save_session();
        }
    }

    /// Step the Active Character's skill set back or forward from the one shown.
    pub fn cycle_skill_set(&mut self, delta: i32) {
        let view = self.gems_view();
        let count = view.skill_sets.len();
        if count == 0 {
            return;
        }
        let current = view
            .skill_set
            .as_ref()
            .and_then(|s| view.skill_sets.iter().position(|t| t == s))
            .unwrap_or(0);
        let next = (current as i32 + delta).rem_euclid(count as i32) as usize;
        let title = view.skill_sets[next].clone();
        if let Some(p) = self.timer.active_profile_mut() {
            p.skill_set = Some(title);
            self.save_session();
        }
    }

    /// Step the Active Character's Tree Stage back or forward from the one shown.
    pub fn cycle_tree_stage(&mut self, delta: i32) {
        let view = self.tree_view();
        let (Some(current), count) = (view.stage_index, view.stages.len()) else {
            return;
        };
        if count == 0 {
            return;
        }
        let next = (current as i32 + delta).rem_euclid(count as i32) as usize;
        if let Some(p) = self.timer.active_profile_mut() {
            p.tree_stage = Some(next);
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

    pub fn lab_view(&self) -> LabView {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        LabView::build(&self.lab, &today)
    }

    pub fn load_lab(&mut self, layout: LabLayout) {
        self.lab.load(layout);
        self.save_session();
    }

    pub fn set_lab_detour(&mut self, on: bool) {
        self.lab.darkshrine_detour = on;
        self.save_session();
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
            lab: self.lab.clone(),
        });
        self.saved_offset = Some(offset);
    }
}
