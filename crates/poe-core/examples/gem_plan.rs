//! Prints where a test Witch build's gems come from along the Guide.
use poe_core::{
    gems::{GemData, GemPlan},
    guide::{Fragment, GuideBook, GuideOptions},
    pob::{Build, BuildGem, SkillGroup},
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
fn main() {
    let build = Build {
        class: "Witch".into(),
        ascendancy: None,
        level: 90,
        bandit: None,
        tree_stages: vec![],
        skill_groups: vec![SkillGroup {
            label: None,
            skill_set: None,
            slot: None,
            enabled: true,
            levels: None,
            gems: vec![
                gem("Raise Zombie", "SkillGemRaiseZombie"),
                gem("Minion Damage", "SupportGemMinionDamage"),
                gem("Frostblink", "SkillGemFrostblink"),
                gem("Arcane Surge", "SupportGemArcaneSurge"),
                gem("Raise Spectre", "SkillGemRaiseSpectre"),
                gem("Summon Raging Spirit", "SkillGemSummonRagingSpirit"),
                gem("Flesh Offering", "SkillGemFleshOffering"),
                gem("Clarity", "SkillGemClarity"),
            ],
        }],
    };
    let book = GuideBook::bundled();
    let guide = book.guide(GuideOptions::default());
    let plan = GemPlan::build(&GemData::bundled(), guide, &build);
    for step in &guide.steps {
        let mut s = step.clone();
        plan.decorate(&mut s);
        for p in s.lines.iter().flat_map(|l| &l.parts) {
            if let Fragment::Quest {
                name,
                npc,
                take,
                buy,
                ..
            } = p
            {
                for t in take {
                    println!(
                        "A{} take {:<22} from {} ({})",
                        s.act,
                        t.name,
                        npc.as_deref().unwrap_or("?"),
                        name
                    );
                }
                for b in buy {
                    println!(
                        "A{} buy  {:<22} from {} (after {})",
                        s.act,
                        b.name,
                        b.npc.as_deref().unwrap_or("?"),
                        name
                    );
                }
            }
        }
    }
    for u in &plan.unobtainable {
        println!("not available: {}", u.name);
    }
}
