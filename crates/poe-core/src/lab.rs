//! The Labyrinth: a poelab Lab Layout ("compass file"), where the player is
//! in it according to the Client Log, and the route to the next Izaro fight.
//!
//! The compass file is JSON with a `difficulty`, a `date` and `rooms`, each
//! with a `name`, map position `x`/`y`, `exits` (direction -> room number)
//! and usually `contents`. Parsing is lenient: unknown fields are ignored and
//! missing ones default.

use std::collections::{BTreeSet, HashMap, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The room where Izaro is fought (each lab has one to three).
const IZARO_ROOM: &str = "Aspirant's Trial";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Difficulty {
    Normal,
    Cruel,
    Merciless,
    Eternal,
}

impl Difficulty {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "cruel" => Some(Self::Cruel),
            "merciless" => Some(Self::Merciless),
            "eternal" | "uber" => Some(Self::Eternal),
            _ => None,
        }
    }

    /// The difficulty of a Labyrinth room's Area ID.
    pub fn of_area(area_id: &str) -> Option<Self> {
        if !area_id.contains("Labyrinth") || area_id.contains("Airlock") {
            return None;
        }
        match area_id.split('_').next()? {
            "1" => Some(Self::Normal),
            "2" => Some(Self::Cruel),
            "3" => Some(Self::Merciless),
            "EndGame" => Some(Self::Eternal),
            _ => None,
        }
    }
}

/// Things worth knowing about in a room.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomContents {
    pub darkshrine: bool,
    pub silver_key: bool,
    pub silver_door: bool,
    pub argus: bool,
    /// Everything listed, as written in the file.
    pub raw: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabRoom {
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// Indexes into [`LabLayout::rooms`].
    pub exits: Vec<usize>,
    pub contents: RoomContents,
    pub izaro: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabLayout {
    pub difficulty: Option<Difficulty>,
    /// `YYYY-MM-DD`, the day the layout is for (UTC).
    pub date: Option<String>,
    pub rooms: Vec<LabRoom>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    NotJson,
    NoRooms,
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson => write!(f, "that isn't a poelab lab compass file"),
            Self::NoRooms => write!(f, "the lab compass file has no rooms"),
        }
    }
}

/// Does this text look like a compass file (as opposed to anything else that
/// happens to be on the clipboard)?
pub fn looks_like_layout(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with('{') && t.contains("\"difficulty\"") && t.contains("\"rooms\"")
}

impl LabLayout {
    pub fn parse(text: &str) -> Result<Self, LayoutError> {
        let v: Value = serde_json::from_str(text).map_err(|_| LayoutError::NotJson)?;
        let rooms_v = v
            .get("rooms")
            .and_then(Value::as_array)
            .ok_or(LayoutError::NoRooms)?;
        if rooms_v.is_empty() {
            return Err(LayoutError::NoRooms);
        }
        // Exits refer to rooms by `id` when rooms have one, else 1-based number.
        let by_id: HashMap<String, usize> = rooms_v
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    text_of(r.get("id")).unwrap_or_else(|| (i + 1).to_string()),
                    i,
                )
            })
            .collect();
        let rooms = rooms_v
            .iter()
            .map(|r| {
                let name = text_of(r.get("name")).unwrap_or_default();
                let exits = r
                    .get("exits")
                    .and_then(Value::as_object)
                    .map(|o| {
                        o.values()
                            .filter_map(|e| by_id.get(&text_of(Some(e))?).copied())
                            .collect()
                    })
                    .unwrap_or_default();
                let raw: Vec<String> = r
                    .get("contents")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|c| text_of(Some(c))).collect())
                    .unwrap_or_default();
                let has = |needle: &str| raw.iter().any(|c| c.to_lowercase().contains(needle));
                LabRoom {
                    izaro: name.eq_ignore_ascii_case(IZARO_ROOM),
                    x: number_of(r.get("x")),
                    y: number_of(r.get("y")),
                    exits,
                    contents: RoomContents {
                        darkshrine: has("darkshrine"),
                        silver_key: has("key"),
                        silver_door: has("door"),
                        argus: has("argus"),
                        raw: raw.clone(),
                    },
                    name,
                }
            })
            .collect();
        Ok(Self {
            difficulty: text_of(v.get("difficulty")).and_then(|d| Difficulty::parse(&d)),
            date: text_of(v.get("date")),
            rooms,
        })
    }

    /// Shortest route (room indexes, excluding `from`) to the nearest room
    /// matching `goal`, following exits both ways.
    fn route_to(&self, from: usize, goal: impl Fn(usize) -> bool) -> Option<Vec<usize>> {
        let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); self.rooms.len()];
        for (i, r) in self.rooms.iter().enumerate() {
            for &e in &r.exits {
                neighbours[i].push(e);
                neighbours[e].push(i);
            }
        }
        let mut prev: Vec<Option<usize>> = vec![None; self.rooms.len()];
        let mut seen = vec![false; self.rooms.len()];
        seen[from] = true;
        let mut queue = VecDeque::from([from]);
        while let Some(i) = queue.pop_front() {
            if i != from && goal(i) {
                let mut path = vec![i];
                let mut cur = i;
                while let Some(p) = prev[cur] {
                    if p == from {
                        break;
                    }
                    path.push(p);
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            for &n in &neighbours[i] {
                if !seen[n] {
                    seen[n] = true;
                    prev[n] = Some(i);
                    queue.push_back(n);
                }
            }
        }
        None
    }
}

fn text_of(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn number_of(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// The loaded Lab Layout and the player's progress through it (persisted).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LabState {
    pub layout: Option<LabLayout>,
    /// Room index the player is in, `None` outside the lab rooms.
    pub current: Option<usize>,
    pub visited: BTreeSet<usize>,
    /// Route through unvisited darkshrines before the next Izaro fight.
    pub darkshrine_detour: bool,
    /// The last Area entered was the Aspirants' Plaza or a lab room.
    pub in_lab: bool,
    /// Difficulty of the lab being run (from Area IDs).
    pub running: Option<Difficulty>,
}

impl LabState {
    /// The player entered an Area.
    pub fn on_area(&mut self, area_id: &str, area_name: &str) {
        if area_id == "Labyrinth_Airlock" {
            // A fresh run starts at the plaza.
            self.in_lab = true;
            self.current = None;
            self.visited.clear();
            return;
        }
        let Some(difficulty) = Difficulty::of_area(area_id) else {
            self.in_lab = false;
            self.current = None;
            return;
        };
        self.in_lab = true;
        self.running = Some(difficulty);
        let Some(layout) = &self.layout else {
            return;
        };
        let named = |i: &usize| layout.rooms[*i].name.eq_ignore_ascii_case(area_name);
        let next = match self.current {
            // Prefer a matching exit of the room we're in: names repeat.
            Some(cur) => layout.rooms[cur]
                .exits
                .iter()
                .copied()
                .find(named)
                .or_else(|| {
                    // Doors are listed one way; also look at rooms leading here.
                    (0..layout.rooms.len())
                        .find(|i| named(i) && layout.rooms[*i].exits.contains(&cur))
                })
                .or_else(|| (cur + 1..layout.rooms.len()).find(named))
                .or_else(|| (0..layout.rooms.len()).find(named)),
            None => (0..layout.rooms.len()).find(named),
        };
        if let Some(i) = next {
            self.current = Some(i);
            self.visited.insert(i);
        }
    }

    pub fn load(&mut self, layout: LabLayout) {
        self.layout = Some(layout);
        self.current = None;
        self.visited.clear();
    }
}

/// What the lab Overlay draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabView {
    pub layout: Option<LabLayout>,
    pub current: Option<usize>,
    pub visited: Vec<usize>,
    /// Rooms to walk through next, in order, ending at an Izaro fight.
    pub route: Vec<usize>,
    pub darkshrine_detour: bool,
    pub in_lab: bool,
    pub running: Option<Difficulty>,
    /// The layout is for another day than `today`.
    pub outdated: bool,
    /// The layout is for another difficulty than the lab being run.
    pub wrong_difficulty: bool,
}

impl LabView {
    /// `today` is the current UTC date as `YYYY-MM-DD`.
    pub fn build(state: &LabState, today: &str) -> Self {
        let layout = state.layout.as_ref();
        let route = layout.map(|l| route(l, state)).unwrap_or_default();
        Self {
            outdated: layout
                .and_then(|l| l.date.as_deref())
                .is_some_and(|d| d != today),
            wrong_difficulty: match (layout.and_then(|l| l.difficulty), state.running) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            },
            layout: state.layout.clone(),
            current: state.current,
            visited: state.visited.iter().copied().collect(),
            route,
            darkshrine_detour: state.darkshrine_detour,
            in_lab: state.in_lab,
            running: state.running,
        }
    }
}

/// From the current room (or the entrance) to the next Izaro fight, via
/// unvisited darkshrines first when the detour is on.
fn route(layout: &LabLayout, state: &LabState) -> Vec<usize> {
    if layout.rooms.is_empty() {
        return Vec::new();
    }
    let (start, mut path) = match state.current {
        Some(c) => (c, Vec::new()),
        // Not in yet: the route starts with the entrance room.
        None => (0, vec![0]),
    };
    // The next fight is an Izaro room after the furthest one visited.
    let last_fight = state
        .visited
        .iter()
        .copied()
        .filter(|&i| layout.rooms[i].izaro)
        .max();
    let is_next_fight = |i: usize| {
        layout.rooms[i].izaro && last_fight.is_none_or(|f| i > f) && !state.visited.contains(&i)
    };
    let mut at = start;
    let mut visited: BTreeSet<usize> = state.visited.clone();
    if state.darkshrine_detour {
        let fight = layout
            .route_to(at, is_next_fight)
            .and_then(|p| p.last().copied());
        loop {
            let shrine = layout.route_to(at, |i| {
                layout.rooms[i].contents.darkshrine
                    && !visited.contains(&i)
                    && fight.is_none_or(|f| i < f)
                    && !layout.rooms[i].izaro
            });
            let Some(leg) = shrine else { break };
            at = *leg.last().expect("non-empty route");
            visited.extend(leg.iter().copied());
            path.extend(leg);
        }
    }
    if (!layout.rooms[at].izaro || at == start)
        && let Some(leg) = layout.route_to(at, is_next_fight)
    {
        path.extend(leg);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small lab: entrance(1) -> two ways to Aspirant's Trial(4) -> exit(5)
    /// -> final Aspirant's Trial(6). Room 2 has a darkshrine; names repeat.
    const LAYOUT: &str = r#"{
        "difficulty": "Normal", "date": "2026-11-19", "weapon": "Spears",
        "rooms": [
            {"id": "1", "name": "Estate Path", "x": 10, "y": 50, "exits": {"NE": "2", "SE": "3"}, "contents": []},
            {"id": "2", "name": "Estate Walkways", "x": 40, "y": 20, "exits": {"E": "4"}, "contents": ["darkshrine", "silver-key"]},
            {"id": "3", "name": "Estate Path", "x": 40, "y": 80, "exits": {"E": "4"}, "contents": ["argus"]},
            {"id": "4", "name": "Aspirant's Trial", "x": 70, "y": 50, "exits": {"E": "5"}, "contents": []},
            {"id": "5", "name": "Domain Crossing", "x": 90, "y": 50, "exits": {"E": "6"}, "contents": ["silver-door"]},
            {"id": "6", "name": "Aspirant's Trial", "x": 110, "y": 50, "exits": {}, "contents": []}
        ]
    }"#;

    fn state() -> LabState {
        let mut s = LabState::default();
        s.load(LabLayout::parse(LAYOUT).unwrap());
        s
    }

    #[test]
    fn parses_rooms_exits_and_contents() {
        let l = LabLayout::parse(LAYOUT).unwrap();
        assert_eq!(l.difficulty, Some(Difficulty::Normal));
        assert_eq!(l.date.as_deref(), Some("2026-11-19"));
        assert_eq!(l.rooms.len(), 6);
        assert_eq!(l.rooms[0].exits, [1, 2]);
        assert!(l.rooms[1].contents.darkshrine && l.rooms[1].contents.silver_key);
        assert!(l.rooms[2].contents.argus);
        assert!(l.rooms[4].contents.silver_door);
        assert!(l.rooms[3].izaro && l.rooms[5].izaro);
        assert!(looks_like_layout(LAYOUT));
        assert!(!looks_like_layout("hello"));
        assert_eq!(LabLayout::parse("{}"), Err(LayoutError::NoRooms));
        assert_eq!(LabLayout::parse("nope"), Err(LayoutError::NotJson));
    }

    #[test]
    fn exits_by_number_without_ids() {
        let l = LabLayout::parse(
            r#"{"difficulty":"Cruel","rooms":[{"name":"A","exits":{"E":2}},{"name":"B","exits":{}}]}"#,
        )
        .unwrap();
        assert_eq!(l.rooms[0].exits, [1]);
        assert_eq!(l.difficulty, Some(Difficulty::Cruel));
    }

    #[test]
    fn difficulty_from_area_ids() {
        assert_eq!(
            Difficulty::of_area("1_Labyrinth_OH_straight"),
            Some(Difficulty::Normal)
        );
        assert_eq!(
            Difficulty::of_area("3_Labyrinth_boss_2"),
            Some(Difficulty::Merciless)
        );
        assert_eq!(
            Difficulty::of_area("EndGame_Labyrinth_boss_1"),
            Some(Difficulty::Eternal)
        );
        assert_eq!(Difficulty::of_area("Labyrinth_Airlock"), None);
        assert_eq!(Difficulty::of_area("1_1_1"), None);
    }

    #[test]
    fn tracks_rooms_with_repeated_names() {
        let mut s = state();
        s.on_area("Labyrinth_Airlock", "Aspirants' Plaza");
        assert!(s.in_lab && s.current.is_none());
        s.on_area("1_Labyrinth_P_straight", "Estate Path");
        assert_eq!(s.current, Some(0));
        // Through the south door: "Estate Path" again, but it's room 3 (index 2).
        s.on_area("1_Labyrinth_P_branch", "Estate Path");
        assert_eq!(s.current, Some(2));
        s.on_area("1_Labyrinth_boss_1", "Aspirant's Trial");
        assert_eq!(s.current, Some(3));
        assert_eq!(s.running, Some(Difficulty::Normal));
        s.on_area("1_1_town", "Lioneye's Watch");
        assert!(!s.in_lab && s.current.is_none());
    }

    #[test]
    fn routes_to_the_next_izaro_fight() {
        let mut s = state();
        let v = LabView::build(&s, "2026-11-19");
        assert_eq!(v.route, [0, 1, 3], "from the entrance to the first fight");
        assert!(!v.outdated);

        s.on_area("1_Labyrinth_x", "Estate Path");
        s.on_area("1_Labyrinth_x", "Estate Path");
        s.on_area("1_Labyrinth_boss_1", "Aspirant's Trial");
        let v = LabView::build(&s, "2026-11-19");
        assert_eq!(v.route, [4, 5], "after the first fight, on to the second");
    }

    #[test]
    fn darkshrine_detour() {
        let mut s = state();
        s.darkshrine_detour = true;
        s.on_area("1_Labyrinth_x", "Estate Path");
        let v = LabView::build(&s, "2026-11-19");
        assert_eq!(v.route, [1, 3], "via the darkshrine room");
    }

    #[test]
    fn warns_about_stale_or_mismatched_layouts() {
        let mut s = state();
        assert!(LabView::build(&s, "2026-11-20").outdated);
        s.on_area("2_Labyrinth_x", "Estate Path");
        assert!(LabView::build(&s, "2026-11-19").wrong_difficulty);
    }
}
