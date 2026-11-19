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

        // Gems missing from the data still have PoB's name for them.
        let pob_names: HashMap<&str, &str> = build
            .skill_groups
            .iter()
            .flat_map(|g| &g.gems)
            .filter_map(|g| Some((g.gem_id.as_deref()?, g.name.as_str())))
            .collect();
        let unobtainable = needed
            .iter()
            .filter(|g| !have.contains(*g))
            .map(|g| {
                let mut pick = data.pick(g, None);
                if data.gem(g).is_none()
                    && let Some(name) = pob_names.get(g.as_str())
                {
                    pick.name = (*name).to_owned();
                }
                pick
            })
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

/// Add the Build's gem picks to a Guide view: what to take and buy at each
/// hand-in shown, the current Step's Vendor Regex, and gems that can't be
/// had from quests.
pub fn decorate_view(
    view: &mut crate::guide::GuideView,
    data: &GemData,
    guide: &Guide,
    build: &Build,
) {
    let plan = GemPlan::build(data, guide, build);
    for step in view
        .current
        .iter_mut()
        .chain(view.upcoming.iter_mut())
        .chain(view.act_steps.iter_mut())
    {
        plan.decorate(step);
    }
    view.vendor_regex = view
        .current
        .as_ref()
        .map(|s| vendor_regex(step_purchases(s).iter().map(|g| g.name.as_str())))
        .unwrap_or_default();
    view.unobtainable = plan.unobtainable;
}

/// Gems to buy at a (decorated) Step.
pub fn step_purchases(step: &crate::guide::Step) -> Vec<&GemPick> {
    step.lines
        .iter()
        .flat_map(|l| &l.parts)
        .flat_map(|p| match p {
            Fragment::Quest { buy, .. } => buy.iter().collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
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

/// What the gem links Overlay shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GemsView {
    pub character: Option<String>,
    pub level: u32,
    pub has_build: bool,
    /// The PoB skill set shown (builds often have one per stage).
    pub skill_set: Option<String>,
    /// All skill set titles, in order, for picking one by hand.
    pub skill_sets: Vec<String>,
    /// The player picked the skill set (instead of by level).
    pub skill_set_by_hand: bool,
    /// Skill Groups for the current level, gems coloured.
    pub groups: Vec<GroupView>,
}

/// Skill set titles in Build order, once each.
pub fn skill_sets(build: &Build) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for g in &build.skill_groups {
        if let Some(t) = &g.skill_set
            && !out.contains(t)
        {
            out.push(t.clone());
        }
    }
    out
}

/// The skill set for `level`: the one whose title's range includes it (the
/// latest-starting if several), else the latest one already started.
/// `None` when no set title says a level (then all groups are used).
pub fn pick_skill_set(build: &Build, level: u32) -> Option<String> {
    let ranged: Vec<(String, crate::pob::LevelRange)> = skill_sets(build)
        .into_iter()
        .filter_map(|t| crate::pob::level_range(&t).map(|r| (t, r)))
        .collect();
    let containing = ranged
        .iter()
        .filter(|(_, r)| r.contains(level))
        .max_by_key(|(_, r)| r.from);
    containing
        .or_else(|| {
            ranged
                .iter()
                .filter(|(_, r)| r.from <= level)
                .max_by_key(|(_, r)| r.from)
        })
        .or_else(|| ranged.iter().min_by_key(|(_, r)| r.from))
        .map(|(t, _)| t.clone())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupView {
    pub label: Option<String>,
    pub skill_set: Option<String>,
    pub slot: Option<String>,
    pub levels: Option<crate::pob::LevelRange>,
    pub gems: Vec<GemView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GemView {
    pub name: String,
    pub colour: String,
    pub support: bool,
    /// Can't be used yet at this character level.
    pub too_high: bool,
}

impl GemsView {
    pub fn build(data: &GemData, profile: Option<&crate::profile::Profile>) -> Self {
        let level = profile.map_or(1, |p| p.level);
        let build = profile.and_then(|p| p.build.as_ref());
        let overrides = profile.map(|p| &p.group_levels);
        let sets = build.map(skill_sets).unwrap_or_default();
        let by_hand = profile
            .and_then(|p| p.skill_set.clone())
            .filter(|s| sets.contains(s));
        let chosen = by_hand
            .clone()
            .or_else(|| build.and_then(|b| pick_skill_set(b, level)));
        let groups = build
            .map(|b| {
                b.skill_groups
                    .iter()
                    .enumerate()
                    .filter(|(_, g)| chosen.is_none() || g.skill_set == chosen)
                    // Separators and headings ("==== Auras ====") have no gems.
                    .filter(|(_, g)| g.gems.iter().any(|gem| gem.enabled))
                    .filter_map(|(i, g)| {
                        let levels = overrides
                            .and_then(|o| o.get(&i).copied())
                            .unwrap_or(g.levels);
                        // Within a set picked by hand, show all of it.
                        let in_range =
                            by_hand.is_some() || levels.is_none_or(|r| r.contains(level));
                        (g.enabled && in_range).then_some((g, levels))
                    })
                    .map(|(g, levels)| GroupView {
                        label: g.label.clone(),
                        skill_set: g.skill_set.clone(),
                        slot: g.slot.clone(),
                        levels,
                        gems: g
                            .gems
                            .iter()
                            .filter(|gem| gem.enabled)
                            .map(|gem| {
                                let info = gem.gem_id.as_deref().and_then(|id| data.gem(id));
                                GemView {
                                    name: gem.name.clone(),
                                    colour: info.map_or_else(|| "w".into(), |i| i.colour.clone()),
                                    support: info.is_some_and(|i| i.support),
                                    too_high: info.is_some_and(|i| i.level > level),
                                }
                            })
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            character: profile.map(|p| p.name.clone()),
            level,
            has_build: build.is_some(),
            skill_set: chosen,
            skill_sets: sets,
            skill_set_by_hand: by_hand.is_some(),
            groups,
        }
    }
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
    fn hand_set_ranges_override_labels() {
        let data = GemData::bundled();
        let mut p = crate::profile::Profile::new("A");
        p.level = 20;
        p.apply_build(witch_build(), "code".into());
        let names = |p: &crate::profile::Profile| {
            GemsView::build(&data, Some(p))
                .groups
                .iter()
                .map(|g| g.gems[0].name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&p), ["Frostblink"]);
        // Limit the unlabelled Frostblink group to levels 1-10, and widen zombies.
        p.group_levels.insert(
            1,
            Some(LevelRange {
                from: 1,
                to: Some(10),
            }),
        );
        p.group_levels.insert(0, None);
        assert_eq!(names(&p), ["Raise Zombie"]);
    }

    #[test]
    fn one_skill_set_at_a_time() {
        let data = GemData::bundled();
        let set = |title: &str, label: &str, name: &str| SkillGroup {
            label: Some(label.into()),
            skill_set: Some(title.into()),
            slot: None,
            enabled: true,
            gems: vec![gem(name, "SkillGemRaiseZombie")],
            levels: crate::pob::level_range(label).or_else(|| crate::pob::level_range(title)),
        };
        let mut b = witch_build();
        b.skill_groups = vec![
            set("Act 1", "Zombies", "A1 zombies"),
            set("Act 1", "Mobility lvl 8", "A1 dash"),
            set("Act 4-10", "Main", "A4 main"),
            set("Lvl70 after 3rd lab", "Main", "70 main"),
            set("Early Covenant", "Main", "covenant"),
        ];
        // Separator group: ignored.
        b.skill_groups.push(SkillGroup {
            label: Some("-----".into()),
            skill_set: Some("Act 1".into()),
            slot: None,
            enabled: true,
            gems: vec![],
            levels: None,
        });
        let mut p = crate::profile::Profile::new("A");
        p.apply_build(b, "code".into());
        let shown = |p: &crate::profile::Profile| {
            let v = GemsView::build(&data, Some(p));
            (
                v.skill_set.clone(),
                v.groups
                    .iter()
                    .map(|g| g.gems[0].name.clone())
                    .collect::<Vec<_>>(),
            )
        };
        p.level = 5;
        assert_eq!(
            shown(&p),
            (Some("Act 1".into()), vec!["A1 zombies".to_string()])
        );
        p.level = 9;
        assert_eq!(shown(&p).1, ["A1 zombies", "A1 dash"]);
        p.level = 40;
        assert_eq!(
            shown(&p),
            (Some("Act 4-10".into()), vec!["A4 main".to_string()])
        );
        p.level = 75;
        assert_eq!(shown(&p).0.as_deref(), Some("Lvl70 after 3rd lab"));
        // Picked by hand: the unranged Covenant set.
        p.skill_set = Some("Early Covenant".into());
        assert_eq!(shown(&p).1, ["covenant"]);
        assert!(GemsView::build(&data, Some(&p)).skill_set_by_hand);
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
