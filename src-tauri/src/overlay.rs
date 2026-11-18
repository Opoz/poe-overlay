//! Overlay windows: transparent, click-through, non-activating, positioned
//! relative to the game window and shown only while the game is focused
//! (ADR 0005).

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

fn build(app: &AppHandle, label: &str, w: f64, h: f64, opacity: f64) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(format!("poe-overlay {label}"))
        .inner_size(w, h)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()?;
    if let Some(h) = raw(&window) {
        platform::make_overlay(h, opacity);
    }
    Ok(())
}

pub fn create_overlays(app: &AppHandle, opacity: f64) -> tauri::Result<()> {
    build(app, TIMER, 280.0, 340.0, opacity)?;
    build(app, TREE, 800.0, 600.0, 1.0)?;
    Ok(())
}

fn raw(window: &WebviewWindow) -> Option<RawHwnd> {
    window.hwnd().ok().map(|h| h.0 as RawHwnd)
}

fn raw_of(app: &AppHandle, label: &str) -> Option<RawHwnd> {
    app.get_webview_window(label).as_ref().and_then(raw)
}

/// What the Overlays should look like right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wanted {
    pub timer_placement: OverlayPlacement,
    /// Interact Mode: the timer can be dragged.
    pub interact: bool,
    pub tree_visible: bool,
    /// Calibration needs the tree Overlay to take clicks.
    pub tree_clickable: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Applied {
    active: bool,
    timer_visible: bool,
    tree_visible: bool,
    timer_clickable: bool,
    tree_clickable: bool,
}

/// Keeps Overlays and hotkeys in step with the game window.
pub struct Follower {
    shortcuts: Vec<Shortcut>,
    applied: Applied,
    was_interact: bool,
    timer_target: Option<(i32, i32)>,
    tree_rect: Option<GameRect>,
    last_rect: Option<GameRect>,
}

impl Follower {
    pub fn new(shortcuts: Vec<Shortcut>) -> Self {
        Self {
            shortcuts,
            applied: Applied::default(),
            was_interact: false,
            timer_target: None,
            tree_rect: None,
            last_rect: None,
        }
    }

    /// Returns a new timer placement when the player finished moving it.
    pub fn sync(
        &mut self,
        app: &AppHandle,
        game: Option<RawHwnd>,
        wanted: Wanted,
    ) -> Option<OverlayPlacement> {
        let timer_window = app.get_webview_window(TIMER)?;
        let timer = raw(&timer_window)?;
        let tree = raw_of(app, TREE)?;
        let rect = game.and_then(platform::game_rect);
        // An Overlay may hold focus while being dragged or clicked.
        let foreground = platform::foreground_window();
        let active = rect.is_some()
            && game.is_some_and(|g| foreground == g || foreground == timer || foreground == tree);

        let next = Applied {
            active,
            timer_visible: active,
            tree_visible: active && wanted.tree_visible,
            timer_clickable: wanted.interact,
            tree_clickable: wanted.tree_clickable,
        };
        if next.active != self.applied.active {
            // Hotkeys only exist while the game is focused, so they never
            // swallow keys in other apps.
            let shortcuts = self.shortcuts.clone();
            let _ = if active {
                app.global_shortcut().register_multiple(shortcuts)
            } else {
                app.global_shortcut().unregister_multiple(shortcuts)
            };
        }
        if next.timer_clickable != self.applied.timer_clickable {
            platform::set_click_through(timer, !next.timer_clickable);
        }
        if next.tree_clickable != self.applied.tree_clickable {
            platform::set_click_through(tree, !next.tree_clickable);
        }

        let mut moved = None;
        if self.was_interact && !wanted.interact {
            // Interact Mode ended: remember where the player put the timer.
            if let (Some(r), Ok(pos)) = (self.last_rect, timer_window.outer_position()) {
                moved = Some(OverlayPlacement {
                    x: f64::from(pos.x - r.x) / f64::from(r.width),
                    y: f64::from(pos.y - r.y) / f64::from(r.height),
                });
                self.timer_target = Some((pos.x, pos.y));
            }
        }
        self.was_interact = wanted.interact;

        if let Some(r) = rect {
            self.last_rect = Some(r);
            if !wanted.interact && moved.is_none() {
                let p = wanted.timer_placement;
                let target = (
                    r.x + (p.x * f64::from(r.width)).round() as i32,
                    r.y + (p.y * f64::from(r.height)).round() as i32,
                );
                if self.timer_target != Some(target) {
                    let _ = timer_window.set_position(PhysicalPosition::new(target.0, target.1));
                    self.timer_target = Some(target);
                }
            }
            if self.tree_rect != Some(r)
                && let Some(w) = app.get_webview_window(TREE)
            {
                let _ = w.set_position(PhysicalPosition::new(r.x, r.y));
                let _ = w.set_size(PhysicalSize::new(r.width as u32, r.height as u32));
                self.tree_rect = Some(r);
            }
        }

        if next.timer_visible != self.applied.timer_visible {
            platform::set_visible(timer, next.timer_visible);
        }
        if next.tree_visible != self.applied.tree_visible {
            platform::set_visible(tree, next.tree_visible);
        }
        self.applied = next;
        moved
    }
}
