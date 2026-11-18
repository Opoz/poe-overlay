//! Overlay windows: transparent, click-through, non-activating, positioned
//! relative to the game window and shown only while the game is focused
//! (ADR 0005).

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::{
    platform::{self, GameRect, RawHwnd},
    storage::OverlayPlacement,
};

pub const TIMER: &str = "timer";

pub fn create_timer_overlay(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, TIMER, WebviewUrl::App("index.html".into()))
        .title("poe-overlay timer")
        .inner_size(280.0, 340.0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()?;
    window.set_ignore_cursor_events(true)?;
    if let Some(h) = raw(&window) {
        platform::make_non_activating(h);
    }
    Ok(())
}

pub fn set_interact(app: &AppHandle, on: bool) {
    if let Some(w) = app.get_webview_window(TIMER) {
        let _ = w.set_ignore_cursor_events(!on);
        // Changing cursor handling rewrites the window's extended styles.
        if let Some(h) = raw(&w) {
            platform::make_non_activating(h);
        }
    }
}

fn raw(window: &WebviewWindow) -> Option<RawHwnd> {
    window.hwnd().ok().map(|h| h.0 as RawHwnd)
}

/// Keeps Overlays and hotkeys in step with the game window.
pub struct Follower {
    shortcuts: Vec<Shortcut>,
    active: bool,
    was_interact: bool,
    last_target: Option<(i32, i32)>,
    last_rect: Option<GameRect>,
}

impl Follower {
    pub fn new(shortcuts: Vec<Shortcut>) -> Self {
        Self {
            shortcuts,
            active: false,
            was_interact: false,
            last_target: None,
            last_rect: None,
        }
    }

    /// Returns a new placement when the player finished moving the Overlay.
    pub fn sync(
        &mut self,
        app: &AppHandle,
        game: Option<RawHwnd>,
        placement: OverlayPlacement,
        interact: bool,
    ) -> Option<OverlayPlacement> {
        let window = app.get_webview_window(TIMER)?;
        let overlay = raw(&window)?;
        let rect = game.and_then(platform::game_rect);
        // The Overlay itself may hold focus while being dragged in Interact Mode.
        let foreground = platform::foreground_window();
        let active =
            game.is_some_and(|g| foreground == g || foreground == overlay) && rect.is_some();

        if active != self.active {
            // Hotkeys only exist while the game is focused, so they never
            // swallow keys in other apps.
            let shortcuts = self.shortcuts.clone();
            let _ = if active {
                app.global_shortcut().register_multiple(shortcuts)
            } else {
                app.global_shortcut().unregister_multiple(shortcuts)
            };
            platform::set_visible(overlay, active);
            self.active = active;
        }

        let mut moved = None;
        if self.was_interact && !interact {
            // Interact Mode ended: remember where the player put it.
            if let (Some(r), Ok(pos)) = (self.last_rect, window.outer_position()) {
                let p = OverlayPlacement {
                    x: f64::from(pos.x - r.x) / f64::from(r.width),
                    y: f64::from(pos.y - r.y) / f64::from(r.height),
                };
                self.last_target = Some((pos.x, pos.y));
                moved = Some(p);
            }
        }
        self.was_interact = interact;

        if let Some(r) = rect {
            self.last_rect = Some(r);
            if !interact && moved.is_none() {
                let target = (
                    r.x + (placement.x * f64::from(r.width)).round() as i32,
                    r.y + (placement.y * f64::from(r.height)).round() as i32,
                );
                if self.last_target != Some(target) {
                    let _ = window.set_position(PhysicalPosition::new(target.0, target.1));
                    self.last_target = Some(target);
                }
            }
        }
        moved
    }
}
