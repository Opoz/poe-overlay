//! Atlas passive tree Overlay: rings the nodes of an imported atlas plan over
//! the in-game atlas tree. The Client Log doesn't record atlas points, so it
//! never knows what's taken: the rings leave the nodes' centres clear, and
//! the game's own lit / unlit nodes show what's done.

use base64::{Engine, engine::general_purpose};
use serde::{Deserialize, Serialize};

use crate::tree::{NodeKind, PassiveTree, Transform, TreeNode};

/// The atlas start: the hub in the middle of the tree every path leaves from.
pub fn atlas_start(tree: &PassiveTree) -> Option<&TreeNode> {
    tree.node(tree.classes.first()?.start_node?)
}

/// Which point the player is being asked to click, with the atlas tree
/// zoomed fully out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum AtlasCalibrating {
    /// One click on the atlas start, reusing the measured zoomed-out scale.
    Quick,
    Start,
    Partner {
        start_at: (f64, f64),
    },
}

/// An atlas plan: the nodes to take and mastery picks (node, effect).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AtlasPlan {
    pub nodes: Vec<u32>,
    pub masteries: Vec<(u32, u32)>,
}

/// The atlas Overlay's state (persisted between sessions).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AtlasOverlay {
    pub visible: bool,
    pub calibrating: Option<AtlasCalibrating>,
    pub transform: Option<Transform>,
    /// Fully zoomed out, the atlas tree's scale per pixel of game height,
    /// measured by a two-click Calibration.
    pub zoomed_out_scale_per_px: Option<f64>,
    pub plan: AtlasPlan,
}

/// The node the second Calibration click goes on: the atlas's topmost
/// keystone, far from the start and easy to spot.
pub fn calibration_partner(tree: &PassiveTree) -> Option<&TreeNode> {
    tree.nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Keystone)
        .min_by(|a, b| a.y.total_cmp(&b.y))
}

impl AtlasOverlay {
    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.calibrating = None;
        }
    }

    /// Cycle Calibration: one click (once the zoom is known) -> two clicks ->
    /// cancelled.
    pub fn toggle_calibration(&mut self) {
        self.calibrating = match self.calibrating {
            None if self.zoomed_out_scale_per_px.is_some() => Some(AtlasCalibrating::Quick),
            None | Some(AtlasCalibrating::Quick) => Some(AtlasCalibrating::Start),
            Some(_) => None,
        };
        if self.calibrating.is_some() {
            self.visible = true;
        }
    }

    /// Align from a click on the atlas start with the tree fully zoomed out.
    /// Needs the zoom measured once (two-click Calibration); `false` if not.
    pub fn align_on_start(&mut self, tree: &PassiveTree, at: (f64, f64), game_height: f64) -> bool {
        let (Some(per_px), Some(start)) = (self.zoomed_out_scale_per_px, atlas_start(tree)) else {
            return false;
        };
        let scale = per_px * game_height;
        self.transform = Some(Transform {
            scale,
            offset_x: at.0 - start.x * scale,
            offset_y: at.1 - start.y * scale,
        });
        self.calibrating = None;
        true
    }

    /// The player clicked `at` (game-window pixels) during Calibration.
    pub fn click(&mut self, tree: &PassiveTree, at: (f64, f64), game_height: f64) {
        match self.calibrating {
            Some(AtlasCalibrating::Quick) => {
                self.align_on_start(tree, at, game_height);
            }
            Some(AtlasCalibrating::Start) => {
                self.calibrating = Some(AtlasCalibrating::Partner { start_at: at });
            }
            Some(AtlasCalibrating::Partner { start_at }) => {
                let (Some(start), Some(partner)) = (atlas_start(tree), calibration_partner(tree))
                else {
                    self.calibrating = None;
                    return;
                };
                if let Some(t) = Transform::from_two_points(
                    (start.x, start.y),
                    start_at,
                    (partner.x, partner.y),
                    at,
                ) {
                    self.transform = Some(t);
                    self.calibrating = None;
                    if game_height > 0.0 {
                        self.zoomed_out_scale_per_px = Some(t.scale / game_height);
                    }
                }
            }
            None => {}
        }
    }
}

/// Read an atlas plan from a tree link or code: the official
/// `pathofexile.com/fullscreen-atlas-skill-tree/<code>` format (also used by
/// planners that link there). Nodes not on the atlas are dropped.
pub fn parse_plan(text: &str, tree: &PassiveTree) -> Result<AtlasPlan, String> {
    let text = text.trim();
    let code = text
        .rsplit('/')
        .next()
        .unwrap_or(text)
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('=')
        .replace('+', "-")
        .replace('/', "_");
    let bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(code.as_bytes())
        .map_err(|_| "That isn't an atlas tree link or code.".to_owned())?;
    let mut r = Reader {
        bytes: &bytes,
        at: 0,
    };
    let version = r.u32().ok_or("The tree code is too short.")?;
    if version < 4 {
        return Err(format!("Tree code version {version} isn't supported."));
    }
    r.u8(); // class
    r.u8(); // ascendancy
    let count = r.u8().ok_or("The tree code is cut short.")?;
    let mut nodes = Vec::new();
    for _ in 0..count {
        nodes.push(u32::from(r.u16().ok_or("The tree code is cut short.")?));
    }
    // Cluster jewel nodes (none on the atlas), then mastery picks.
    if let Some(clusters) = r.u8() {
        for _ in 0..clusters {
            r.u16();
        }
    }
    let mut masteries = Vec::new();
    if let Some(n) = r.u8() {
        for _ in 0..n {
            let (Some(a), Some(b)) = (r.u16(), r.u16()) else {
                break;
            };
            // The pair is (effect, node) or (node, effect): the node is the
            // one that's a mastery on the atlas.
            let (a, b) = (u32::from(a), u32::from(b));
            let is_mastery = |id| tree.node(id).is_some_and(|n| n.kind == NodeKind::Mastery);
            if is_mastery(b) {
                masteries.push((b, a));
            } else if is_mastery(a) {
                masteries.push((a, b));
            }
        }
    }
    nodes.retain(|id| tree.node(*id).is_some());
    for (node, _) in &masteries {
        if !nodes.contains(node) {
            nodes.push(*node);
        }
    }
    if nodes.is_empty() {
        return Err("That link has no atlas nodes in it.".to_owned());
    }
    Ok(AtlasPlan { nodes, masteries })
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Option<u8> {
        let b = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(b)
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from(self.u8()?) << 8 | u16::from(self.u8()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from(self.u16()?) << 16 | u32::from(self.u16()?))
    }
}

/// What the atlas Overlay draws.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasView {
    pub visible: bool,
    pub calibrating: Option<AtlasCalibrating>,
    pub transform: Option<Transform>,
    pub start: Option<TreeNode>,
    pub partner: Option<TreeNode>,
    /// Planned nodes (including masteries), to ring.
    pub planned: Vec<u32>,
    /// Whether a one-click Calibration is possible (zoom measured).
    pub zoom_known: bool,
}

impl AtlasView {
    pub fn build(tree: &PassiveTree, overlay: &AtlasOverlay) -> Self {
        Self {
            visible: overlay.visible,
            calibrating: overlay.calibrating,
            transform: overlay.transform,
            start: atlas_start(tree).cloned(),
            partner: calibration_partner(tree).cloned(),
            planned: overlay.plan.nodes.clone(),
            zoom_known: overlay.zoomed_out_scale_per_px.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(nodes: &[u16], masteries: &[(u16, u16)]) -> String {
        let mut b = vec![0, 0, 0, 6, 0, 0, nodes.len() as u8];
        for n in nodes {
            b.extend(n.to_be_bytes());
        }
        b.push(0);
        b.push(masteries.len() as u8);
        for (x, y) in masteries {
            b.extend(x.to_be_bytes());
            b.extend(y.to_be_bytes());
        }
        general_purpose::URL_SAFE_NO_PAD.encode(b)
    }

    #[test]
    fn reads_an_official_atlas_link() {
        let tree = PassiveTree::atlas();
        let notable = tree
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Notable)
            .unwrap()
            .id;
        let mastery = tree
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Mastery)
            .unwrap()
            .id;
        let link = format!(
            "https://www.pathofexile.com/fullscreen-atlas-skill-tree/{}?accountName=x",
            code(&[notable as u16, 65000], &[(7, mastery as u16)])
        );
        let plan = parse_plan(&link, &tree).unwrap();
        assert_eq!(
            plan.nodes,
            [notable, mastery],
            "unknown node dropped, mastery added"
        );
        assert_eq!(plan.masteries, [(mastery, 7)]);
        assert!(parse_plan("not a tree", &tree).is_err());
    }

    #[test]
    fn two_clicks_measure_the_zoom_then_one_click_aligns() {
        let tree = PassiveTree::atlas();
        let start = atlas_start(&tree).unwrap().clone();
        let partner = calibration_partner(&tree).unwrap().clone();
        let mut o = AtlasOverlay::default();
        o.toggle_calibration();
        assert_eq!(
            o.calibrating,
            Some(AtlasCalibrating::Start),
            "no zoom yet: two clicks"
        );
        let scale = 0.05;
        let screen = |n: &TreeNode| (n.x * scale + 900.0, n.y * scale + 1200.0);
        o.click(&tree, screen(&start), 1440.0);
        o.click(&tree, screen(&partner), 1440.0);
        let t = o.transform.unwrap();
        assert!((t.scale - scale).abs() < 1e-9);
        o.toggle_calibration();
        assert_eq!(o.calibrating, Some(AtlasCalibrating::Quick));
        o.click(&tree, (100.0, 200.0), 1440.0);
        let (x, y) = o.transform.unwrap().apply(start.x, start.y);
        assert!((x - 100.0).abs() < 1e-6 && (y - 200.0).abs() < 1e-6);
    }
}
