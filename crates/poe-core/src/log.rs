//! Client Log parsing: turns raw `Client.txt` lines into [`LogEvent`]s.
//!
//! The log is not a stable API (ADR 0002), so parsing is strict about the
//! shapes it recognises and silently ignores everything else.
//!
//! Player-typed chat can never produce an event: system messages are only
//! recognised after the exact `] : ` marker, which chat lines never contain
//! (chat is `] #Name: …`, `] @From Name: …`, `] Name: …`).

use std::io::BufRead;

use serde::{Deserialize, Serialize};

/// Seconds since the Unix epoch, taken from the line's local wall-clock stamp.
///
/// The game writes local time without a zone, so this is "local seconds",
/// only meaningful for ordering and differences.
pub type LocalSeconds = i64;

/// One parsed Client Log line that carried a [`LogEvent`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
    pub time: LocalSeconds,
    /// The client's millisecond uptime counter. Precise within one client
    /// session, resets when the game restarts. `None` on `LogOpened` lines.
    pub tick_ms: Option<u64>,
    pub event: LogEvent,
}

/// A Log Event: one meaningful fact from the Client Log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LogEvent {
    /// The game client started (a new session; precedes login).
    LogOpened,
    /// An Area instance was generated. Precedes [`LogEvent::AreaEntered`] and
    /// is the authoritative Area signal (names are not unique, IDs are).
    AreaGenerated {
        area_id: String,
        area_level: u32,
    },
    /// "You have entered X." Display name only.
    AreaEntered {
        name: String,
    },
    /// A Character reached a new level. The only line that names the Character.
    LevelUp {
        character: String,
        class: String,
        level: u32,
    },
    /// A Character died.
    Slain {
        character: String,
    },
    PassiveAllocated {
        id: String,
        name: String,
    },
    PassiveUnallocated {
        id: String,
        name: String,
    },
    /// A loading screen finished, with its measured duration.
    LoadingScreen {
        area_name: String,
        seconds: f64,
    },
}

/// Parse one raw line. Returns `None` for lines that carry no event.
pub fn parse_line(line: &str) -> Option<LogRecord> {
    let line = line.trim_end_matches(['\r', '\n']);
    let (time, rest) = parse_timestamp(line)?;

    if rest.trim() == "***** LOG FILE OPENING *****" {
        return Some(LogRecord {
            time,
            tick_ms: None,
            event: LogEvent::LogOpened,
        });
    }

    // "<tick> <hash> [LEVEL Client <pid>] <message>"
    let (tick, rest) = rest.split_once(' ')?;
    let tick_ms = tick.parse().ok()?;
    let (_hash, rest) = rest.split_once(' ')?;
    let rest = rest.strip_prefix('[')?;
    let (_tag, message) = rest.split_once("] ")?;

    let event = parse_message(message)?;
    Some(LogRecord {
        time,
        tick_ms: Some(tick_ms),
        event,
    })
}

fn parse_message(msg: &str) -> Option<LogEvent> {
    if let Some(system) = msg.strip_prefix(": ") {
        return parse_system_message(system);
    }
    if let Some(rest) = msg.strip_prefix("Generating level ") {
        // `47 area "2_6_7_1" with seed 2345974639`
        let (level, rest) = rest.split_once(" area \"")?;
        let (area_id, _) = rest.split_once('"')?;
        return Some(LogEvent::AreaGenerated {
            area_id: area_id.to_owned(),
            area_level: level.parse().ok()?,
        });
    }
    if let Some(rest) = msg.strip_prefix("Successfully allocated passive skill id: ") {
        let (id, name) = parse_passive(rest)?;
        return Some(LogEvent::PassiveAllocated { id, name });
    }
    if let Some(rest) = msg.strip_prefix("Successfully unallocated passive skill id: ") {
        let (id, name) = parse_passive(rest)?;
        return Some(LogEvent::PassiveUnallocated { id, name });
    }
    if let Some(rest) = msg.strip_prefix("[LOADING SCREEN] (") {
        // `The Twilight Strand) Duration = 0.773652 seconds`
        let (area_name, rest) = rest.rsplit_once(") Duration = ")?;
        let seconds = rest.strip_suffix(" seconds")?.parse().ok()?;
        return Some(LogEvent::LoadingScreen {
            area_name: area_name.to_owned(),
            seconds,
        });
    }
    None
}

fn parse_system_message(msg: &str) -> Option<LogEvent> {
    if let Some(name) = msg
        .strip_prefix("You have entered ")
        .and_then(|s| s.strip_suffix('.'))
    {
        return Some(LogEvent::AreaEntered {
            name: name.to_owned(),
        });
    }
    if let Some((who, level)) = msg.rsplit_once(") is now level ") {
        // `Name (Class` + `46`
        let (character, class) = who.rsplit_once(" (")?;
        return Some(LogEvent::LevelUp {
            character: character.to_owned(),
            class: class.to_owned(),
            level: level.parse().ok()?,
        });
    }
    if let Some(character) = msg.strip_suffix(" has been slain.") {
        // Character names never contain spaces; this rejects e.g. NPC deaths.
        if !character.is_empty() && !character.contains(' ') {
            return Some(LogEvent::Slain {
                character: character.to_owned(),
            });
        }
    }
    None
}

/// `mana_spell_block2610, name: Mana and Spell Block`
fn parse_passive(rest: &str) -> Option<(String, String)> {
    let (id, name) = rest.split_once(", name: ")?;
    Some((id.to_owned(), name.to_owned()))
}

/// `2026/11/18 01:36:37 <rest>`
fn parse_timestamp(line: &str) -> Option<(LocalSeconds, &str)> {
    let stamp = line.get(..19)?;
    let rest = line.get(20..)?;
    let b = stamp.as_bytes();
    if b[4] != b'/' || b[7] != b'/' || b[10] != b' ' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| stamp.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    Some((
        days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s,
        rest,
    ))
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Replay: parse every event from a recorded Client Log, in order.
///
/// Invalid UTF-8 lines (the log occasionally contains them) are skipped.
pub fn replay<R: BufRead>(reader: R) -> impl Iterator<Item = LogRecord> {
    reader
        .split(b'\n')
        .map_while(Result::ok)
        .filter_map(|bytes| parse_line(std::str::from_utf8(&bytes).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(line: &str) -> Option<LogEvent> {
        parse_line(line).map(|r| r.event)
    }

    #[test]
    fn area_generated() {
        let r = parse_line(
            "2026/11/18 01:36:37 29599968 1186a803 [DEBUG Client 4976] Generating level 45 area \"2_6_1\" with seed 3344171319",
        )
        .unwrap();
        assert_eq!(r.tick_ms, Some(29_599_968));
        assert_eq!(
            r.event,
            LogEvent::AreaGenerated {
                area_id: "2_6_1".into(),
                area_level: 45
            }
        );
    }

    #[test]
    fn area_entered() {
        assert_eq!(
            event(
                "2026/11/18 01:36:37 29600468 cffb06be [INFO Client 4976] : You have entered Lioneye's Watch."
            ),
            Some(LogEvent::AreaEntered {
                name: "Lioneye's Watch".into()
            })
        );
    }

    #[test]
    fn level_up() {
        assert_eq!(
            event(
                "2026/11/18 01:36:51 29613843 cffb06be [INFO Client 4976] : TestExile (Necromancer) is now level 46"
            ),
            Some(LogEvent::LevelUp {
                character: "TestExile".into(),
                class: "Necromancer".into(),
                level: 46
            })
        );
    }

    #[test]
    fn slain() {
        assert_eq!(
            event("2026/11/18 01:00:00 1 abc [INFO Client 1] : TestExile has been slain."),
            Some(LogEvent::Slain {
                character: "TestExile".into()
            })
        );
    }

    #[test]
    fn passive_allocation() {
        assert_eq!(
            event(
                "2026/11/18 01:37:58 29680640 fb7ae248 [INFO Client 4976] Successfully allocated passive skill id: mana_spell_block2610, name: Mana and Spell Block"
            ),
            Some(LogEvent::PassiveAllocated {
                id: "mana_spell_block2610".into(),
                name: "Mana and Spell Block".into()
            })
        );
        assert!(matches!(
            event(
                "2024/07/30 23:30:21 19519984 fb7ab8c1 [INFO Client 21436] Successfully unallocated passive skill id: versatility1146, name: Versatility"
            ),
            Some(LogEvent::PassiveUnallocated { .. })
        ));
    }

    #[test]
    fn loading_screen() {
        assert_eq!(
            event(
                "2026/11/18 01:36:37 29600609 a15e95e2 [INFO Client 4976] [LOADING SCREEN] (The Twilight Strand) Duration = 0.773652 seconds"
            ),
            Some(LogEvent::LoadingScreen {
                area_name: "The Twilight Strand".into(),
                seconds: 0.773652
            })
        );
    }

    #[test]
    fn log_opened() {
        let r = parse_line("2026/11/17 20:54:27 ***** LOG FILE OPENING *****").unwrap();
        assert_eq!(r.event, LogEvent::LogOpened);
        assert_eq!(r.tick_ms, None);
    }

    #[test]
    fn chat_cannot_spoof_events() {
        for line in [
            "2026/11/18 01:00:00 1 abc [INFO Client 1] #Troll: You have entered The Coast.",
            "2026/11/18 01:00:00 1 abc [INFO Client 1] @From Troll: Troll (Witch) is now level 99",
            "2026/11/18 01:00:00 1 abc [INFO Client 1] Troll: : You have entered The Coast.",
            "2026/11/18 01:00:00 1 abc [INFO Client 1] Cal, the Halfwit: Eh?",
            "2026/11/18 01:00:00 1 abc [INFO Client 1] : Hillock, the Blood-Stained has been slain.",
        ] {
            assert_eq!(event(line), None, "{line}");
        }
    }

    #[test]
    fn ignores_noise_and_garbage() {
        for line in [
            "",
            "garbage",
            "2026/11/18 01:36:22 29584750 f22b6a02 [INFO Client 4976] Tile hash: 2140406439",
            "2026/13/18 01:36:22 29584750 f22b6a02 [INFO Client 4976] : You have entered X.",
            "2026/11/18 01:36:22 notatick f22b6a02 [INFO Client 4976] : You have entered X.",
        ] {
            assert_eq!(event(line), None, "{line:?}");
        }
    }

    #[test]
    fn timestamps_are_ordered_seconds() {
        let a =
            parse_line("2026/11/17 23:59:59 1 a [INFO Client 1] : You have entered X.").unwrap();
        let b =
            parse_line("2026/11/18 00:00:01 2 a [INFO Client 1] : You have entered X.").unwrap();
        assert_eq!(b.time - a.time, 2);
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    }

    #[test]
    fn handles_crlf() {
        assert!(
            event("2026/11/18 01:00:00 1 abc [INFO Client 1] : You have entered The Coast.\r")
                .is_some()
        );
    }
}
