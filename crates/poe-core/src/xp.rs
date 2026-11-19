//! Experience penalty for killing monsters of a different level (PoE1).
//!
//! From the poewiki Experience page: within a safe zone of
//! `3 + floor(level / 16)` levels there is no penalty; beyond it the
//! effective difference `d` scales experience by
//! `((level + 5) / (level + 5 + d^2.5))^1.5`. (Characters 95+ get an extra
//! penalty, irrelevant in the campaign, so it isn't modelled.)

use serde::{Deserialize, Serialize};

/// Levels either side of the character's own with no experience penalty.
pub fn safe_zone(player_level: u32) -> u32 {
    3 + player_level / 16
}

/// Experience multiplier (0..=1) for a character killing monsters of `monster_level`.
pub fn multiplier(player_level: u32, monster_level: u32) -> f64 {
    let gap = player_level.abs_diff(monster_level);
    let effective = gap.saturating_sub(safe_zone(player_level)) as f64;
    let p = f64::from(player_level) + 5.0;
    (p / (p + effective.powf(2.5))).powf(1.5)
}

/// Character levels that get full experience from monsters of `monster_level`.
pub fn full_xp_levels(monster_level: u32) -> (u32, u32) {
    let full = |l: u32| multiplier(l, monster_level) >= 0.9999;
    let lo = (1..=monster_level)
        .find(|&l| full(l))
        .unwrap_or(monster_level);
    let hi = (monster_level..=100)
        .rev()
        .find(|&l| full(l))
        .unwrap_or(monster_level);
    (lo, hi)
}

/// What the guide's level ruler draws for one zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XpView {
    pub area_name: String,
    pub monster_level: u32,
    /// The zone is where the current Step is headed, not where it happens
    /// (the current Step is in a town).
    pub upcoming: bool,
    pub player_level: Option<u32>,
    /// The character's multiplier here, if their level is known.
    pub multiplier: Option<f64>,
    /// Character levels with full experience here.
    pub full_from: u32,
    pub full_to: u32,
    /// One cell per character level around the zone, with its multiplier.
    pub cells: Vec<(u32, f64)>,
}

impl XpView {
    /// `spread` levels are shown beyond each end of the full-XP range.
    pub fn build(
        area_name: &str,
        monster_level: u32,
        upcoming: bool,
        player_level: Option<u32>,
        spread: u32,
    ) -> Self {
        let (full_from, full_to) = full_xp_levels(monster_level);
        let mut lo = full_from.saturating_sub(spread).max(1);
        let mut hi = (full_to + spread).min(100);
        // Keep the character on the ruler when they're far off.
        if let Some(p) = player_level {
            lo = lo.min(p.max(1));
            hi = hi.max(p.min(100));
        }
        Self {
            area_name: area_name.to_owned(),
            monster_level,
            upcoming,
            player_level,
            multiplier: player_level.map(|p| multiplier(p, monster_level)),
            full_from,
            full_to,
            cells: (lo..=hi)
                .map(|l| (l, multiplier(l, monster_level)))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_zone_grows_every_16_levels() {
        assert_eq!(safe_zone(1), 3);
        assert_eq!(safe_zone(15), 3);
        assert_eq!(safe_zone(16), 4);
        assert_eq!(safe_zone(24), 4);
        assert_eq!(safe_zone(64), 7);
    }

    #[test]
    fn no_penalty_inside_the_safe_zone() {
        assert_eq!(multiplier(24, 20), 1.0);
        assert_eq!(multiplier(24, 28), 1.0);
        assert!(multiplier(24, 29) < 1.0);
        assert!(multiplier(24, 19) < 1.0);
    }

    #[test]
    fn matches_the_formula() {
        // Level 30 in a level 40 zone: safe zone 4, effective difference 6.
        let p: f64 = 35.0;
        let want = (p / (p + 6f64.powf(2.5))).powf(1.5);
        assert!((multiplier(30, 40) - want).abs() < 1e-12);
        // Falls off with distance.
        assert!(multiplier(30, 45) < multiplier(30, 40));
    }

    #[test]
    fn full_xp_range_for_a_zone() {
        // Monster level 12: levels 9 (safe zone 3) through 15 (safe zone 3);
        // level 16 has safe zone 4, so 16 is also full.
        assert_eq!(full_xp_levels(12), (9, 16));
        let (lo, hi) = full_xp_levels(45);
        assert!(multiplier(lo, 45) == 1.0 && multiplier(lo - 1, 45) < 1.0);
        assert!(multiplier(hi, 45) == 1.0 && multiplier(hi + 1, 45) < 1.0);
    }

    #[test]
    fn ruler_keeps_the_character_in_view() {
        let v = XpView::build("The Ledge", 12, false, Some(40), 6);
        assert_eq!(v.cells.first().unwrap().0, 3);
        assert_eq!(v.cells.last().unwrap().0, 40);
        assert!(v.multiplier.unwrap() < 0.5);
    }
}
