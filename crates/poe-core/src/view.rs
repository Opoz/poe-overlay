//! Render-ready snapshots for the frontend. The frontend only draws these
//! (ADR 0003); every comparison is worked out here.

use serde::{Deserialize, Serialize};

use crate::{
    area::ACTS,
    clock::Millis,
    pb::{Comparison, Metric},
    run::{Phase, Run, SplitTime},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerView {
    /// `None` when no Run is in progress.
    pub phase: Option<Phase>,
    pub character: Option<String>,
    pub class: Option<String>,
    pub category: Option<String>,
    pub current_act: Option<u8>,
    /// Elapsed times at snapshot time. While running, the frontend keeps
    /// counting from these with its own clock.
    pub real_ms: Millis,
    pub load_removed_ms: Millis,
    pub splits: Vec<SplitRow>,
    pub best_possible_real_ms: Option<Millis>,
    pub best_possible_load_removed_ms: Option<Millis>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitRow {
    pub act: u8,
    pub time: Option<SplitTime>,
    /// The Personal Best's cumulative times for this act.
    pub pb_real_ms: Option<Millis>,
    pub pb_load_removed_ms: Option<Millis>,
    /// Live minus Personal Best (negative = ahead).
    pub delta_real_ms: Option<Millis>,
    pub delta_load_removed_ms: Option<Millis>,
    /// This act was faster than any previous attempt at it (real time).
    pub best_segment: bool,
}

impl TimerView {
    /// `history` is every saved Run in the current Run's category (or the
    /// selected category when idle), not including the current Run.
    pub fn build<'a>(
        run: Option<&Run>,
        history: impl IntoIterator<Item = &'a Run> + Clone,
        now: Millis,
    ) -> Self {
        let real = Comparison::from_runs(Metric::Real, history.clone());
        let load_removed = Comparison::from_runs(Metric::LoadRemoved, history);

        let splits = (1..=ACTS)
            .map(|act| {
                let i = act as usize - 1;
                let time = run.and_then(|r| r.splits[i]);
                SplitRow {
                    act,
                    time,
                    pb_real_ms: real.personal_best.and_then(|pb| pb[i]),
                    pb_load_removed_ms: load_removed.personal_best.and_then(|pb| pb[i]),
                    delta_real_ms: time.and_then(|t| real.delta(act, t)),
                    delta_load_removed_ms: time.and_then(|t| load_removed.delta(act, t)),
                    best_segment: run.is_some_and(|r| real.is_best_segment(r, act)),
                }
            })
            .collect();

        Self {
            phase: run.map(|r| r.phase),
            character: run.and_then(|r| r.character.clone()),
            class: run.and_then(|r| r.class.clone()),
            category: run.and_then(|r| r.category.clone()),
            current_act: run.and_then(Run::current_act),
            real_ms: run.map_or(0, |r| r.real_ms(now)),
            load_removed_ms: run.map_or(0, |r| r.load_removed_ms(now)),
            splits,
            best_possible_real_ms: real.best_possible_time(),
            best_possible_load_removed_ms: load_removed.best_possible_time(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{Command, RunTimer, TimerSettings};

    #[test]
    fn idle_view_shows_personal_best() {
        let mut t = RunTimer::new(TimerSettings::default());
        t.command(Command::Start, 0);
        for act in 1..=10 {
            t.command(Command::Split, act * 60_000);
        }
        let history = vec![t.run().unwrap().clone()];
        let view = TimerView::build(None, &history, 0);
        assert_eq!(view.phase, None);
        assert_eq!(view.splits.len(), 10);
        assert_eq!(view.splits[9].pb_real_ms, Some(600_000));
        assert_eq!(view.best_possible_real_ms, Some(600_000));
    }

    #[test]
    fn live_view_has_deltas() {
        let mut pb = RunTimer::new(TimerSettings::default());
        pb.command(Command::Start, 0);
        for act in 1..=10 {
            pb.command(Command::Split, act * 60_000);
        }
        let history = vec![pb.run().unwrap().clone()];

        let mut live = RunTimer::new(TimerSettings::default());
        live.command(Command::Start, 0);
        live.command(Command::Split, 50_000);
        let view = TimerView::build(live.run(), &history, 70_000);
        assert_eq!(view.real_ms, 70_000);
        assert_eq!(view.current_act, Some(2));
        assert_eq!(view.splits[0].delta_real_ms, Some(-10_000));
        assert!(view.splits[0].best_segment);
        assert_eq!(view.splits[1].delta_real_ms, None);
    }
}
