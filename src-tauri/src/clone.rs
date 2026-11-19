//! Screen copies ("clone frames"): live copies of parts of the game window
//! (flasks, buffs, a skill's cooldown) shown somewhere else, bigger or
//! fainter. Windows' compositor draws them ([`platform::Thumbnail`]); this
//! app never reads the pixels and never acts on them.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::{
    overlay,
    platform::{self, GameRect, RawHwnd, Thumbnail},
    storage::OverlayPlacement,
};

/// Window labels, one per possible screen copy (labels must be `'static`).
pub const LABELS: [&str; 8] = [
    "clone-0", "clone-1", "clone-2", "clone-3", "clone-4", "clone-5", "clone-6", "clone-7",
];

/// The full-window Overlay for dragging out a screen copy's source.
pub const PICKER: &str = "picker";

/// One screen copy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CloneFrame {
    /// Index into [`LABELS`].
    pub slot: usize,
    pub name: String,
    /// What to copy: x, y, width, height as fractions of the game window.
    pub source: [f64; 4],
    /// Where the copy's top-left corner goes.
    pub target: OverlayPlacement,
    /// Size of the copy relative to the source.
    pub scale: f64,
    /// 0.2-1.
    pub opacity: f64,
    pub enabled: bool,
}

impl Default for CloneFrame {
    fn default() -> Self {
        Self {
            slot: 0,
            name: String::new(),
            source: [0.0; 4],
            target: OverlayPlacement { x: 0.4, y: 0.4 },
            scale: 1.5,
            opacity: 1.0,
            enabled: false,
        }
    }
}

impl CloneFrame {
    /// Whether a source area has been picked.
    pub fn has_source(&self) -> bool {
        self.source[2] > 0.0 && self.source[3] > 0.0
    }

    /// The source in game client pixels, and the copy's size in pixels.
    pub fn geometry(&self, game: &GameRect) -> ((i32, i32, i32, i32), (i32, i32)) {
        let [x, y, w, h] = self.source;
        let (gw, gh) = (f64::from(game.width), f64::from(game.height));
        let src = (
            (x * gw).round() as i32,
            (y * gh).round() as i32,
            (w * gw).round().max(1.0) as i32,
            (h * gh).round().max(1.0) as i32,
        );
        let size = (
            (f64::from(src.2) * self.scale).round() as i32,
            (f64::from(src.3) * self.scale).round() as i32,
        );
        (src, size)
    }
}

/// The first slot no screen copy uses.
pub fn free_slot(frames: &[CloneFrame]) -> Option<usize> {
    (0..LABELS.len()).find(|s| frames.iter().all(|f| f.slot != *s))
}

/// What a copy was last set to: source rect, size, opacity, Interact Mode.
type Applied = ((i32, i32, i32, i32), (i32, i32), u8, bool);

/// One copy's compositor link to the game window.
struct Live {
    game: RawHwnd,
    thumbnail: Thumbnail,
    applied: Option<Applied>,
}

/// Keeps one window and compositor copy per enabled screen copy.
#[derive(Default)]
pub struct Copies {
    live: HashMap<usize, Live>,
}

impl Copies {
    /// Create, size and point the copies' windows; drop copies that went away.
    pub fn sync(
        &mut self,
        app: &AppHandle,
        game: Option<RawHwnd>,
        rect: Option<GameRect>,
        frames: &[CloneFrame],
        interact: bool,
    ) {
        let wanted: Vec<&CloneFrame> = frames
            .iter()
            .filter(|f| f.enabled && f.has_source() && f.slot < LABELS.len())
            .collect();
        self.live
            .retain(|slot, _| wanted.iter().any(|f| f.slot == *slot) && game.is_some());
        let (Some(game), Some(rect)) = (game, rect) else {
            return;
        };
        for f in wanted {
            let label = LABELS[f.slot];
            let window = match app.get_webview_window(label) {
                Some(w) => w,
                None => match create(app, label) {
                    Some(w) => w,
                    None => continue,
                },
            };
            let Some(dest) = window.hwnd().ok().map(|h| h.0 as RawHwnd) else {
                continue;
            };
            if self.live.get(&f.slot).is_some_and(|l| l.game != game) {
                self.live.remove(&f.slot);
            }
            let live = match self.live.entry(f.slot) {
                std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    let Some(thumbnail) = Thumbnail::register(dest, game) else {
                        continue;
                    };
                    e.insert(Live {
                        game,
                        thumbnail,
                        applied: None,
                    })
                }
            };
            let (src, size) = f.geometry(&rect);
            let opacity = (f.opacity.clamp(0.2, 1.0) * 255.0).round() as u8;
            let state = (src, size, opacity, interact);
            if live.applied != Some(state) {
                let _ = window.set_size(PhysicalSize::new(
                    size.0.max(1) as u32,
                    size.1.max(1) as u32,
                ));
                // In Interact Mode, inset the copy so the frame's outline
                // (drawn by its web view) shows around it.
                let inset = if interact { 3 } else { 0 };
                live.thumbnail.update(
                    src,
                    (inset, inset, size.0 - 2 * inset, size.1 - 2 * inset),
                    opacity,
                );
                live.applied = Some(state);
            }
        }
    }
}

fn create(app: &AppHandle, label: &str) -> Option<tauri::WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(format!("poe-overlay {label}"))
        .inner_size(100.0, 100.0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()
        .ok()?;
    if let Ok(h) = window.hwnd() {
        platform::make_overlay(h.0 as RawHwnd);
    }
    Some(window)
}

/// The Overlays to keep in place for the screen copies.
pub fn wanted(frames: &[CloneFrame]) -> Vec<overlay::Wanted> {
    frames
        .iter()
        .filter(|f| f.slot < LABELS.len())
        .map(|f| overlay::Wanted {
            label: LABELS[f.slot],
            anchor: overlay::Anchor::At(f.target),
            visible: f.enabled && f.has_source(),
            clickable: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_and_size_follow_the_game_window() {
        let f = CloneFrame {
            source: [0.1, 0.9, 0.05, 0.04],
            scale: 2.0,
            enabled: true,
            ..Default::default()
        };
        let game = GameRect {
            x: 0,
            y: 0,
            width: 2000,
            height: 1000,
        };
        assert_eq!(f.geometry(&game), ((200, 900, 100, 40), (200, 80)));
    }

    #[test]
    fn slots_fill_from_the_first_free_one() {
        let used = |s| CloneFrame {
            slot: s,
            ..Default::default()
        };
        assert_eq!(free_slot(&[]), Some(0));
        assert_eq!(free_slot(&[used(0), used(2)]), Some(1));
        let full: Vec<CloneFrame> = (0..8).map(used).collect();
        assert_eq!(free_slot(&full), None);
    }
}
