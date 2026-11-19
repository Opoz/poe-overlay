//! Path of Building import: decode a PoB export code and read the Build.
//!
//! An export code is URL-safe base64 of zlib-compressed XML. Links
//! (pobb.in, pastebin) are fetched by the shell; this module only sees the
//! code.

use std::io::Read;

use base64::{Engine, engine::general_purpose};
use serde::{Deserialize, Serialize};

use crate::guide::BanditChoice;

/// A Build imported from Path of Building.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub class: String,
    pub ascendancy: Option<String>,
    pub level: u32,
    /// `None` when PoB says "None" (kill all bandits) or doesn't say.
    pub bandit: Option<BanditChoice>,
    pub skill_groups: Vec<SkillGroup>,
    pub tree_stages: Vec<TreeStage>,
}

/// A set of linked gems, with the character levels it's meant for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGroup {
    pub label: Option<String>,
    /// PoB skill set title, when the build has several (often "Act 1-2" etc.).
    pub skill_set: Option<String>,
    pub slot: Option<String>,
    pub enabled: bool,
    pub gems: Vec<BuildGem>,
    /// Character levels this group is for, read from its label or skill set
    /// title. `None` when neither says.
    pub levels: Option<LevelRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGem {
    pub name: String,
    /// `Metadata/Items/Gems/...`, matching the gem data.
    pub gem_id: Option<String>,
    pub level: u32,
    pub quality: u32,
    pub enabled: bool,
}

/// Inclusive character level range; open-ended when `to` is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelRange {
    pub from: u32,
    pub to: Option<u32>,
}

impl LevelRange {
    pub fn contains(&self, level: u32) -> bool {
        level >= self.from && self.to.is_none_or(|to| level <= to)
    }
}

/// One passive tree in the Build (PoB "spec"), e.g. "Level 30".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeStage {
    pub title: Option<String>,
    /// Allocated tree node ids.
    pub nodes: Vec<u32>,
    /// Mastery node id -> chosen effect id.
    pub mastery_effects: Vec<(u32, u32)>,
    /// Levels this stage targets, read from its title.
    pub levels: Option<LevelRange>,
    /// The character level this tree is for ("LvLing Tree lvl 36" -> 36).
    #[serde(default)]
    pub target: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PobError {
    NotBase64,
    NotZlib,
    NotXml(String),
    NotABuild,
}

impl std::fmt::Display for PobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotBase64 => write!(f, "not a Path of Building code"),
            Self::NotZlib => write!(f, "the code is damaged (could not decompress)"),
            Self::NotXml(e) => write!(f, "the code is damaged ({e})"),
            Self::NotABuild => write!(f, "the code does not contain a build"),
        }
    }
}

/// What the player pasted: an export code, or a link to fetch one from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Code(String),
    /// URL that returns the raw export code.
    Fetch(String),
}

/// Recognise pobb.in and pastebin links (fetched as raw code); anything else
/// is treated as a code.
pub fn source(input: &str) -> Source {
    let s = input.trim();
    let path = s
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let id = |rest: &str| {
        rest.trim_start_matches('/')
            .trim_start_matches("raw/")
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .to_owned()
    };
    if let Some(rest) = path.strip_prefix("pobb.in") {
        return Source::Fetch(format!("https://pobb.in/{}/raw", id(rest)));
    }
    if let Some(rest) = path.strip_prefix("pastebin.com") {
        return Source::Fetch(format!("https://pastebin.com/raw/{}", id(rest)));
    }
    Source::Code(s.to_owned())
}

/// Decode a PoB export code into its XML.
pub fn decode(code: &str) -> Result<String, PobError> {
    let cleaned: String = code
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '+' => '-',
            '/' => '_',
            c => c,
        })
        .collect();
    let bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(cleaned.trim_end_matches('='))
        .map_err(|_| PobError::NotBase64)?;
    let mut xml = String::new();
    flate2::read::ZlibDecoder::new(&bytes[..])
        .read_to_string(&mut xml)
        .map_err(|_| PobError::NotZlib)?;
    Ok(xml)
}

/// Decode and parse a PoB export code.
pub fn import(code: &str) -> Result<Build, PobError> {
    parse(&decode(code)?)
}

/// Parse PoB XML.
pub fn parse(xml: &str) -> Result<Build, PobError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| PobError::NotXml(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "PathOfBuilding" {
        return Err(PobError::NotABuild);
    }
    let build = child(root, "Build").ok_or(PobError::NotABuild)?;
    let class = build.attribute("className").unwrap_or("Scion").to_owned();
    let ascendancy = build
        .attribute("ascendClassName")
        .filter(|a| !a.is_empty() && *a != "None")
        .map(str::to_owned);
    let bandit = match build.attribute("bandit") {
        Some("Alira") => Some(BanditChoice::Alira),
        Some("Kraityn") => Some(BanditChoice::Kraityn),
        Some("Oak") => Some(BanditChoice::Oak),
        Some("None") => Some(BanditChoice::KillAll),
        _ => None,
    };

    let mut skill_groups = Vec::new();
    if let Some(skills) = child(root, "Skills") {
        let sets: Vec<_> = skills
            .children()
            .filter(|n| n.has_tag_name("SkillSet"))
            .collect();
        if sets.is_empty() {
            // Older PoB: groups directly under <Skills>.
            skill_groups.extend(groups_in(skills, None));
        } else {
            for set in sets {
                let title = set
                    .attribute("title")
                    .map(clean_text)
                    .filter(|t| !t.is_empty());
                skill_groups.extend(groups_in(set, title.as_deref()));
            }
        }
    }

    let tree_stages = child(root, "Tree")
        .map(|tree| {
            tree.children()
                .filter(|n| n.has_tag_name("Spec"))
                .map(|spec| {
                    let title = spec
                        .attribute("title")
                        .map(clean_text)
                        .filter(|t| !t.is_empty());
                    TreeStage {
                        levels: title.as_deref().and_then(level_range),
                        target: title.as_deref().and_then(target_level),
                        title,
                        nodes: spec
                            .attribute("nodes")
                            .unwrap_or_default()
                            .split(',')
                            .filter_map(|n| n.trim().parse().ok())
                            .collect(),
                        mastery_effects: mastery_effects(
                            spec.attribute("masteryEffects").unwrap_or_default(),
                        ),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Build {
        class,
        ascendancy,
        level: build
            .attribute("level")
            .and_then(|l| l.parse().ok())
            .unwrap_or(1),
        bandit,
        skill_groups,
        tree_stages,
    })
}

fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children().find(|n| n.has_tag_name(name))
}

fn groups_in(parent: roxmltree::Node, set_title: Option<&str>) -> Vec<SkillGroup> {
    parent
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .map(|skill| {
            let label = skill
                .attribute("label")
                .map(clean_text)
                .filter(|l| !l.is_empty());
            let gems = skill
                .children()
                .filter(|n| n.has_tag_name("Gem"))
                .map(|gem| BuildGem {
                    name: gem
                        .attribute("nameSpec")
                        .or_else(|| gem.attribute("skillId"))
                        .unwrap_or("?")
                        .to_owned(),
                    gem_id: gem.attribute("gemId").map(str::to_owned),
                    level: gem
                        .attribute("level")
                        .and_then(|l| l.parse().ok())
                        .unwrap_or(1),
                    quality: gem
                        .attribute("quality")
                        .and_then(|q| q.parse().ok())
                        .unwrap_or(0),
                    enabled: gem.attribute("enabled") != Some("false"),
                })
                .collect();
            let levels = label
                .as_deref()
                .and_then(level_range)
                .or_else(|| set_title.and_then(level_range));
            SkillGroup {
                label,
                skill_set: set_title.map(str::to_owned),
                slot: skill
                    .attribute("slot")
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
                enabled: skill.attribute("enabled") != Some("false"),
                gems,
                levels,
            }
        })
        .collect()
}

/// `{12345,6789},{...}`
fn mastery_effects(raw: &str) -> Vec<(u32, u32)> {
    raw.split('}')
        .filter_map(|pair| {
            let pair = pair.trim_start_matches([',', '{', ' ']);
            let (node, effect) = pair.split_once(',')?;
            Some((node.trim().parse().ok()?, effect.trim().parse().ok()?))
        })
        .collect()
}

/// Remove Path of Building formatting from a label or title: colour codes
/// (`^6`, `^xRRGGBB`), skill set ids (`{9}`) and repeated spaces.
pub fn clean_text(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '^' if chars.get(i + 1).is_some_and(char::is_ascii_digit) => i += 2,
            '^' if chars.get(i + 1) == Some(&'x')
                && chars.len() >= i + 8
                && chars[i + 2..i + 8].iter().all(char::is_ascii_hexdigit) =>
            {
                i += 8;
            }
            '{' => {
                let close = chars[i..].iter().position(|&c| c == '}');
                match close {
                    Some(n) if chars[i + 1..i + n].iter().all(char::is_ascii_digit) => i += n + 1,
                    _ => {
                        out.push('{');
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Drop link-count tokens ("3L", "4-6L", "1S") so they aren't read as levels.
fn without_link_counts(text: &str) -> String {
    text.split_whitespace()
        .filter(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            let Some(body) = w.strip_suffix(['L', 'l', 'S', 's']) else {
                return true;
            };
            !(!body.is_empty() && body.chars().all(|c| c.is_ascii_digit() || c == '-'))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The level a Tree Stage is for: the first level mentioned in its title.
pub fn target_level(title: &str) -> Option<u32> {
    let text = without_link_counts(&clean_text(title).to_lowercase());
    if !mentions_level(&text) || text.contains("act") {
        return level_range(title).map(|r| r.to.unwrap_or(r.from));
    }
    numbers_with_plus(&text)
        .first()
        .map(|(n, _)| *n)
        .filter(|n| (1..=100).contains(n))
}

fn mentions_level(lower: &str) -> bool {
    lower.contains("lvl")
        || lower.contains("level")
        || lower.contains("lv ")
        || lower.contains("lv.")
}

/// Approximate character level when each act starts (entering its first area).
const ACT_START_LEVEL: [u32; 11] = [1, 1, 13, 23, 33, 40, 45, 50, 55, 61, 64];

/// Read a character level range from a label or title. Understands
/// "lvl 12+", "Level 1-12", "12 - 28", "(31)", "Act 2", "Acts 3-4", "Act 4+".
/// Returns `None` when there are no numbers in a recognisable shape.
pub fn level_range(text: &str) -> Option<LevelRange> {
    let lower = without_link_counts(&clean_text(text).to_lowercase());
    // Only trust numbers in a label that talks about levels or acts, or
    // that is nothing but a range ("12-28", "(31)").
    let only_numbers = lower
        .chars()
        .all(|c| c.is_ascii_digit() || " -+()".contains(c));
    let is_act = lower.contains("act");
    if !(mentions_level(&lower) || is_act || only_numbers) {
        return None;
    }
    let numbers = numbers_with_plus(&lower);
    let (a, b, plus) = match numbers.as_slice() {
        [(a, plus)] => (*a, None, *plus),
        [(a, _), (b, plus), ..] => (*a, Some(*b), *plus),
        [] => return None,
    };
    if is_act {
        let act_from = a.clamp(1, 10) as usize;
        let from = ACT_START_LEVEL[act_from];
        let act_to = b.filter(|b| *b >= a).unwrap_or(a).clamp(1, 10) as usize;
        // The last act runs until the campaign ends.
        let to = if plus || act_to >= 10 {
            None
        } else {
            Some(ACT_START_LEVEL[act_to + 1].saturating_sub(1).max(from))
        };
        return Some(LevelRange { from, to });
    }
    if a == 0 || a > 100 {
        return None;
    }
    Some(match b {
        Some(b) if b >= a && b <= 100 && !plus => LevelRange {
            from: a,
            to: Some(b),
        },
        _ => LevelRange { from: a, to: None },
    })
}

/// Numbers in order, each with whether a `+` follows it.
fn numbers_with_plus(text: &str) -> Vec<(u32, bool)> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let n: String = chars[start..i].iter().collect();
            let plus = chars[i..].iter().find(|c| !c.is_whitespace()) == Some(&'+');
            if let Ok(n) = n.parse() {
                out.push((n, plus));
            }
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
  <Build level="92" className="Witch" ascendClassName="Necromancer" bandit="Alira" mainSocketGroup="1">
  </Build>
  <Skills activeSkillSet="1">
    <SkillSet id="1" title="Act 1-2">
      <Skill label="" enabled="true" slot="Body Armour">
        <Gem nameSpec="Raise Zombie" gemId="Metadata/Items/Gems/SkillGemRaiseZombie" level="1" quality="0" enabled="true"/>
        <Gem nameSpec="Minion Damage" gemId="Metadata/Items/Gems/SupportGemMinionDamage" level="1" quality="0" enabled="true"/>
      </Skill>
    </SkillSet>
    <SkillSet id="2" title="Endgame">
      <Skill label="Main lvl 28+" enabled="true" slot="Helmet">
        <Gem nameSpec="Raise Spectre" gemId="Metadata/Items/Gems/SkillGemRaiseSpectre" level="20" quality="20" enabled="true"/>
      </Skill>
      <Skill label="Auras" enabled="false">
        <Gem nameSpec="Clarity" gemId="Metadata/Items/Gems/SkillGemClarity" level="10" quality="0" enabled="false"/>
      </Skill>
    </SkillSet>
  </Skills>
  <Tree activeSpec="1">
    <Spec title="Level 30" nodes="54447,19635,50826" masteryEffects="{53188,64875},{1,2}" treeVersion="3_25">
      <URL>https://www.pathofexile.com/passive-skill-tree/x</URL>
    </Spec>
    <Spec title="" nodes="54447" treeVersion="3_25"/>
  </Tree>
</PathOfBuilding>"#;

    fn encode(xml: &str) -> String {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(xml.as_bytes()).unwrap();
        general_purpose::URL_SAFE.encode(z.finish().unwrap())
    }

    #[test]
    fn decodes_and_parses_a_code() {
        let b = import(&encode(XML)).unwrap();
        assert_eq!(b.class, "Witch");
        assert_eq!(b.ascendancy.as_deref(), Some("Necromancer"));
        assert_eq!(b.level, 92);
        assert_eq!(b.bandit, Some(BanditChoice::Alira));
        assert_eq!(b.skill_groups.len(), 3);
        let zombies = &b.skill_groups[0];
        assert_eq!(zombies.skill_set.as_deref(), Some("Act 1-2"));
        assert_eq!(zombies.gems[1].name, "Minion Damage");
        assert_eq!(
            zombies.gems[0].gem_id.as_deref(),
            Some("Metadata/Items/Gems/SkillGemRaiseZombie")
        );
        assert_eq!(
            zombies.levels,
            Some(LevelRange {
                from: 1,
                to: Some(22)
            })
        );
        assert_eq!(
            b.skill_groups[1].levels,
            Some(LevelRange { from: 28, to: None })
        );
        // Unlabelled group in an unranged set: no levels.
        assert_eq!(b.skill_groups[2].levels, None);
        assert!(!b.skill_groups[2].enabled);

        assert_eq!(b.tree_stages.len(), 2);
        assert_eq!(b.tree_stages[0].nodes, [54447, 19635, 50826]);
        assert_eq!(b.tree_stages[0].mastery_effects, [(53188, 64875), (1, 2)]);
        assert_eq!(
            b.tree_stages[0].levels,
            Some(LevelRange { from: 30, to: None })
        );
        assert_eq!(b.tree_stages[1].title, None);
    }

    #[test]
    fn tolerates_padding_whitespace_and_standard_base64() {
        let code = encode(XML);
        let messy = format!("  {}\n", code.replace('-', "+").replace('_', "/"));
        assert!(import(&messy).is_ok());
    }

    #[test]
    fn recognises_links() {
        assert_eq!(
            source(" https://pobb.in/AbC123 "),
            Source::Fetch("https://pobb.in/AbC123/raw".into())
        );
        assert_eq!(
            source("pobb.in/AbC123/raw"),
            Source::Fetch("https://pobb.in/AbC123/raw".into())
        );
        assert_eq!(
            source("https://pastebin.com/xyz789"),
            Source::Fetch("https://pastebin.com/raw/xyz789".into())
        );
        assert_eq!(
            source("https://pastebin.com/raw/xyz789"),
            Source::Fetch("https://pastebin.com/raw/xyz789".into())
        );
        assert_eq!(source("eNrtWl1v2zYU"), Source::Code("eNrtWl1v2zYU".into()));
    }

    #[test]
    fn cleans_pob_formatting() {
        assert_eq!(
            clean_text("^6^4GEMS IN ^xFF0000WEAPON SWAP"),
            "GEMS IN WEAPON SWAP"
        );
        assert_eq!(clean_text("^2Early Covenant {9}"), "Early Covenant");
        assert_eq!(clean_text("Keep {braces} ^ alone"), "Keep {braces} ^ alone");
    }

    #[test]
    fn tree_targets() {
        assert_eq!(target_level("^2LvLing Tree lvl 36"), Some(36));
        assert_eq!(
            target_level("^2LvLing Tree lvl 80 (Respec 18 points)"),
            Some(80)
        );
        assert_eq!(target_level("^1Mid lvl 96 {5}"), Some(96));
        assert_eq!(target_level("Level 30"), Some(30));
        assert_eq!(target_level("Act 3"), Some(32));
        assert_eq!(
            target_level("^2(Optional) ^5United In Dream Covenant {4}"),
            None
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(import("not a code!"), Err(PobError::NotBase64));
        assert_eq!(import("aGVsbG8"), Err(PobError::NotZlib));
        assert_eq!(import(&encode("<Other/>")), Err(PobError::NotABuild));
    }

    #[test]
    fn older_pob_without_skill_sets() {
        let xml = r#"<PathOfBuilding><Build className="Ranger" bandit="None"/>
            <Skills><Skill label="lvl 1-12"><Gem nameSpec="Split Arrow"/></Skill></Skills></PathOfBuilding>"#;
        let b = parse(xml).unwrap();
        assert_eq!(b.bandit, Some(BanditChoice::KillAll));
        assert_eq!(
            b.skill_groups[0].levels,
            Some(LevelRange {
                from: 1,
                to: Some(12)
            })
        );
        assert_eq!(b.skill_groups[0].gems[0].gem_id, None);
    }

    #[test]
    fn reads_level_ranges_from_labels() {
        let r = |from, to| Some(LevelRange { from, to });
        assert_eq!(level_range("lvl 12+"), r(12, None));
        assert_eq!(level_range("Level 1-12"), r(1, Some(12)));
        assert_eq!(level_range("12 - 28"), r(12, Some(28)));
        assert_eq!(level_range("Leveling (31)"), r(31, None));
        assert_eq!(level_range("Act 2"), r(13, Some(22)));
        assert_eq!(level_range("Acts 3-4"), r(23, Some(39)));
        assert_eq!(level_range("Act 4+"), r(33, None));
        assert_eq!(level_range("Main skill"), None);
        assert_eq!(level_range("Level 0"), None);
        // PoB formatting and link counts are not levels.
        assert_eq!(
            level_range("^62L Before minions (Replace when you get srs)"),
            None
        );
        assert_eq!(level_range("^6Act 4-10"), r(33, None));
        assert_eq!(level_range("^6^5Lvl70 after 3rd lab"), r(70, None));
        assert_eq!(
            level_range("^2lvl 80-92 Early Rare 6 link {1}"),
            r(80, Some(92))
        );
        assert_eq!(level_range("^2Early Covenant {9}"), None);
        assert_eq!(level_range("3L ^2Spectres (1 forest warrior) & AG"), None);
        assert_eq!(level_range("^61S Flesh offering at lvl 12"), r(12, None));
        assert_eq!(level_range("Act 10"), r(64, None));
        assert!(r(1, Some(12)).unwrap().contains(12));
        assert!(!r(1, Some(12)).unwrap().contains(13));
    }
}
