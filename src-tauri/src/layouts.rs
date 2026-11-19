//! Zone layouts: the player's own images for the current Area, read from the
//! layouts folder and sent to the Overlay as data URLs.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use base64::{Engine, engine::general_purpose};
use poe_core::layouts::images_for;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{AppState, storage::data_dir};

/// Event carrying a fresh [`LayoutsPayload`].
const LAYOUTS_VIEW: &str = "layouts-view";

/// Where players put their layout images.
pub fn folder() -> PathBuf {
    data_dir().join("layouts")
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutImageView {
    pub label: String,
    /// `data:image/...;base64,...`
    pub src: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutsPayload {
    pub area_id: Option<String>,
    pub images: Vec<LayoutImageView>,
    pub visible: bool,
    /// How many images the folder holds in all (for the main window).
    pub total: usize,
    pub folder: String,
}

/// The images for the last Area looked up, so files are only read again
/// when the Area or the folder's contents change.
#[derive(Default)]
pub struct Cache {
    key: Option<(Option<String>, Vec<String>)>,
    images: Vec<LayoutImageView>,
    total: usize,
}

impl Cache {
    pub fn has_images(&self) -> bool {
        !self.images.is_empty()
    }
}

/// Every file under the folder (two levels deep), relative and `/`-separated.
fn list(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        if path.is_dir() {
            if let Ok(inner) = fs::read_dir(&path) {
                for f in inner.flatten() {
                    if f.path().is_file() {
                        out.push(format!("{name}/{}", f.file_name().to_string_lossy()));
                    }
                }
            }
        } else {
            out.push(name);
        }
    }
    out.sort();
    out
}

fn data_url(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    Some(format!(
        "data:{mime};base64,{}",
        general_purpose::STANDARD.encode(bytes)
    ))
}

fn payload(app: &AppHandle) -> LayoutsPayload {
    let state = app.state::<AppState>();
    let area = state.tracker.lock().unwrap().current_area();
    let dir = folder();
    let files = list(&dir);
    let mut cache = state.layouts.lock().unwrap();
    let key = (area.clone(), files.clone());
    if cache.key.as_ref() != Some(&key) {
        cache.images = area
            .as_deref()
            .map(|a| images_for(a, files.iter().map(String::as_str)))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|img| {
                Some(LayoutImageView {
                    src: data_url(&dir.join(&img.path))?,
                    label: img.label,
                })
            })
            .collect();
        cache.total = files.len();
        cache.key = Some(key);
    }
    LayoutsPayload {
        area_id: area,
        images: cache.images.clone(),
        visible: state.layouts_visible.load(Ordering::SeqCst),
        total: cache.total,
        folder: dir.to_string_lossy().into_owned(),
    }
}

/// What was last sent: Area, image count, visibility, folder size.
type Sent = (Option<String>, usize, bool, usize);

/// Sends the layouts when the Area, the images or visibility changed.
pub fn emit(app: &AppHandle) {
    static LAST: std::sync::Mutex<Option<Sent>> = std::sync::Mutex::new(None);
    let p = payload(app);
    let key = (p.area_id.clone(), p.images.len(), p.visible, p.total);
    let mut last = LAST.lock().unwrap();
    if last.as_ref() != Some(&key) {
        let _ = app.emit(LAYOUTS_VIEW, &p);
        *last = Some(key);
    }
}

#[tauri::command]
pub fn get_layouts_view(app: AppHandle) -> LayoutsPayload {
    payload(&app)
}

/// Open the layouts folder in Explorer (creating it first).
#[tauri::command]
pub fn open_layouts_folder() -> Result<(), String> {
    let dir = folder();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
