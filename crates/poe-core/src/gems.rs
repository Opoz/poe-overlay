//! Gems for an imported Build: which quest reward to take and what to buy at
//! each hand-in along the Guide, the Vendor Regex, and the Skill Groups for the
//! Character's level.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    guide::{Fragment, GemPick, Guide, QuestInfo, bundled_quests},
    pob::{Build, SkillGroup},
};

const GEMS_JSON: &str = include_str!("../data/gems.json");

/// The in-game search box takes about this many characters.
pub const SEARCH_LIMIT: usize = 250;

#[derive(Debug, Clone, Deserialize)]
pub struct GemInfo {
    pub name: String,
    /// r / g / b / w
    pub colour: String,
    /// Required character level.
    pub level: u32,
    #[serde(default)]
    pub support: bool,
}

#[derive(Debug, Clone)]
pub struct GemData {
    gems: HashMap<String, GemInfo>,
    quests: HashMap<String, QuestInfo>,
}

impl GemData {
    pub fn bundled() -> Self {
        Self {
            gems: serde_json::from_str(GEMS_JSON).expect("bundled gems.json is valid"),
            quests: bundled_quests(),
        }
    }

    pub fn gem(&self, id: &str) -> Option<&GemInfo> {
        self.gems.get(id)
    }

    fn pick(&self, id: &str, npc: Option<&str>) -> GemPick {
        let info = self.gem(id);
        GemPick {
            gem_id: id.to_owned(),
            name: info.map_or_else(|| id.to_owned(), |g| g.name.clone()),
            colour: info.map_or_else(|| "w".to_owned(), |g| g.colour.clone()),
            npc: npc.map(str::to_owned),
        }
    }
}

/// Where each of the Build's gems comes from along the Guide.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GemPlan {
    /// Keyed by `"{step id}:{quest id}"`.
    pub picks: HashMap<String, OfferPicks>,
    /// Gems the Build uses that no quest or vendor along the route provides
    /// for this class (drop-only, or unknown to the data).
    pub unobtainable: Vec<GemPick>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferPicks {
    /// One per reward offer at most.
    pub take: Vec<GemPick>,
    pub buy: Vec<GemPick>,
}

fn key(step: u32, quest: &str) -> String {
    format!("{step}:{quest}")
}

/// An empty class list in the data means every class.
fn for_class(classes: &[String], class: &str) -> bool {
    classes.is_empty() || classes.iter().any(|c| c == class)
}

/// The Build's gems in the order it lists them (enabled groups first), once each.
pub fn build_gems(build: &Build) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut groups: Vec<&SkillGroup> = build.skill_groups.iter().collect();
    groups.sort_by_key(|g| !g.enabled);
    groups
        .into_iter()
        .flat_map(|g| &g.gems)
        .filter_map(|g| g.gem_id.clone())
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

impl GemPlan {
    /// Walk the Guide's hand-ins in order. At each one, take the first
    /// still-needed gem (in Build order) this class may pick as the reward,
    /// then buy every other still-needed gem the vendor now sells to it.
    pub fn build(data: &GemData, guide: &Guide, build: &Build) -> Self {
        let class = build.class.as_str();
        let needed = build_gems(build);
        let mut have: HashSet<String> = HashSet::new();
        let mut picks = HashMap::new();

        for step in &guide.steps {
            for part in step.lines.iter().flat_map(|l| &l.parts) {
                let Fragment::Quest {
                    quest_id,
                    offer_ids,
                    ..
                } = part
                else {
                    continue;
                };
                let Some(quest) = data.quests.get(quest_id) else {
                    continue;
                };
                let offers: Vec<_> = offer_ids
                    .iter()
                    .filter_map(|id| quest.offers.get(id))
                    .collect();
                let mut found = OfferPicks::default();
                // Each offer is its own reward pick.
                for offer in &offers {
                    let take = needed.iter().filter(|g| !have.contains(*g)).find(|g| {
                        offer
                            .quest_gems
                            .get(*g)
                            .is_some_and(|classes| for_class(classes, class))
                    });
                    if let Some(g) = take {
                        have.insert(g.clone());
                        found.take.push(data.pick(g, None));
                    }
                }
                for offer in &offers {
                    for g in needed.iter().filter(|g| !have.contains(*g)) {
                        if let Some(v) = offer.vendor_gems.get(g)
                            && for_class(&v.classes, class)
                        {
                            found.buy.push(data.pick(g, Some(&v.npc)));
                        }
                    }
                    for b in &found.buy {
                        have.insert(b.gem_id.clone());
                    }
                }
                if !found.take.is_empty() || !found.buy.is_empty() {
                    picks.insert(key(step.id, quest_id), found);
                }
            }
        }

        let unobtainable = needed
            .iter()
            .filter(|g| !have.contains(*g))
            .map(|g| data.pick(g, None))
            .collect();
        Self {
            picks,
            unobtainable,
        }
    }

    /// Fill a Step's quest hand-ins with what to take and buy.
    pub fn decorate(&self, step: &mut crate::guide::Step) {
        let id = step.id;
        for part in step.lines.iter_mut().flat_map(|l| &mut l.parts) {
            if let Fragment::Quest {
                quest_id,
                take,
                buy,
                ..
            } = part
                && let Some(p) = self.picks.get(&key(id, quest_id))
            {
                take.clone_from(&p.take);
                buy.clone_from(&p.buy);
            }
        }
    }
}

/// Search strings for the vendor search box that highlight `gems`, each at
/// most [`SEARCH_LIMIT`] characters (several if the names don't fit in one).
pub fn vendor_regex<'a>(gems: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for name in gems {
        // `.` matches the space (and anything else that would need escaping).
        let term: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '.' })
            .collect();
        let len_with = |terms: &[String]| terms.iter().map(|t| t.len() + 1).sum::<usize>() + 1;
        let mut candidate = current.clone();
        candidate.push(term.clone());
        if len_with(&candidate) > SEARCH_LIMIT && !current.is_empty() {
            out.push(format!("\"{}\"", current.join("|")));
            current = vec![term];
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        out.push(format!("\"{}\"", current.join("|")));
    }
    out
}

/// Skill Groups for a character level: those whose range includes it, plus
/// groups without a range. Disabled groups are left out.
pub fn groups_for_level(build: &Build, level: u32) -> Vec<&SkillGroup> {
    build
        .skill_groups
        .iter()
        .filter(|g| g.enabled && g.levels.is_none_or(|r| r.contains(level)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        guide::{GuideBook, GuideOptions},
        pob::{BuildGem, LevelRange},
    };

    fn gem(name: &str, id: &str) -> BuildGem {
        BuildGem {
            name: name.into(),
            gem_id: Some(format!("Metadata/Items/Gems/{id}")),
            level: 1,
            quality: 0,
            enabled: true,
        }
    }

    fn group(levels: Option<LevelRange>, gems: Vec<BuildGem>) -> SkillGroup {
        SkillGroup {
            label: None,
            skill_set: None,
            slot: None,
            enabled: true,
            gems,
            levels,
        }
    }

    fn witch_build() -> Build {
        Build {
            class: "Witch".into(),
            ascendancy: None,
            level: 90,
            bandit: None,
            skill_groups: vec![
                group(
                    Some(LevelRange {
                        from: 1,
                        to: Some(12),
                    }),
                    vec![
                        gem("Raise Zombie", "SkillGemRaiseZombie"),
                        gem("Minion Damage", "SupportGemMinionDamage"),
                    ],
                ),
                group(
                    None,
                    vec![
                        gem("Frostblink", "SkillGemFrostblink"),
                        gem("Arcane Surge", "SupportGemArcaneSurge"),
                    ],
                ),
                group(
                    Some(LevelRange { from: 28, to: None }),
                    vec![gem("Raise Spectre", "SkillGemRaiseSpectre")],
                ),
            ],
            tree_stages: vec![],
        }
    }

    #[test]
    fn plans_rewards_along_the_guide() {
        let data = GemData::bundled();
        let book = GuideBook::bundled();
        let guide = book.guide(GuideOptions::default());
        let plan = GemPlan::build(&data, guide, &witch_build());

        let mut takes = Vec::new();
        let mut buys = Vec::new();
        for step in &guide.steps {
            let mut s = step.clone();
            plan.decorate(&mut s);
            for part in s.lines.iter().flat_map(|l| &l.parts) {
                if let Fragment::Quest {
                    take, buy, name, ..
                } = part
                {
                    takes.extend(take.iter().map(|t| (name.clone(), t.name.clone())));
                    buys.extend(buy.iter().map(|b| (name.clone(), b.name.clone())));
                }
            }
        }
        // A Witch can take Raise Zombie from the very first quest.
        assert_eq!(
            takes[0],
            ("Enemy at the Gate".into(), "Raise Zombie".into())
        );
        // Every gem comes from exactly one place.
        let mut all: Vec<_> = takes.iter().chain(&buys).map(|(_, g)| g.clone()).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "a gem was planned twice: {takes:?} {buys:?}");
        assert!(plan.unobtainable.is_empty(), "{:?}", plan.unobtainable);
        assert_eq!(n, 5);
        // Purchases name the vendor.
        let decorated: Vec<_> = plan.picks.values().flat_map(|p| &p.buy).collect();
        assert!(decorated.iter().all(|b| b.npc.is_some()));
    }

    #[test]
    fn regex_fits_the_search_box() {
        assert_eq!(
            vendor_regex(["Arcane Surge", "Frostblink"]),
            ["\"Arcane.Surge|Frostblink\""]
        );
        let many: Vec<String> = (0..40).map(|i| format!("Some Long Gem Name {i}")).collect();
        let regexes = vendor_regex(many.iter().map(String::as_str));
        assert!(regexes.len() > 1);
        assert!(regexes.iter().all(|r| r.len() <= SEARCH_LIMIT));
        let total: usize = regexes.iter().map(|r| r.matches('|').count() + 1).sum();
        assert_eq!(total, 40);
    }

    #[test]
    fn groups_follow_character_level() {
        let b = witch_build();
        let names = |level| {
            groups_for_level(&b, level)
                .iter()
                .map(|g| g.gems[0].name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(5), ["Raise Zombie", "Frostblink"]);
        assert_eq!(names(20), ["Frostblink"]);
        assert_eq!(names(40), ["Frostblink", "Raise Spectre"]);
    }
}
