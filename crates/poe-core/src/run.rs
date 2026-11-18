//! Runs and Splits: the campaign timer as a pure state machine.
//!
//! Driven by Log Records ([`RunTimer::on_record`]) and manual hotkey
//! [`Command`]s. All times are [`Millis`] on the local wall-clock scale.
//!
//! - A Run starts when a level-1 Character's Twilight Strand (`1_1_1`) is generated.
//! - Act N's Split is taken on first reaching act N+1's Town; act 10's on the Epilogue,
//!   which also finishes the Run.
//! - Real time counts everything except Pauses. Load-removed time additionally
//!   subtracts loading screens that happened while the Run was running.

use serde::{Deserialize, Serialize};

use crate::{
    area::{ACTS, AreaKind},
    clock::{LogClock, Millis},
    log::{LogEvent, LogRecord},
};

const TWILIGHT_STRAND: &str = "1_1_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerSettings {
    /// Automatically Pause while in a Hideout.
    pub pause_in_hideout: bool,
}

impl Default for TimerSettings {
    fn default() -> Self {
        Self {
            pause_in_hideout: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PauseReason {
    /// By hotkey; only a hotkey resumes it.
    Manual,
    /// The game client restarted; resumes on entering a campaign Area.
    Login,
    /// Entered a Hideout; resumes on entering a campaign Area.
    Hideout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", content = "reason", rename_all = "camelCase")]
pub enum Phase {
    Running,
    Paused(PauseReason),
    Finished,
}

/// Elapsed time at which a Split was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitTime {
    pub real_ms: Millis,
    pub load_removed_ms: Millis,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub started_at: Millis,
    pub character: Option<String>,
    pub class: Option<String>,
    /// The Run Category this Run counts toward.
    pub category: Option<String>,
    /// Index `act - 1`.
    pub splits: [Option<SplitTime>; ACTS as usize],
    pub phase: Phase,
    /// Real time counted up to `counted_until`.
    counted_ms: Millis,
    counted_until: Millis,
    load_ms: Millis,
}

impl Run {
    fn new(at: Millis, category: Option<String>) -> Self {
        Self {
            started_at: at,
            character: None,
            class: None,
            category,
            splits: [None; ACTS as usize],
            phase: Phase::Running,
            counted_ms: 0,
            counted_until: at,
            load_ms: 0,
        }
    }

    /// Real elapsed time at `now`.
    pub fn real_ms(&self, now: Millis) -> Millis {
        match self.phase {
            Phase::Running => self.counted_ms + (now - self.counted_until).max(0),
            _ => self.counted_ms,
        }
    }

    /// Load-removed elapsed time at `now`.
    pub fn load_removed_ms(&self, now: Millis) -> Millis {
        (self.real_ms(now) - self.load_ms).max(0)
    }

    pub fn is_finished(&self) -> bool {
        self.phase == Phase::Finished
    }

    /// The act currently being played (first act without a Split).
    pub fn current_act(&self) -> Option<u8> {
        self.next_split_act()
    }

    /// Final time of a finished Run.
    pub fn final_time(&self) -> Option<SplitTime> {
        if self.is_finished() {
            self.splits[ACTS as usize - 1]
        } else {
            None
        }
    }

    fn highest_split_act(&self) -> u8 {
        self.splits
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |i| i as u8 + 1)
    }

    fn next_split_act(&self) -> Option<u8> {
        let next = self.highest_split_act() + 1;
        (next <= ACTS).then_some(next)
    }

    fn settle(&mut self, now: Millis) {
        self.counted_ms = self.real_ms(now);
        self.counted_until = now;
    }

    fn pause(&mut self, reason: PauseReason, now: Millis) {
        if self.phase == Phase::Running {
            self.settle(now);
            self.phase = Phase::Paused(reason);
        }
    }

    fn resume(&mut self, now: Millis) {
        if let Phase::Paused(_) = self.phase {
            self.counted_until = now;
            self.phase = Phase::Running;
        }
    }

    /// Record act `act`'s Split if it moves the Run forward. Returns whether it did.
    fn split(&mut self, act: u8, now: Millis) -> bool {
        if self.is_finished() || act == 0 || act <= self.highest_split_act() || act > ACTS {
            return false;
        }
        self.splits[act as usize - 1] = Some(SplitTime {
            real_ms: self.real_ms(now),
            load_removed_ms: self.load_removed_ms(now),
        });
        if act == ACTS {
            self.settle(now);
            self.phase = Phase::Finished;
        }
        true
    }

    fn undo_split(&mut self) -> Option<u8> {
        let act = self.highest_split_act();
        if act == 0 {
            return None;
        }
        self.splits[act as usize - 1] = None;
        if self.phase == Phase::Finished {
            // Continue as if the Run had never stopped.
            self.phase = Phase::Running;
        }
        Some(act)
    }

    fn on_area(&mut self, kind: AreaKind, settings: &TimerSettings, at: Millis) -> Vec<TimerEvent> {
        let mut out = Vec::new();
        if self.is_finished() {
            return out;
        }
        match (self.phase, kind) {
            (Phase::Running, AreaKind::Hideout) if settings.pause_in_hideout => {
                self.pause(PauseReason::Hideout, at)
            }
            (Phase::Paused(PauseReason::Login | PauseReason::Hideout), k) if k.resumes_run() => {
                self.resume(at)
            }
            _ => {}
        }
        let split_act = match kind {
            AreaKind::Town { act } => act - 1,
            AreaKind::Epilogue => ACTS,
            _ => 0,
        };
        if self.split(split_act, at) {
            out.push(TimerEvent::Split { act: split_act });
            if self.is_finished() {
                out.push(TimerEvent::Finished(self.clone()));
            }
        }
        out
    }
}

/// Manual timer hotkeys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Command {
    /// Start a Run now if none is in progress.
    Start,
    TogglePause,
    /// Take the next Split now.
    Split,
    UndoSplit,
    /// Abandon the current Run.
    Reset,
}

/// What changed, for the shell to persist or announce.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "camelCase")]
pub enum TimerEvent {
    Started,
    Split {
        act: u8,
    },
    SplitUndone {
        act: u8,
    },
    /// A finished Run, ready to save to history.
    Finished(Run),
    /// An unfinished Run that was reset or replaced by a new Character's Run.
    Abandoned(Run),
}

#[derive(Debug, Default)]
pub struct RunTimer {
    pub settings: TimerSettings,
    /// Run Category given to newly started Runs.
    pub category: Option<String>,
    clock: LogClock,
    current: Option<Run>,
    /// A possible new Run, started by a Twilight Strand while another Run is in
    /// progress. Becomes current if the next level up is to level 2; dropped
    /// otherwise (an existing Character just walked back there).
    candidate: Option<Run>,
}

impl RunTimer {
    pub fn new(settings: TimerSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    /// The current (possibly finished) Run.
    pub fn run(&self) -> Option<&Run> {
        self.current.as_ref()
    }

    pub fn on_record(&mut self, record: &LogRecord) -> Vec<TimerEvent> {
        let at = self.clock.instant(record);
        let mut out = Vec::new();
        match &record.event {
            LogEvent::LogOpened => {
                self.each_run(|run| run.pause(PauseReason::Login, at));
            }
            LogEvent::AreaGenerated {
                area_id,
                area_level,
            } => {
                if area_id == TWILIGHT_STRAND && *area_level == 1 {
                    out.extend(self.twilight_strand(at));
                }
                let kind = AreaKind::of(area_id);
                let settings = self.settings;
                if let Some(c) = &mut self.candidate {
                    c.on_area(kind, &settings, at);
                }
                if let Some(run) = &mut self.current {
                    out.extend(run.on_area(kind, &settings, at));
                }
            }
            LogEvent::LevelUp {
                character,
                class,
                level,
            } => out.extend(self.level_up(character, class, *level)),
            LogEvent::LoadingScreen { seconds, .. } => {
                let ms = (seconds * 1000.0).round() as Millis;
                self.each_run(|run| {
                    if run.phase == Phase::Running {
                        run.load_ms += ms;
                    }
                });
            }
            _ => {}
        }
        out
    }

    pub fn command(&mut self, command: Command, now: Millis) -> Vec<TimerEvent> {
        let mut out = Vec::new();
        match command {
            Command::Start => {
                if self.current.as_ref().is_none_or(Run::is_finished) {
                    self.current = Some(Run::new(now, self.category.clone()));
                    self.candidate = None;
                    out.push(TimerEvent::Started);
                }
            }
            Command::Reset => {
                self.candidate = None;
                if let Some(run) = self.current.take()
                    && !run.is_finished()
                {
                    out.push(TimerEvent::Abandoned(run));
                }
            }
            Command::TogglePause => {
                if let Some(run) = &mut self.current {
                    match run.phase {
                        Phase::Running => run.pause(PauseReason::Manual, now),
                        Phase::Paused(_) => run.resume(now),
                        Phase::Finished => {}
                    }
                }
            }
            Command::Split => {
                if let Some(run) = &mut self.current
                    && let Some(act) = run.next_split_act()
                    && run.split(act, now)
                {
                    out.push(TimerEvent::Split { act });
                    if run.is_finished() {
                        out.push(TimerEvent::Finished(run.clone()));
                    }
                }
            }
            Command::UndoSplit => {
                if let Some(act) = self.current.as_mut().and_then(Run::undo_split) {
                    out.push(TimerEvent::SplitUndone { act });
                }
            }
        }
        out
    }

    fn each_run(&mut self, mut f: impl FnMut(&mut Run)) {
        self.current
            .iter_mut()
            .chain(self.candidate.iter_mut())
            .for_each(&mut f);
    }

    fn twilight_strand(&mut self, at: Millis) -> Vec<TimerEvent> {
        match &self.current {
            None => {}
            Some(run) if run.is_finished() => {}
            // Same fresh Character re-entering before its first level up.
            Some(run) if run.character.is_none() => return vec![],
            Some(_) => {
                self.candidate = Some(Run::new(at, self.category.clone()));
                return vec![];
            }
        }
        self.current = Some(Run::new(at, self.category.clone()));
        vec![TimerEvent::Started]
    }

    fn level_up(&mut self, character: &str, class: &str, level: u32) -> Vec<TimerEvent> {
        let mut out = Vec::new();
        if let Some(candidate) = self.candidate.take()
            && level == 2
        {
            if let Some(old) = self.current.replace(candidate)
                && !old.is_finished()
            {
                out.push(TimerEvent::Abandoned(old));
            }
            out.push(TimerEvent::Started);
        }
        if let Some(run) = &mut self.current {
            match &run.character {
                None => {
                    run.character = Some(character.to_owned());
                    run.class = Some(class.to_owned());
                }
                // Ascending changes the class name.
                Some(name) if name == character => run.class = Some(class.to_owned()),
                Some(_) => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::parse_line;

    /// Builds log lines one second apart, starting at 01:00:00.
    struct Script {
        timer: RunTimer,
        second: i64,
        events: Vec<TimerEvent>,
    }

    impl Script {
        fn new() -> Self {
            Self {
                timer: RunTimer::new(TimerSettings::default()),
                second: 0,
                events: vec![],
            }
        }

        fn stamp(&self) -> String {
            let s = self.second;
            format!(
                "2026/11/18 {:02}:{:02}:{:02}",
                1 + s / 3600,
                (s / 60) % 60,
                s % 60
            )
        }

        fn wait(&mut self, seconds: i64) -> &mut Self {
            self.second += seconds;
            self
        }

        fn line(&mut self, msg: &str) -> &mut Self {
            let line = format!(
                "{} {} x [INFO Client 1] {msg}",
                self.stamp(),
                self.second * 1000
            );
            let events = self.timer.on_record(&parse_line(&line).unwrap());
            self.events.extend(events);
            self
        }

        fn area(&mut self, id: &str, level: u32) -> &mut Self {
            self.line(&format!(
                "Generating level {level} area \"{id}\" with seed 1"
            ))
        }

        fn level(&mut self, name: &str, level: u32) -> &mut Self {
            self.line(&format!(": {name} (Witch) is now level {level}"))
        }

        fn restart(&mut self) -> &mut Self {
            let line = format!("{} ***** LOG FILE OPENING *****", self.stamp());
            self.timer.on_record(&parse_line(&line).unwrap());
            self
        }

        fn now(&self) -> Millis {
            parse_line(&format!(
                "{} 0 x [INFO Client 1] : You have entered X.",
                self.stamp()
            ))
            .unwrap()
            .time
                * 1000
        }

        fn cmd(&mut self, c: Command) -> &mut Self {
            let now = self.now();
            let events = self.timer.command(c, now);
            self.events.extend(events);
            self
        }

        fn run(&self) -> &Run {
            self.timer.run().expect("a run")
        }

        fn real_s(&self) -> i64 {
            self.run().real_ms(self.now()) / 1000
        }

        fn split_s(&self, act: u8) -> Option<i64> {
            self.run().splits[act as usize - 1].map(|s| s.real_ms / 1000)
        }
    }

    #[test]
    fn starts_on_new_characters_twilight_strand() {
        let mut s = Script::new();
        s.area("1_1_1", 1).wait(10);
        assert_eq!(s.events, vec![TimerEvent::Started]);
        assert_eq!(s.real_s(), 10);
        s.level("Exile", 2);
        assert_eq!(s.run().character.as_deref(), Some("Exile"));
    }

    #[test]
    fn splits_on_next_acts_town_only_going_forward() {
        let mut s = Script::new();
        s.area("1_1_1", 1).wait(60).area("1_1_town", 13).wait(60);
        assert_eq!(s.split_s(1), None, "act 1's own town is not a split");
        s.area("1_2_town", 23).wait(30);
        assert_eq!(s.split_s(1), Some(120));
        s.area("1_1_town", 13).wait(30).area("1_2_town", 23);
        assert_eq!(s.split_s(1), Some(120), "backtracking does not re-split");
        assert_eq!(s.run().current_act(), Some(2));
    }

    #[test]
    fn act_6_town_splits_act_5_and_epilogue_finishes() {
        let mut s = Script::new();
        s.area("1_1_1", 1).wait(100).area("2_6_town", 50);
        assert_eq!(s.split_s(5), Some(100));
        s.wait(100).area("2_11_endgame_town", 69).wait(100);
        assert_eq!(s.split_s(10), Some(200));
        assert!(s.run().is_finished());
        assert_eq!(s.real_s(), 200, "finished timer stops");
        assert!(matches!(s.events.last(), Some(TimerEvent::Finished(_))));
    }

    #[test]
    fn hideout_pauses_until_campaign_area() {
        let mut s = Script::new();
        s.area("1_1_1", 1)
            .wait(10)
            .area("HideoutCourts", 1)
            .wait(500);
        assert_eq!(s.real_s(), 10);
        s.area("1_1_town", 13).wait(5);
        assert_eq!(s.real_s(), 15);
    }

    #[test]
    fn hideout_pause_can_be_disabled() {
        let mut s = Script::new();
        s.timer.settings.pause_in_hideout = false;
        s.area("1_1_1", 1)
            .wait(10)
            .area("HideoutCourts", 1)
            .wait(10);
        assert_eq!(s.real_s(), 20);
    }

    #[test]
    fn restart_pauses_until_campaign_area() {
        let mut s = Script::new();
        s.area("1_1_1", 1).wait(10).restart().wait(300);
        assert_eq!(s.real_s(), 10);
        s.area("1_1_2", 2).wait(5);
        assert_eq!(s.real_s(), 15);
    }

    #[test]
    fn manual_pause_ignores_areas() {
        let mut s = Script::new();
        s.area("1_1_1", 1)
            .wait(10)
            .cmd(Command::TogglePause)
            .wait(50);
        s.area("1_1_2", 2).wait(50);
        assert_eq!(s.real_s(), 10);
        s.cmd(Command::TogglePause).wait(5);
        assert_eq!(s.real_s(), 15);
    }

    #[test]
    fn loading_screens_are_removed_only_while_running() {
        let mut s = Script::new();
        s.area("1_1_1", 1).wait(10);
        s.line("[LOADING SCREEN] (The Coast) Duration = 2.5 seconds");
        s.area("HideoutCourts", 1);
        s.line("[LOADING SCREEN] (Hideout) Duration = 4 seconds");
        let run = s.run();
        assert_eq!(run.real_ms(0), 10_000);
        assert_eq!(run.load_removed_ms(0), 7_500);
    }

    #[test]
    fn returning_character_does_not_restart_run() {
        let mut s = Script::new();
        s.area("1_1_1", 1)
            .wait(5)
            .level("Exile", 2)
            .wait(100)
            .area("1_2_town", 23);
        s.wait(10)
            .area("1_1_1", 1)
            .wait(10)
            .level("Exile", 20)
            .wait(10);
        assert_eq!(s.real_s(), 135);
        assert_eq!(s.split_s(1), Some(105));
        assert_eq!(
            s.events
                .iter()
                .filter(|e| **e == TimerEvent::Started)
                .count(),
            1
        );
    }

    #[test]
    fn new_character_replaces_unfinished_run() {
        let mut s = Script::new();
        s.area("1_1_1", 1).wait(5).level("Old", 2).wait(100);
        s.area("1_1_1", 1).wait(20).level("New", 2).wait(10);
        assert_eq!(s.run().character.as_deref(), Some("New"));
        assert_eq!(s.real_s(), 30, "new Run counts from its Twilight Strand");
        assert!(s.events.iter().any(
            |e| matches!(e, TimerEvent::Abandoned(r) if r.character.as_deref() == Some("Old"))
        ));
    }

    #[test]
    fn fresh_character_reentering_strand_keeps_run() {
        let mut s = Script::new();
        s.area("1_1_1", 1).wait(30).area("1_1_1", 1).wait(30);
        assert_eq!(s.real_s(), 60);
    }

    #[test]
    fn manual_split_undo_and_reset() {
        let mut s = Script::new();
        s.cmd(Command::Start).wait(10).cmd(Command::Split).wait(10);
        assert_eq!(s.split_s(1), Some(10));
        s.cmd(Command::UndoSplit);
        assert_eq!(s.split_s(1), None);
        s.cmd(Command::Reset);
        assert!(s.timer.run().is_none());
        assert!(matches!(s.events.last(), Some(TimerEvent::Abandoned(_))));
    }

    #[test]
    fn undoing_final_split_continues_timer() {
        let mut s = Script::new();
        s.area("1_1_1", 1)
            .wait(100)
            .area("2_11_endgame_town", 69)
            .wait(20);
        s.cmd(Command::UndoSplit).wait(5);
        assert!(!s.run().is_finished());
        assert_eq!(s.real_s(), 125);
    }

    #[test]
    fn ascendancy_updates_class() {
        let mut s = Script::new();
        s.area("1_1_1", 1).level("Exile", 2);
        s.line(": Exile (Necromancer) is now level 33");
        assert_eq!(s.run().class.as_deref(), Some("Necromancer"));
    }
}
