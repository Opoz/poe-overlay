//! Profiles: what the app keeps per Character.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{clock::Millis, guide::GuideOptions, pob::Build, run::Run};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub class: Option<String>,
    /// Last level seen in the Client Log.
    pub level: u32,
    /// When this Character last levelled up.
    pub last_seen: Millis,
    /// This Character's current (possibly finished) Run.
    pub run: Option<Run>,
    /// Allocated passives, as Client Log passive ids.
    #[serde(default)]
    pub passives: BTreeSet<String>,
    /// Picked mastery effect per mastery (both by Client Log id).
    #[serde(default)]
    pub masteries: BTreeMap<String, u32>,
    /// League start and Bandit Choice for this Character's Guide.
    #[serde(default)]
    pub guide_options: GuideOptions,
    /// Id of the Current Step; `None` until placed in the Guide.
    #[serde(default)]
    pub guide_step: Option<u32>,
    /// The last Area this Character was seen entering (Area ID).
    #[serde(default)]
    pub last_area: Option<String>,
    /// Imported Path of Building build.
    #[serde(default)]
    pub build: Option<Build>,
    /// What was pasted to import it (code or link), for re-importing.
    #[serde(default)]
    pub build_source: Option<String>,
    /// The player picked the Bandit Choice by hand, so imports don't change it.
    #[serde(default)]
    pub bandit_set_by_hand: bool,
}

impl Profile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            class: None,
            level: 1,
            last_seen: 0,
            run: None,
            passives: BTreeSet::new(),
            masteries: BTreeMap::new(),
            guide_options: GuideOptions::default(),
            guide_step: None,
            last_area: None,
            build: None,
            build_source: None,
            bandit_set_by_hand: false,
        }
    }

    /// Attach an imported Build. Its Bandit Choice is used unless the player
    /// already picked one by hand.
    pub fn apply_build(&mut self, build: Build, source: String) {
        if !self.bandit_set_by_hand
            && let Some(bandit) = build.bandit
        {
            self.guide_options.bandit = bandit;
        }
        self.build = Some(build);
        self.build_source = Some(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guide::BanditChoice;

    fn build(bandit: Option<BanditChoice>) -> Build {
        Build {
            class: "Witch".into(),
            ascendancy: None,
            level: 90,
            bandit,
            skill_groups: vec![],
            tree_stages: vec![],
        }
    }

    #[test]
    fn import_sets_bandit_unless_picked_by_hand() {
        let mut p = Profile::new("A");
        p.apply_build(build(Some(BanditChoice::Oak)), "code".into());
        assert_eq!(p.guide_options.bandit, BanditChoice::Oak);

        p.guide_options.bandit = BanditChoice::Alira;
        p.bandit_set_by_hand = true;
        p.apply_build(build(Some(BanditChoice::Kraityn)), "code2".into());
        assert_eq!(p.guide_options.bandit, BanditChoice::Alira);
        assert_eq!(p.build_source.as_deref(), Some("code2"));
    }
}
