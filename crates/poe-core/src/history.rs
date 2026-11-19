//! Past Runs for the History page: per-act segments, which Run is the
//! Personal Best in its Run Category, and how each act compares with it.

use serde::Serialize;

use crate::{
    area::ACTS,
    clock::Millis,
    pb::{Comparison, Metric, segments},
    run::Run,
};

/// One past Run.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRun {
    /// Identifies the Run (for deleting it).
    pub started_at: Millis,
    pub character: Option<String>,
    pub class: Option<String>,
    pub category: String,
    pub finished: bool,
    /// The last act with a Split, 0 if none.
    pub acts_done: u8,
    /// Real time at the last Split.
    pub time_ms: Option<Millis>,
    /// Duration of each act (real time), index `act - 1`.
    pub segments: [Option<Millis>; ACTS as usize],
    /// The Personal Best of its Run Category.
    pub personal_best: bool,
    /// Act segments that are the fastest in its Run Category.
    pub best_segments: [bool; ACTS as usize],
    /// Each Split minus the Personal Best's (negative = ahead), cumulative.
    pub deltas: [Option<Millis>; ACTS as usize],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryView {
    /// Run Categories with at least one Run, most recently used first.
    pub categories: Vec<String>,
    /// Newest first.
    pub runs: Vec<HistoryRun>,
    /// The slowest act segment across all Runs, for scaling bars.
    pub longest_ms: Millis,
}

const DEFAULT_CATEGORY: &str = "Default";

impl HistoryView {
    pub fn build(runs: &[Run]) -> Self {
        let category = |r: &Run| {
            r.category
                .clone()
                .unwrap_or_else(|| DEFAULT_CATEGORY.to_owned())
        };
        let mut categories: Vec<String> = Vec::new();
        for r in runs.iter().rev() {
            let c = category(r);
            if !categories.contains(&c) {
                categories.push(c);
            }
        }

        let mut out = Vec::new();
        for cat in &categories {
            let in_cat: Vec<&Run> = runs.iter().filter(|r| &category(r) == cat).collect();
            let cmp = Comparison::from_runs(Metric::Real, in_cat.iter().copied());
            let pb_started = in_cat
                .iter()
                .filter_map(|r| Some((r.final_time()?.real_ms, r.started_at)))
                .min()
                .map(|(_, s)| s);
            for r in in_cat {
                let times = r.splits.map(|s| s.map(|s| s.real_ms));
                let segs = segments(&times);
                let acts_done = r
                    .splits
                    .iter()
                    .rposition(Option::is_some)
                    .map_or(0, |i| i as u8 + 1);
                let mut best = [false; ACTS as usize];
                let mut deltas = [None; ACTS as usize];
                for i in 0..ACTS as usize {
                    best[i] = segs[i].is_some() && segs[i] == cmp.best_segments[i];
                    deltas[i] = match (times[i], cmp.personal_best.and_then(|p| p[i])) {
                        (Some(t), Some(p)) => Some(t - p),
                        _ => None,
                    };
                }
                out.push(HistoryRun {
                    started_at: r.started_at,
                    character: r.character.clone(),
                    class: r.class.clone(),
                    category: cat.clone(),
                    finished: r.is_finished(),
                    acts_done,
                    time_ms: acts_done.checked_sub(1).and_then(|i| times[i as usize]),
                    segments: segs,
                    personal_best: Some(r.started_at) == pb_started,
                    best_segments: best,
                    deltas,
                });
            }
        }
        out.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        let longest_ms = out
            .iter()
            .flat_map(|r| r.segments.iter().flatten())
            .copied()
            .max()
            .unwrap_or(0);
        Self {
            categories,
            runs: out,
            longest_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{Command, RunTimer, TimerSettings};

    fn run(category: &str, start: i64, splits_s: &[i64]) -> Run {
        let mut timer = RunTimer::new(TimerSettings::default());
        timer.category = Some(category.into());
        timer.command(Command::Start, start * 1000);
        for s in splits_s {
            timer.command(Command::Split, (start + s) * 1000);
        }
        timer.run().unwrap().clone()
    }

    #[test]
    fn marks_the_personal_best_per_category() {
        let full = |step: i64| (1..=10).map(|i| i * step).collect::<Vec<_>>();
        let runs = vec![
            run("SSF", 0, &full(100)),
            run("SSF", 10_000, &full(90)),
            run("Trade", 20_000, &full(200)),
            // Unfinished but with a fast first act.
            run("SSF", 30_000, &[50]),
        ];
        let v = HistoryView::build(&runs);
        assert_eq!(v.categories, ["SSF", "Trade"]);
        assert_eq!(v.runs[0].started_at, 30_000_000, "newest first");
        let pbs: Vec<i64> = v
            .runs
            .iter()
            .filter(|r| r.personal_best)
            .map(|r| r.started_at)
            .collect();
        assert_eq!(pbs, [20_000_000, 10_000_000]);
        let unfinished = &v.runs[0];
        assert_eq!(unfinished.acts_done, 1);
        assert!(unfinished.best_segments[0]);
        assert_eq!(unfinished.deltas[0], Some(50_000 - 90_000));
        let slow = v.runs.iter().find(|r| r.started_at == 0).unwrap();
        assert_eq!(slow.deltas[9], Some(100_000));
        assert_eq!(v.longest_ms, 200_000);
    }
}
