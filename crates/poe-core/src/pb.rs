//! Personal Best and Best Possible Time, computed from finished Runs.
//!
//! Each timing metric is compared independently: the load-removed Personal
//! Best can be a different Run from the real-time one.

use serde::{Deserialize, Serialize};

use crate::{
    area::ACTS,
    clock::Millis,
    run::{Run, SplitTime},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Metric {
    Real,
    LoadRemoved,
}

impl Metric {
    fn of(self, t: SplitTime) -> Millis {
        match self {
            Self::Real => t.real_ms,
            Self::LoadRemoved => t.load_removed_ms,
        }
    }
}

type Times = [Option<Millis>; ACTS as usize];

/// What live Splits are compared against, for one metric in one Run Category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub metric: Metric,
    /// Cumulative Split times of the Personal Best Run.
    pub personal_best: Option<Times>,
    /// Fastest duration of each act across all Runs (segment, not cumulative).
    pub best_segments: Times,
}

impl Comparison {
    /// Build from the Runs of one Run Category. Unfinished Runs only
    /// contribute best segments.
    pub fn from_runs<'a>(metric: Metric, runs: impl IntoIterator<Item = &'a Run>) -> Self {
        let mut personal_best: Option<(Millis, Times)> = None;
        let mut best_segments: Times = [None; ACTS as usize];

        for run in runs {
            let times: Times = run.splits.map(|s| s.map(|s| metric.of(s)));
            for (act, segment) in segments(&times).into_iter().enumerate() {
                if let Some(seg) = segment {
                    let best = &mut best_segments[act];
                    *best = Some(best.map_or(seg, |b| b.min(seg)));
                }
            }
            if let Some(final_time) = run.final_time().map(|t| metric.of(t))
                && personal_best
                    .as_ref()
                    .is_none_or(|(pb, _)| final_time < *pb)
            {
                personal_best = Some((final_time, times));
            }
        }

        Self {
            metric,
            personal_best: personal_best.map(|(_, t)| t),
            best_segments,
        }
    }

    /// Sum of best segments, once every act has one.
    pub fn best_possible_time(&self) -> Option<Millis> {
        self.best_segments.iter().copied().sum()
    }

    /// Live Split minus the Personal Best's Split for `act` (negative = ahead).
    pub fn delta(&self, act: u8, live: SplitTime) -> Option<Millis> {
        let pb = self.personal_best?[act.checked_sub(1)? as usize]?;
        Some(self.metric.of(live) - pb)
    }

    /// Whether the live segment ending at `act` beat the best segment.
    pub fn is_best_segment(&self, run: &Run, act: u8) -> bool {
        let i = match act.checked_sub(1) {
            Some(i) if i < ACTS => i as usize,
            _ => return false,
        };
        let times: Times = run.splits.map(|s| s.map(|s| self.metric.of(s)));
        match (segments(&times)[i], self.best_segments[i]) {
            (Some(live), Some(best)) => live < best,
            (Some(_), None) => true,
            _ => false,
        }
    }
}

/// Per-act durations; an act's segment is only known if the previous Split is.
fn segments(times: &Times) -> Times {
    let mut out = [None; ACTS as usize];
    let mut prev = Some(0);
    for (i, t) in times.iter().enumerate() {
        out[i] = match (prev, t) {
            (Some(p), Some(t)) => Some(t - p),
            _ => None,
        };
        prev = *t;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{Command, RunTimer, TimerSettings};

    /// A Run with manual splits at the given cumulative seconds.
    fn run(splits_s: &[i64]) -> Run {
        let mut timer = RunTimer::new(TimerSettings::default());
        timer.command(Command::Start, 0);
        for s in splits_s {
            timer.command(Command::Split, s * 1000);
        }
        timer.run().unwrap().clone()
    }

    fn full(step_s: i64) -> Run {
        run(&(1..=10).map(|i| i * step_s).collect::<Vec<_>>())
    }

    #[test]
    fn personal_best_is_fastest_finished_run() {
        let runs = [full(100), full(90), run(&[10, 20])];
        let c = Comparison::from_runs(Metric::Real, &runs);
        assert_eq!(c.personal_best.unwrap()[9], Some(900_000));
    }

    #[test]
    fn best_segments_include_unfinished_runs() {
        let runs = [full(100), run(&[10, 20])];
        let c = Comparison::from_runs(Metric::Real, &runs);
        assert_eq!(c.best_segments[0], Some(10_000));
        assert_eq!(c.best_segments[1], Some(10_000));
        assert_eq!(c.best_segments[2], Some(100_000));
        assert_eq!(c.best_possible_time(), Some(20_000 + 8 * 100_000));
    }

    #[test]
    fn best_possible_needs_every_act() {
        let c = Comparison::from_runs(Metric::Real, &[run(&[10, 20])]);
        assert_eq!(c.best_possible_time(), None);
    }

    #[test]
    fn delta_against_personal_best() {
        let c = Comparison::from_runs(Metric::Real, &[full(100)]);
        let live = SplitTime {
            real_ms: 95_000,
            load_removed_ms: 90_000,
        };
        assert_eq!(c.delta(1, live), Some(-5_000));
        assert_eq!(c.delta(0, live), None);
        assert_eq!(
            Comparison::from_runs(Metric::Real, &[]).delta(1, live),
            None
        );
    }

    #[test]
    fn detects_best_segment() {
        let c = Comparison::from_runs(Metric::Real, &[full(100)]);
        assert!(c.is_best_segment(&run(&[50]), 1));
        assert!(!c.is_best_segment(&run(&[150]), 1));
        assert!(!c.is_best_segment(&run(&[50]), 2));
    }
}
