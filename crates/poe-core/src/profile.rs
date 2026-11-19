//! Profiles: what the app keeps per Character.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{clock::Millis, guide::GuideOptions, run::Run};

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
        }
    }
}
