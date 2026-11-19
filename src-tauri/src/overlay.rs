//! Overlay windows: transparent, non-activating, positioned relative to the
//! game window and shown only while the game is focused (ADR 0005).

use std::collections::HashMap;

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::{
    platform::{self, GameRect, RawHwnd},
    storage::OverlayPlacement,
};

pub const TIMER: &str = "timer";
/// Covers the whole game window; draws over the in-game passive tree.
pub const TREE: &str = "tree";
pub const GUIDE: &str = "guide";
/// The full-act Guide view.
pub const ACT: &str = "act";
/// Gem links for the current level.
pub const GEMS: &str = "gems";
/// The Labyrinth map.
pub const LAB: &str = "lab";
/// Controls for the tree Overlay (which itself lets clicks through).
pub const TREEBAR: &str = "treebar";
/// The experience level ruler for the current zone.
pub const XP: &str = "xp";
/// The Build's notes from Path of Building.
pub const NOTES: &str = "notes";
/// Zone layout images for the current Area.
pub const LAYOUTS: &str = "layouts";

/// (label, logical width, logical height)
const WINDOWS: [(&str, f64, f64); 10] = [
    (TIMER, 280.0, 340.0),
    (TREE, 800.0, 600.0),
    (GUIDE, 380.0, 420.0),
    (ACT, 520.0, 720.0),
    (GEMS, 320.0, 420.0),
    (LAB, 780.0, 560.0),
    (TREEBAR, 860.0, 52.0),
    (XP, 380.0, 110.0),
    (NOTES, 420.0, 560.0),
    (LAYOUTS, 900.0, 260.0),
];

/// The Overlay's size at 100%, in logical pixels.
pub fn base_size(label: &str) -> Option<(f64, f64)> {
    WINDOWS
        .iter()
        .find(|(l, ..)| *l == label)
        .map(|(_, w, h)| (*w, *h))
}

/// Resize an Overlay and zoom its content (`scale` 1 = 100%). Overlays that
/// fit their content re-measure themselves afterwards.
pub fn apply_scale(app: &AppHandle, label: &str, scale: f64) {
    if label == TREE {
        return;
    }
    if let (Some(window), Some((w, h))) = (app.get_webview_window(label), base_size(label)) {
        let _ = window.set_zoom(scale);
        let _ = window.set_size(tauri::LogicalSize::new(w * scale, h * scale));
    }
}

pub fn create_overlays(app: &AppHandle, scale_of: impl Fn(&str) -> f64) -> tauri::Result<()> {
    for (label, w, h) in WINDOWS {
        let scale = if label == TREE { 1.0 } else { scale_of(label) };
        let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
            .title(format!("poe-overlay {label}"))
            .inner_size(w * scale, h * scale)
            .zoom_hotkeys_enabled(false)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .build()?;
        if scale != 1.0 {
            let _ = window.set_zoom(scale);
        }
        if let Some(h) = raw(&window) {
            platform::make_overlay(h);
        }
    }
    Ok(())
}

/// Put the small Overlays back above the full-window tree Overlay. Clicking
/// the tree Overlay's content can bring it to the front (the web view inside
/// it isn't covered by "never activate").
pub fn raise_small(app: &AppHandle) {
    for (label, ..) in WINDOWS {
        if label != TREE
            && let Some(h) = app.get_webview_window(label).as_ref().and_then(raw)
        {
            platform::raise(h);
        }
    }
}

fn raw(window: &WebviewWindow) -> Option<RawHwnd> {
    window.hwnd().ok().map(|h| h.0 as RawHwnd)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    /// Top-left at a fraction of the game window; the player can move it.
    At(OverlayPlacement),
    /// Top-right corner at a fraction of the game window, so a window that
    /// shrinks (the tree bar collapsing) stays put on its right side.
    RightAt(OverlayPlacement),
    /// Exactly over the game window.
    Cover,
}

/// What one Overlay should look like right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wanted {
    pub label: &'static str,
    pub anchor: Anchor,
    pub visible: bool,
    /// Takes the mouse instead of letting clicks through to the game.
    pub clickable: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Applied {
    visible: bool,
    clickable: bool,
    target: Option<(i32, i32, i32, i32)>,
}

/// Keeps Overlays and hotkeys in step with the game window.
pub struct Follower {
    shortcuts: Vec<Shortcut>,
    /// Hotkeys Windows refused (another app has them), as text.
    pub failed: Vec<String>,
    hotkeys_on: bool,
    applied: HashMap<&'static str, Applied>,
    was_interact: bool,
    last_rect: Option<GameRect>,
}

impl Follower {
    pub fn new(shortcuts: Vec<Shortcut>) -> Self {
        Self {
            shortcuts,
            failed: Vec::new(),
            hotkeys_on: false,
            applied: HashMap::new(),
            was_interact: false,
            last_rect: None,
        }
    }

    /// Swap in rebound hotkeys (re-registering them if they're live).
    pub fn set_shortcuts(&mut self, app: &AppHandle, shortcuts: Vec<Shortcut>) {
        if self.hotkeys_on {
            for s in &self.shortcuts {
                let _ = app.global_shortcut().unregister(*s);
            }
        }
        self.shortcuts = shortcuts;
        // Registered afresh on the next sync while the game is focused.
        self.hotkeys_on = false;
        self.failed.clear();
    }

    /// Returns the new placements of Overlays the player finished moving
    /// (when Interact Mode ends).
    pub fn sync(
        &mut self,
        app: &AppHandle,
        game: Option<RawHwnd>,
        interact: bool,
        wanted: &[Wanted],
    ) -> Vec<(&'static str, OverlayPlacement)> {
        let windows: Vec<(Wanted, WebviewWindow, RawHwnd)> = wanted
            .iter()
            .filter_map(|w| {
                let window = app.get_webview_window(w.label)?;
                let h = raw(&window)?;
                Some((*w, window, h))
            })
            .collect();

        let rect = game.and_then(platform::game_rect);
        // An Overlay may hold focus while being dragged or clicked.
        let foreground = platform::foreground_window();
        let active = rect.is_some()
            && game.is_some_and(|g| {
                foreground == g || windows.iter().any(|(_, _, h)| foreground == *h)
            });

        if active != self.hotkeys_on {
            // Hotkeys only exist while the game is focused, so they never
            // swallow keys in other apps.
            if active {
                // One at a time, so one taken hotkey doesn't block the rest.
                self.failed.clear();
                for s in &self.shortcuts {
                    if app.global_shortcut().register(*s).is_err() {
                        self.failed.push(s.into_string());
                    }
                }
            } else {
                for s in &self.shortcuts {
                    let _ = app.global_shortcut().unregister(*s);
                }
            }
            self.hotkeys_on = active;
        }

        let mut moved = Vec::new();
        let interact_ended = self.was_interact && !interact;
        self.was_interact = interact;
        // When the full-window Overlay changes, put the small ones back on
        // top of it (its clicks would otherwise land on the tree, not the bar).
        let mut cover_changed = false;

        for (w, window, h) in &windows {
            let applied = self.applied.entry(w.label).or_default();
            let movable = matches!(w.anchor, Anchor::At(_) | Anchor::RightAt(_));
            let right_anchored = matches!(w.anchor, Anchor::RightAt(_));

            if interact_ended
                && movable
                && let (Some(r), Ok(pos)) = (self.last_rect, window.outer_position())
            {
                // Remember where the player put it. Only pull it back if it
                // was dropped (mostly) outside the game window: windows can
                // be taller than what they show.
                const KEEP: i32 = 80;
                let size = window.outer_size().unwrap_or_default();
                let x = (pos.x - r.x).clamp(KEEP - size.width as i32, r.width - KEEP);
                let y = (pos.y - r.y).clamp(0, r.height - KEEP);
                let x = if right_anchored {
                    x + size.width as i32
                } else {
                    x
                };
                moved.push((
                    w.label,
                    OverlayPlacement {
                        x: f64::from(x) / f64::from(r.width),
                        y: f64::from(y) / f64::from(r.height),
                    },
                ));
                // Snaps back next tick if it was dropped outside.
                applied.target = None;
                continue;
            }

            let clickable = w.clickable || (interact && movable);
            if clickable != applied.clickable {
                platform::set_click_through(*h, !clickable);
                applied.clickable = clickable;
                cover_changed |= matches!(w.anchor, Anchor::Cover);
            }

            if let Some(r) = rect
                && !(interact && movable)
            {
                let target = match w.anchor {
                    Anchor::At(p) => (
                        r.x + (p.x * f64::from(r.width)).round() as i32,
                        r.y + (p.y * f64::from(r.height)).round() as i32,
                        0,
                        0,
                    ),
                    Anchor::RightAt(p) => {
                        let width = window.outer_size().map_or(0, |s| s.width as i32);
                        (
                            r.x + (p.x * f64::from(r.width)).round() as i32 - width,
                            r.y + (p.y * f64::from(r.height)).round() as i32,
                            0,
                            0,
                        )
                    }
                    Anchor::Cover => (r.x, r.y, r.width, r.height),
                };
                if applied.target != Some(target) {
                    let _ = window.set_position(PhysicalPosition::new(target.0, target.1));
                    if target.2 > 0 {
                        let _ =
                            window.set_size(PhysicalSize::new(target.2 as u32, target.3 as u32));
                    }
                    applied.target = Some(target);
                }
            }

            let visible = active && w.visible;
            if visible != applied.visible {
                platform::set_visible(*h, visible);
                applied.visible = visible;
                cover_changed |= visible && matches!(w.anchor, Anchor::Cover);
            }
        }
        if cover_changed {
            for (w, _, h) in &windows {
                if !matches!(w.anchor, Anchor::Cover) {
                    platform::raise(*h);
                }
            }
        }
        if rect.is_some() {
            self.last_rect = rect;
        }
        moved
    }
}
