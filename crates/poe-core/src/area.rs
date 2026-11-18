//! Classifying Areas by Area ID. Names are not unique (two "The Twilight
//! Strand"s), so everything keys off the ID.

use serde::{Deserialize, Serialize};

/// Number of acts in the PoE1 campaign.
pub const ACTS: u8 = 10;

/// What kind of place an Area is, as far as Runs care.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AreaKind {
    /// A campaign Area in act 1..=10 (not a Town).
    Campaign {
        act: u8,
    },
    /// An act's Town.
    Town {
        act: u8,
    },
    /// The epilogue Town reached after defeating Kitava; ends a Run.
    Epilogue,
    Hideout,
    Labyrinth,
    /// Anything else (maps, league content, unknown).
    Other,
}

impl AreaKind {
    pub fn of(area_id: &str) -> Self {
        if area_id == "2_11_endgame_town" {
            return Self::Epilogue;
        }
        if area_id.starts_with("Hideout") {
            return Self::Hideout;
        }
        if area_id.contains("Labyrinth") {
            return Self::Labyrinth;
        }
        campaign(area_id).unwrap_or(Self::Other)
    }

    /// Areas where an automatically paused Run resumes.
    pub fn resumes_run(self) -> bool {
        matches!(
            self,
            Self::Campaign { .. } | Self::Town { .. } | Self::Epilogue | Self::Labyrinth
        )
    }
}

/// `1_<act>_<rest>` for acts 1-5, `2_<act>_<rest>` for acts 6-10, where
/// `<rest>` is `town` or starts with a digit.
fn campaign(area_id: &str) -> Option<AreaKind> {
    let mut parts = area_id.splitn(3, '_');
    let part = parts.next()?;
    let act: u8 = parts.next()?.parse().ok()?;
    let rest = parts.next()?;
    let valid_act = match part {
        "1" => (1..=5).contains(&act),
        "2" => (6..=ACTS).contains(&act),
        _ => false,
    };
    if !valid_act {
        return None;
    }
    if rest == "town" {
        Some(AreaKind::Town { act })
    } else if rest.starts_with(|c: char| c.is_ascii_digit()) {
        Some(AreaKind::Campaign { act })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::AreaKind::{self, *};

    #[test]
    fn classifies_area_ids() {
        for (id, kind) in [
            ("1_1_1", Campaign { act: 1 }),
            ("1_1_11_2", Campaign { act: 1 }),
            ("1_1_town", Town { act: 1 }),
            ("1_5_town", Town { act: 5 }),
            ("2_6_1", Campaign { act: 6 }),
            ("2_6_7_1", Campaign { act: 6 }),
            ("2_10_town", Town { act: 10 }),
            ("2_11_endgame_town", Epilogue),
            ("HideoutCourts", Hideout),
            ("1_Labyrinth_OH_quad_#", Labyrinth),
            ("Labyrinth_Airlock", Labyrinth),
            ("EndGame_Labyrinth_boss_3", Labyrinth),
            ("MapWorldsBeach", Other),
            ("1_6_1", Other),
            ("2_3_1", Other),
            ("garbage", Other),
        ] {
            assert_eq!(AreaKind::of(id), kind, "{id}");
        }
    }
}
