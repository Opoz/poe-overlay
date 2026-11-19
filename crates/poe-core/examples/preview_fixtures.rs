//! Builds the data the UI preview renders (`src/preview/fixtures.json`) from
//! the real domain logic: the scrubbed Act 1-9 replay fixture, a synthetic
//! Witch build and the real poelab lab file. No private data.
//!
//! Usage: cargo run -p poe-core --example preview_fixtures > src/preview/fixtures.json

use std::{collections::BTreeMap, io::BufReader};

use poe_core::{
    gems::{GemData, GemsView, decorate_view},
    guide::{GuideBook, GuideView},
    lab::{LabLayout, LabState, LabView},
    log::{LogEvent, replay},
    pob::{Build, BuildGem, LevelRange, SkillGroup, TreeStage, level_range},
    run::{Command, RunTimer, TimerSettings},
    tree::{PassiveTree, Transform, TreeOverlay, TreeView, allocations_from_log},
    view::TimerView,
};
use serde_json::{Value, json};

const LOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/witch-acts1-9.log"
);
const LAB: &str = include_str!("../tests/fixtures/poelab-normal-2026-11-19.json");

fn gem(name: &str, id: &str) -> BuildGem {
    BuildGem {
        name: name.into(),
        gem_id: Some(format!("Metadata/Items/Gems/{id}")),
        level: 1,
        quality: 0,
        enabled: true,
    }
}

fn group(set: &str, label: &str, gems: Vec<BuildGem>) -> SkillGroup {
    SkillGroup {
        label: Some(label.into()),
        skill_set: Some(set.into()),
        slot: None,
        enabled: true,
        levels: level_range(label).or_else(|| level_range(set)),
        gems,
    }
}

fn build(tree: &PassiveTree, allocated: &[u32]) -> Build {
    // Tree stages: what's allocated so far, plus the next few nodes outward.
    let have: std::collections::HashSet<u32> = allocated.iter().copied().collect();
    let mut extra: Vec<u32> = Vec::new();
    for n in &tree.nodes {
        if extra.len() >= 14 {
            break;
        }
        if n.ascendancy.is_none()
            && !have.contains(&n.id)
            && n.out.iter().any(|o| have.contains(o) || extra.contains(o))
        {
            extra.push(n.id);
        }
    }
    let mut stage2: Vec<u32> = allocated.to_vec();
    stage2.extend(&extra);
    Build {
        class: "Witch".into(),
        ascendancy: Some("Necromancer".into()),
        level: 92,
        bandit: None,
        skill_groups: vec![
            group(
                "Act 1-3",
                "Zombies",
                vec![
                    gem("Raise Zombie", "SkillGemRaiseZombie"),
                    gem("Minion Damage", "SupportGemMinionDamage"),
                    gem("Melee Splash", "SupportGemMeleeSplash"),
                ],
            ),
            group(
                "Act 1-3",
                "Main skill",
                vec![
                    gem("Summon Raging Spirit", "SkillGemSummonRagingSpirit"),
                    gem("Arcane Surge", "SupportGemArcaneSurge"),
                    gem(
                        "Elemental Damage with Attacks",
                        "SupportGemElementalDamageWithAttacks",
                    ),
                ],
            ),
            group(
                "Act 1-3",
                "Mobility",
                vec![gem("Frostblink", "SkillGemFrostblink")],
            ),
            group(
                "Act 1-3",
                "Auras lvl 16",
                vec![
                    gem("Clarity", "SkillGemClarity"),
                    gem("Vitality", "SkillGemVitality"),
                ],
            ),
            group(
                "Act 4-10",
                "Spectres",
                vec![
                    gem("Raise Spectre", "SkillGemRaiseSpectre"),
                    gem("Minion Damage", "SupportGemMinionDamage"),
                ],
            ),
        ],
        tree_stages: vec![
            TreeStage {
                title: Some("LvLing Tree lvl 36".into()),
                nodes: stage2,
                mastery_effects: vec![],
                levels: Some(LevelRange { from: 36, to: None }),
                target: Some(36),
            },
            TreeStage {
                title: Some("LvLing Tree lvl 60".into()),
                nodes: allocated.to_vec(),
                mastery_effects: vec![],
                levels: Some(LevelRange { from: 60, to: None }),
                target: Some(60),
            },
        ],
    }
}

fn main() {
    let records: Vec<_> = replay(BufReader::new(std::fs::File::open(LOG).unwrap())).collect();
    let book = GuideBook::bundled();
    let tree = PassiveTree::bundled();
    let gems = GemData::bundled();

    // Replay up to the middle of Act 3.
    let cut = records
        .iter()
        .position(
            |r| matches!(&r.event, LogEvent::AreaGenerated { area_id, .. } if area_id == "1_3_5"),
        )
        .unwrap();
    let mut timer = RunTimer::new(TimerSettings::default());
    timer.category = Some("SSF HC".into());
    for r in &records[..cut] {
        timer.on_record(r);
        if let LogEvent::AreaGenerated { area_id, .. } = &r.event
            && let Some(p) = timer.active_profile_mut()
        {
            book.on_area(p, area_id);
        }
    }
    let now = records[cut].time * 1000 + 95_000;
    let mut profile = timer.profiles().next().unwrap().clone();
    profile.level = 27;

    let allocations = allocations_from_log(records[..cut].iter().cloned(), &profile.name);
    profile.passives = allocations.passives;
    profile.masteries = allocations.masteries;
    let allocated = tree.allocated_nodes(&profile.passives);
    profile.build = Some(build(&tree, &allocated));

    // A finished past run in the same category, for PB deltas and history.
    let mut pb = RunTimer::new(TimerSettings::default());
    pb.category = Some("SSF HC".into());
    pb.command(Command::Start, 0);
    let pb_splits = [
        3_900, 5_800, 7_700, 11_100, 13_000, 15_600, 17_100, 18_800, 21_000, 23_400,
    ];
    for s in pb_splits {
        pb.command(Command::Split, s * 1000);
    }
    let mut pb_run = pb.run().unwrap().clone();
    pb_run.character = Some("PreviousWitch".into());
    pb_run.class = Some("Necromancer".into());
    let history = vec![pb_run];

    let timer_view = TimerView::build(timer.run(), &history, now);

    let guide = book.guide(profile.guide_options);
    let mut guide_view = GuideView::build(&book, Some(&profile), None);
    decorate_view(
        &mut guide_view,
        &gems,
        guide,
        profile.build.as_ref().unwrap(),
    );

    let gems_view = GemsView::build(&gems, Some(&profile));

    // Tree aligned so the Witch start sits near the top middle of a 1600x900 window.
    let witch = tree.base_class("Witch").unwrap().start_node.unwrap();
    let w = tree.node(witch).unwrap();
    let scale = 0.20;
    let overlay = TreeOverlay {
        visible: true,
        transform: Some(Transform {
            scale,
            offset_x: 800.0 - w.x * scale,
            offset_y: 260.0 - w.y * scale,
        }),
        ..Default::default()
    };
    let tree_view = TreeView::build(&tree, &overlay, Some(&profile));
    let nodes: Vec<Value> = tree
        .nodes
        .iter()
        .map(|n| serde_json::to_value(n).unwrap())
        .collect();

    let mut lab = LabState::default();
    lab.load(LabLayout::parse(LAB).unwrap());
    lab.on_area("Labyrinth_Airlock", "Aspirants' Plaza");
    lab.on_area("1_Labyrinth_P_straight", "Estate Path");
    lab.on_area("1_Labyrinth_boss_1", "Aspirant's Trial");
    let lab_view = LabView::build(&lab, "2026-11-19");

    let mut characters = BTreeMap::new();
    characters.insert(profile.name.clone(), profile.clone());
    let mut other = poe_core::profile::Profile::new("MappingRanger");
    other.class = Some("Deadeye".into());
    other.level = 94;
    other.last_seen = now - 86_400_000;

    let out = json!({
        "timer": timer_view,
        "guide": guide_view,
        "gems": gems_view,
        "tree": tree_view,
        "treeNodes": nodes,
        "lab": lab_view,
        "characters": {
            "active": profile.name,
            "profiles": [profile, other],
        },
        "history": history,
    });
    println!("{}", serde_json::to_string(&out).unwrap());
}
