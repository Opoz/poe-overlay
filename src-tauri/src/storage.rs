//! Settings (TOML) and saved state (JSON) under `%APPDATA%\com.opoz.poe-overlay`
//! (the app identifier; plain `poe-overlay` is taken by an unrelated app).

use std::{
    fs,
    path::{Path, PathBuf},
};

use poe_core::run::{Run, TimerState};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from);
    base.join("com.opoz.poe-overlay")
}

/// Where an Overlay sits, as fractions of the game's client area, so it
/// survives resolution and monitor changes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPlacement {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Manual Client Log path; auto-detected from the game process when unset.
    pub log_path: Option<PathBuf>,
    /// Run Category for new Runs.
    pub category: String,
    pub pause_in_hideout: bool,
    pub timer_overlay: OverlayPlacement,
    pub overlay_opacity: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            log_path: None,
            category: "Default".into(),
            pause_in_hideout: true,
            timer_overlay: OverlayPlacement { x: 0.01, y: 0.25 },
            overlay_opacity: 1.0,
        }
    }
}

/// What lets the app pick up where it left off.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Session {
    pub log_path: Option<PathBuf>,
    /// Byte offset into `log_path` already processed.
    pub log_offset: u64,
    /// Profiles, the active Character and any pending Run.
    pub timer: TimerState,
}

pub struct Storage {
    dir: PathBuf,
}

impl Storage {
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }

    pub fn load_settings(&self) -> Settings {
        fs::read_to_string(self.dir.join("settings.toml"))
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) {
        if let Ok(text) = toml::to_string_pretty(settings) {
            write_atomic(&self.dir.join("settings.toml"), &text);
        }
    }

    pub fn load_session(&self) -> Session {
        load_json(&self.dir.join("session.json")).unwrap_or_default()
    }

    pub fn save_session(&self, session: &Session) {
        save_json(&self.dir.join("session.json"), session);
    }

    pub fn load_history(&self) -> Vec<Run> {
        load_json(&self.dir.join("runs.json")).unwrap_or_default()
    }

    pub fn save_history(&self, runs: &[Run]) {
        save_json(&self.dir.join("runs.json"), &runs);
    }
}

fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn save_json<T: Serialize>(path: &Path, value: &T) {
    if let Ok(text) = serde_json::to_string_pretty(value) {
        write_atomic(path, &text);
    }
}

/// Write via a temp file so a crash never leaves a half-written file.
fn write_atomic(path: &Path, text: &str) {
    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, text).is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}
