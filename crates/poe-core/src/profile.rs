//! Profiles: what the app keeps per Character.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{clock::Millis, run::Run};

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
        }
    }
}
