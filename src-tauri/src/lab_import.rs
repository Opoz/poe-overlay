//! Loading a poelab Lab Layout, Exile UI style: open poelab in the browser,
//! then pick up the compass file from the clipboard.
//!
//! The clipboard is only read during an import the player started, and only
//! while the game isn't focused (they're in the browser). poelab sits behind
//! a browser check, so copying the compass *link* opens it in the browser;
//! copying the JSON text it shows (Ctrl+A, Ctrl+C) loads it.

use std::time::{Duration, Instant};

use poe_core::lab::{LabLayout, looks_like_layout};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::AppState;

const POELAB: &str = "https://www.poelab.com/";
const GIVE_UP_AFTER: Duration = Duration::from_secs(600);
/// Event: progress of an import, for the lab Overlay.
pub const LAB_IMPORT: &str = "lab-import";

pub struct LabImport {
    started: Instant,
    /// Clipboard text already looked at (starts as whatever was there).
    last_clip: Option<String>,
    /// The player has left the game since the import started.
    left_game: bool,
    opened_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStatus {
    /// watching / opened / loaded / failed / stopped
    pub state: &'static str,
    pub message: String,
}

fn status(app: &AppHandle, state: &'static str, message: impl Into<String>) {
    let _ = app.emit(
        LAB_IMPORT,
        ImportStatus {
            state,
            message: message.into(),
        },
    );
}

pub fn open_url(url: &str) {
    // `explorer <url>` opens the default browser.
    let _ = std::process::Command::new("explorer").arg(url).spawn();
}

/// Open poelab and start watching for the compass file.
pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    let current = app.clipboard().read_text().ok();
    *state.lab_import.lock().unwrap() = Some(LabImport {
        started: Instant::now(),
        last_clip: current,
        left_game: false,
        opened_links: Vec::new(),
    });
    open_url(POELAB);
    status(
        app,
        "watching",
        "On poelab, open today's lab and copy the lab compass file link (or its text).",
    );
}

/// Load whatever is on the clipboard now (the "Load copied layout" button).
pub fn load_clipboard(app: &AppHandle) {
    match app.clipboard().read_text() {
        Ok(text) if looks_like_layout(&text) => load_text(app, &text),
        Ok(text) if is_compass_link(&text) => {
            open_url(text.trim());
            status(
                app,
                "opened",
                "Opened the compass file: copy its text (Ctrl+A, Ctrl+C), then load again.",
            );
        }
        _ => status(
            app,
            "failed",
            "The clipboard doesn't hold a lab compass file.",
        ),
    }
}

fn is_compass_link(text: &str) -> bool {
    let t = text.trim();
    t.contains("poelab.com") && t.ends_with(".json") && !t.contains(char::is_whitespace)
}

fn load_text(app: &AppHandle, text: &str) {
    match LabLayout::parse(text) {
        Ok(layout) => {
            let summary = format!(
                "Loaded {} lab for {} ({} rooms).",
                layout
                    .difficulty
                    .map_or("a".to_owned(), |d| format!("{d:?}").to_lowercase()),
                layout
                    .date
                    .clone()
                    .unwrap_or_else(|| "an unknown day".into()),
                layout.rooms.len()
            );
            app.state::<AppState>()
                .tracker
                .lock()
                .unwrap()
                .load_lab(layout);
            *app.state::<AppState>().lab_import.lock().unwrap() = None;
            status(app, "loaded", summary);
            crate::emit_lab(app);
        }
        Err(e) => status(app, "failed", e.to_string()),
    }
}

/// Called from the tracking loop.
pub fn tick(app: &AppHandle, game_focused: bool) {
    let state = app.state::<AppState>();
    let mut guard = state.lab_import.lock().unwrap();
    let Some(import) = guard.as_mut() else {
        return;
    };
    if import.started.elapsed() > GIVE_UP_AFTER {
        *guard = None;
        drop(guard);
        status(app, "stopped", "Stopped waiting for a lab layout.");
        return;
    }
    if game_focused {
        if import.left_game {
            // Back in the game without a layout: stop watching.
            *guard = None;
            drop(guard);
            status(app, "stopped", "Back in game; no layout was copied.");
        }
        return;
    }
    import.left_game = true;
    let Ok(text) = app.clipboard().read_text() else {
        return;
    };
    if import.last_clip.as_deref() == Some(text.as_str()) {
        return;
    }
    import.last_clip = Some(text.clone());

    if looks_like_layout(&text) {
        drop(guard);
        load_text(app, &text);
    } else if is_compass_link(&text) {
        let link = text.trim().to_owned();
        if import.opened_links.contains(&link) {
            return;
        }
        import.opened_links.push(link.clone());
        drop(guard);
        // Try downloading it; poelab usually blocks this, so fall back to
        // opening it in the browser for the player to copy.
        let app = app.clone();
        std::thread::spawn(move || {
            let fetched = ureq::get(&link)
                .call()
                .ok()
                .and_then(|mut r| r.body_mut().read_to_string().ok())
                .filter(|t| looks_like_layout(t));
            match fetched {
                Some(text) => load_text(&app, &text),
                None => {
                    open_url(&link);
                    status(
                        &app,
                        "opened",
                        "Opened the compass file: select all and copy its text (Ctrl+A, Ctrl+C).",
                    );
                }
            }
        });
    }
}
