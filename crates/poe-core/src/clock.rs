//! Turning Log Records into millisecond instants.
//!
//! Wall-clock stamps only have one-second resolution, while the client's
//! uptime tick has milliseconds but resets on restart. [`LogClock`] anchors the
//! tick to the wall clock once per client session, giving ms-precision
//! instants on the same scale as the app's own wall clock, which manual
//! hotkey commands use.

use crate::log::LogRecord;

/// Milliseconds on the local wall-clock scale (`LocalSeconds * 1000`).
pub type Millis = i64;

/// If tick-derived time drifts this far from the wall clock, re-anchor.
const MAX_DRIFT_MS: Millis = 2_000;

#[derive(Debug, Default, Clone)]
pub struct LogClock {
    /// `wall_ms - tick_ms` for the current client session.
    anchor: Option<Millis>,
}

impl LogClock {
    pub fn new() -> Self {
        Self::default()
    }

    /// The instant a record happened at.
    pub fn instant(&mut self, record: &LogRecord) -> Millis {
        let wall = record.time * 1000;
        let Some(tick) = record.tick_ms else {
            // Session start: next ticked record re-anchors.
            self.anchor = None;
            return wall;
        };
        let tick = tick as Millis;
        match self.anchor {
            Some(anchor) if (anchor + tick - wall).abs() <= MAX_DRIFT_MS => anchor + tick,
            _ => {
                self.anchor = Some(wall - tick);
                wall
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::parse_line;

    fn at(clock: &mut LogClock, line: &str) -> Millis {
        clock.instant(&parse_line(line).unwrap())
    }

    #[test]
    fn uses_tick_for_sub_second_precision() {
        let mut c = LogClock::new();
        let a = at(
            &mut c,
            "2026/11/18 01:00:00 10000 x [INFO Client 1] : You have entered A.",
        );
        let b = at(
            &mut c,
            "2026/11/18 01:00:00 10750 x [INFO Client 1] : You have entered B.",
        );
        assert_eq!(b - a, 750);
    }

    #[test]
    fn reanchors_after_restart() {
        let mut c = LogClock::new();
        at(
            &mut c,
            "2026/11/18 01:00:00 99999999 x [INFO Client 1] : You have entered A.",
        );
        at(&mut c, "2026/11/18 02:00:00 ***** LOG FILE OPENING *****");
        let b = at(
            &mut c,
            "2026/11/18 02:00:05 500 x [INFO Client 2] : You have entered B.",
        );
        let wall = parse_line("2026/11/18 02:00:05 500 x [INFO Client 2] : You have entered B.")
            .unwrap()
            .time
            * 1000;
        assert_eq!(b, wall);
    }

    #[test]
    fn reanchors_on_drift_without_log_opening() {
        let mut c = LogClock::new();
        at(
            &mut c,
            "2026/11/18 01:00:00 1000 x [INFO Client 1] : You have entered A.",
        );
        // tick reset (restart line missing): falls back to the wall clock
        let b = at(
            &mut c,
            "2026/11/18 01:10:00 2000 x [INFO Client 1] : You have entered B.",
        );
        let a = at(
            &mut c,
            "2026/11/18 01:00:00 1000 x [INFO Client 1] : You have entered A.",
        );
        assert_eq!(b - a, 600_000);
    }
}
