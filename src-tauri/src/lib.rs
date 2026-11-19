mod hotkeys;
mod lab_import;
mod overlay;
mod platform;
mod storage;
mod tracker;

use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use poe_core::{
    gems::GemsView,
    guide::{GuideCommand, GuideOptions, GuideView},
    history::HistoryView,
    lab::LabView,
    run::Command,
    tree::{TreeNode, TreeView},
    view::TimerView,
};
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

use crate::{
    hotkeys::Hotkey,
    storage::{Settings, Storage, data_dir},
    tracker::Tracker,
};

pub struct AppState {
    tracker: Mutex<Tracker>,
    settings: Mutex<Settings>,
    /// Interact Mode: Overlays accept the mouse.
    interact: AtomicBool,
    /// The full-act Guide view is open.
    act_view: AtomicBool,
    /// The gem links Overlay is showing.
    gems_visible: AtomicBool,
    /// Which Vendor Regex chunk the next Alt+R copies, and for which list.
    regex_cursor: Mutex<(Vec<String>, usize)>,
    /// A lab layout import in progress (watching the clipboard).
    lab_import: Mutex<Option<lab_import::LabImport>>,
    /// The player toggled the lab Overlay; `None` = automatic (shown in the
    /// lab). Reset on entering or leaving the lab.
    lab_manual: Mutex<Option<bool>>,
    /// The act shown in the full-act view, when browsing another act.
    act_browse: Mutex<Option<u8>>,
    /// Hotkeys another app has taken.
    hotkey_failures: Mutex<Vec<String>>,
    /// The tree Overlay was hidden by the player (Hide button / Alt+T) while
    /// the game's tree was open: keep a "Show tree" button. Closing the game's
    /// tree (its key or Escape) hides the bar entirely.
    tree_pill: AtomicBool,
    /// The XP level ruler Overlay is showing.
    xp_visible: AtomicBool,
    /// The timer and Guide Overlays are showing (the player can turn them off).
    timer_visible: AtomicBool,
    guide_visible: AtomicBool,
    /// The Build notes Overlay is showing.
    notes_visible: AtomicBool,
    /// Bound hotkeys (from Settings), and whether they changed since the
    /// tracking loop last registered them.
    bindings: Mutex<Vec<(Shortcut, Hotkey)>>,
    hotkeys_changed: AtomicBool,
}

/// Event carrying a fresh [`TimerView`] to every window.
const TIMER_VIEW: &str = "timer-view";
/// Event carrying the [`Status`] to the main window.
const STATUS: &str = "status";
const INTERACT: &str = "interact-mode";
/// Event carrying a fresh [`TreeView`] to every window.
const TREE_VIEW: &str = "tree-view";
/// Event carrying a fresh [`GuideView`] (plus whether the act view is open).
const GUIDE_VIEW: &str = "guide-view";
/// Event carrying a fresh [`GemsView`] (plus whether it's showing).
const GEMS_VIEW: &str = "gems-view";
/// Event: a Vendor Regex was copied (or there was nothing to copy).
const REGEX_COPIED: &str = "regex-copied";
/// Event carrying a fresh [`LabView`] (plus whether it's showing).
const LAB_VIEW: &str = "lab-view";
/// Event: hotkeys another app has taken.
const HOTKEY_FAILURES: &str = "hotkey-failures";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    log_path: Option<String>,
    game_running: bool,
}

fn on_hotkey(app: &AppHandle, hotkey: Hotkey) {
    let state = app.state::<AppState>();
    match hotkey {
        Hotkey::Timer(command) => {
            state.tracker.lock().unwrap().command(command);
            emit_view(app);
        }
        Hotkey::InteractMode => {
            let on = !state.interact.fetch_xor(true, Ordering::SeqCst);
            let _ = app.emit(INTERACT, on);
        }
        Hotkey::TreeOverlay => {
            let mut tracker = state.tracker.lock().unwrap();
            tracker.toggle_tree();
            let hidden = !tracker.tree_overlay().visible;
            drop(tracker);
            state.tree_pill.store(hidden, Ordering::SeqCst);
            emit_tree(app);
        }
        Hotkey::Calibrate => {
            state.tracker.lock().unwrap().toggle_calibration();
            emit_tree(app);
        }
        Hotkey::Guide(command) => run_guide_command(app, command),
        Hotkey::ActView => {
            state.act_view.fetch_xor(true, Ordering::SeqCst);
            *state.act_browse.lock().unwrap() = None;
            emit_guide(app);
        }
        Hotkey::TreeStage(delta) => {
            state.tracker.lock().unwrap().cycle_tree_stage(delta);
            emit_tree(app);
        }
        Hotkey::GemLinks => {
            state.gems_visible.fetch_xor(true, Ordering::SeqCst);
            emit_gems(app);
        }
        Hotkey::VendorRegex => copy_vendor_regex(app),
        Hotkey::LabOverlay => {
            let shown = lab_visible(app);
            *state.lab_manual.lock().unwrap() = Some(!shown);
            emit_lab(app);
        }
        Hotkey::LabImport => lab_import::start(app),
        Hotkey::XpRuler => {
            state.xp_visible.fetch_xor(true, Ordering::SeqCst);
        }
        Hotkey::BuildNotes => {
            state.notes_visible.fetch_xor(true, Ordering::SeqCst);
            emit_notes(app);
        }
    }
}

/// Shown when toggled on, else automatically while in the lab.
fn lab_visible(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let manual = *state.lab_manual.lock().unwrap();
    manual.unwrap_or_else(|| state.tracker.lock().unwrap().lab_view().in_lab)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LabPayload {
    #[serde(flatten)]
    view: LabView,
    visible: bool,
}

fn lab_payload(app: &AppHandle) -> LabPayload {
    let view = app.state::<AppState>().tracker.lock().unwrap().lab_view();
    LabPayload {
        view,
        visible: lab_visible(app),
    }
}

/// Sends the lab view only when it changed.
pub(crate) fn emit_lab(app: &AppHandle) {
    static LAST: Mutex<String> = Mutex::new(String::new());
    let payload = lab_payload(app);
    let Ok(json) = serde_json::to_string(&payload) else {
        return;
    };
    let mut last = LAST.lock().unwrap();
    if *last != json {
        let _ = app.emit(LAB_VIEW, payload);
        *last = json;
    }
}

#[tauri::command]
fn get_lab_view(app: AppHandle) -> LabPayload {
    lab_payload(&app)
}

#[tauri::command]
fn lab_open(app: AppHandle) {
    lab_import::start(&app);
}

#[tauri::command]
fn lab_load_clipboard(app: AppHandle) {
    lab_import::load_clipboard(&app);
}

#[tauri::command]
fn lab_toggle(app: AppHandle, state: tauri::State<AppState>) {
    let shown = lab_visible(&app);
    *state.lab_manual.lock().unwrap() = Some(!shown);
    emit_lab(&app);
}

#[tauri::command]
fn set_lab_detour(app: AppHandle, state: tauri::State<AppState>, on: bool) {
    state.tracker.lock().unwrap().set_lab_detour(on);
    emit_lab(&app);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegexCopied {
    text: Option<String>,
    /// 1-based part, when the gems need several searches.
    part: usize,
    of: usize,
}

/// Copy the current Step's Vendor Regex; repeated presses cycle through the
/// parts when the gems don't fit in one search.
fn copy_vendor_regex(app: &AppHandle) {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let state = app.state::<AppState>();
    let regexes = state.tracker.lock().unwrap().guide_view(None).vendor_regex;
    let mut cursor = state.regex_cursor.lock().unwrap();
    if cursor.0 != regexes {
        *cursor = (regexes.clone(), 0);
    }
    let payload = match regexes.get(cursor.1) {
        Some(text) => {
            let _ = app.clipboard().write_text(text.clone());
            let part = cursor.1 + 1;
            cursor.1 = part % regexes.len();
            RegexCopied {
                text: Some(text.clone()),
                part,
                of: regexes.len(),
            }
        }
        None => RegexCopied {
            text: None,
            part: 0,
            of: 0,
        },
    };
    let _ = app.emit(REGEX_COPIED, payload);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GemsPayload {
    #[serde(flatten)]
    view: GemsView,
    visible: bool,
}

fn gems_payload(app: &AppHandle) -> GemsPayload {
    let state = app.state::<AppState>();
    let view = state.tracker.lock().unwrap().gems_view();
    GemsPayload {
        view,
        visible: state.gems_visible.load(Ordering::SeqCst),
    }
}

/// Sends the gem links only when they changed.
fn emit_gems(app: &AppHandle) {
    static LAST: Mutex<String> = Mutex::new(String::new());
    let payload = gems_payload(app);
    let Ok(json) = serde_json::to_string(&payload) else {
        return;
    };
    let mut last = LAST.lock().unwrap();
    if *last != json {
        let _ = app.emit(GEMS_VIEW, payload);
        *last = json;
    }
}

/// Event carrying the Build notes (plus whether they're showing).
const NOTES_VIEW: &str = "notes-view";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotesPayload {
    character: Option<String>,
    lines: Vec<Vec<poe_core::pob::NoteSpan>>,
    visible: bool,
}

fn notes_payload(app: &AppHandle) -> NotesPayload {
    let state = app.state::<AppState>();
    let (character, lines) = state.tracker.lock().unwrap().notes();
    NotesPayload {
        character,
        lines,
        visible: state.notes_visible.load(Ordering::SeqCst),
    }
}

/// Sends the notes only when they changed.
fn emit_notes(app: &AppHandle) {
    static LAST: Mutex<String> = Mutex::new(String::new());
    let payload = notes_payload(app);
    let Ok(json) = serde_json::to_string(&payload) else {
        return;
    };
    let mut last = LAST.lock().unwrap();
    if *last != json {
        let _ = app.emit(NOTES_VIEW, payload);
        *last = json;
    }
}

#[tauri::command]
fn get_notes_view(app: AppHandle) -> NotesPayload {
    notes_payload(&app)
}

#[tauri::command]
fn toggle_notes(app: AppHandle, state: tauri::State<AppState>) {
    state.notes_visible.fetch_xor(true, Ordering::SeqCst);
    emit_notes(&app);
}

#[tauri::command]
fn get_gems_view(app: AppHandle) -> GemsPayload {
    gems_payload(&app)
}

#[tauri::command]
fn toggle_gems(app: AppHandle, state: tauri::State<AppState>) {
    state.gems_visible.fetch_xor(true, Ordering::SeqCst);
    emit_gems(&app);
}

#[tauri::command]
fn set_group_levels(
    app: AppHandle,
    state: tauri::State<AppState>,
    name: String,
    index: usize,
    levels: Option<poe_core::pob::LevelRange>,
    reset: bool,
) {
    state
        .tracker
        .lock()
        .unwrap()
        .set_group_levels(&name, index, levels, reset);
    emit_gems(&app);
}

#[tauri::command]
fn copy_regex(app: AppHandle) {
    copy_vendor_regex(&app);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportSummary {
    class: String,
    ascendancy: Option<String>,
    skill_groups: usize,
    tree_stages: usize,
}

/// Import a Path of Building code or pobb.in / pastebin link for a Character.
#[tauri::command]
async fn import_build(
    app: AppHandle,
    name: String,
    input: String,
) -> Result<ImportSummary, String> {
    let code = match poe_core::pob::source(&input) {
        poe_core::pob::Source::Code(code) => code,
        poe_core::pob::Source::Fetch(url) => tauri::async_runtime::spawn_blocking(move || {
            ureq::get(&url)
                .call()
                .map_err(|e| format!("could not fetch the link: {e}"))?
                .body_mut()
                .read_to_string()
                .map_err(|e| format!("could not read the link: {e}"))
        })
        .await
        .map_err(|e| e.to_string())??,
    };
    let build = poe_core::pob::import(&code).map_err(|e| e.to_string())?;
    let summary = ImportSummary {
        class: build.class.clone(),
        ascendancy: build.ascendancy.clone(),
        skill_groups: build.skill_groups.len(),
        tree_stages: build.tree_stages.len(),
    };
    let stored = app.state::<AppState>().tracker.lock().unwrap().set_build(
        &name,
        build,
        input.trim().to_owned(),
    );
    if !stored {
        return Err(format!("no character named {name}"));
    }
    emit_guide(&app);
    emit_gems(&app);
    Ok(summary)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuidePayload {
    #[serde(flatten)]
    view: GuideView,
    act_view: bool,
}

fn guide_payload(app: &AppHandle) -> GuidePayload {
    let state = app.state::<AppState>();
    let browse = *state.act_browse.lock().unwrap();
    let view = state.tracker.lock().unwrap().guide_view(browse);
    GuidePayload {
        view,
        act_view: state.act_view.load(Ordering::SeqCst),
    }
}

/// Sends the Guide only when it changed, so the act view isn't re-rendered
/// (and scrolled) every second.
fn emit_guide(app: &AppHandle) {
    static LAST: Mutex<String> = Mutex::new(String::new());
    let payload = guide_payload(app);
    let Ok(json) = serde_json::to_string(&payload) else {
        return;
    };
    let mut last = LAST.lock().unwrap();
    if *last != json {
        let _ = app.emit(GUIDE_VIEW, payload);
        *last = json;
    }
}

#[tauri::command]
fn get_guide_view(app: AppHandle) -> GuidePayload {
    guide_payload(&app)
}

#[tauri::command]
fn guide_command(app: AppHandle, command: GuideCommand) {
    run_guide_command(&app, command);
}

/// Event: a short note from the Guide (e.g. why find me didn't move).
const GUIDE_MESSAGE: &str = "guide-message";

fn run_guide_command(app: &AppHandle, command: GuideCommand) {
    let state = app.state::<AppState>();
    let message = state.tracker.lock().unwrap().guide_command(command);
    if matches!(command, GuideCommand::Jump { .. }) {
        *state.act_browse.lock().unwrap() = None;
    }
    if let Some(m) = message {
        let _ = app.emit(GUIDE_MESSAGE, m);
    }
    emit_guide(app);
}

/// Show another act in the full-act view (`None` = the current act).
#[tauri::command]
fn browse_act(app: AppHandle, state: tauri::State<AppState>, act: Option<u8>) {
    *state.act_browse.lock().unwrap() = act;
    emit_guide(&app);
}

#[tauri::command]
fn set_skill_set(app: AppHandle, state: tauri::State<AppState>, name: String, set: Option<String>) {
    state.tracker.lock().unwrap().set_skill_set(&name, set);
    emit_gems(&app);
}

#[tauri::command]
fn set_tree_stage(
    app: AppHandle,
    state: tauri::State<AppState>,
    name: String,
    stage: Option<usize>,
) {
    state.tracker.lock().unwrap().set_tree_stage(&name, stage);
    emit_tree(&app);
}

/// "Fix points" mode on the tree Overlay (the tree bar's button).
#[tauri::command]
fn toggle_tree_edit(app: AppHandle, state: tauri::State<AppState>) {
    state.tracker.lock().unwrap().toggle_tree_editing();
    emit_tree(&app);
}

#[tauri::command]
fn cycle_skill_set(app: AppHandle, state: tauri::State<AppState>, delta: i32) {
    state.tracker.lock().unwrap().cycle_skill_set(delta);
    emit_gems(&app);
}

#[tauri::command]
fn cycle_tree_stage(app: AppHandle, state: tauri::State<AppState>, delta: i32) {
    state.tracker.lock().unwrap().cycle_tree_stage(delta);
    emit_tree(&app);
}

/// Hide or show the tree Overlay (the tree bar's button).
#[tauri::command]
fn hotkey_failures(state: tauri::State<AppState>) -> Vec<String> {
    state.hotkey_failures.lock().unwrap().clone()
}

#[tauri::command]
fn toggle_act_view(app: AppHandle, state: tauri::State<AppState>) {
    state.act_view.fetch_xor(true, Ordering::SeqCst);
    *state.act_browse.lock().unwrap() = None;
    emit_guide(&app);
}

#[tauri::command]
fn set_guide_options(
    app: AppHandle,
    state: tauri::State<AppState>,
    name: String,
    options: GuideOptions,
) {
    state
        .tracker
        .lock()
        .unwrap()
        .set_guide_options(&name, options);
    emit_guide(&app);
}

fn emit_tree(app: &AppHandle) {
    let view = app.state::<AppState>().tracker.lock().unwrap().tree_view();
    let _ = app.emit(TREE_VIEW, view);
}

/// Main-tree nodes for drawing (the ascendancy panel is separate in-game).
#[tauri::command]
fn get_tree(state: tauri::State<AppState>) -> Vec<TreeNode> {
    let tracker = state.tracker.lock().unwrap();
    tracker.tree().nodes.clone()
}

#[tauri::command]
fn get_tree_view(state: tauri::State<AppState>) -> TreeView {
    state.tracker.lock().unwrap().tree_view()
}

#[tauri::command]
fn tree_click(app: AppHandle, state: tauri::State<AppState>, x: f64, y: f64) {
    let height = platform::find_game_window()
        .and_then(platform::game_rect)
        .map_or(0.0, |r| f64::from(r.height));
    state.tracker.lock().unwrap().tree_click(x, y, height);
    overlay::raise_small(&app);
    emit_tree(&app);
}

#[tauri::command]
fn toggle_tree(app: AppHandle, state: tauri::State<AppState>, calibrate: bool) {
    let mut tracker = state.tracker.lock().unwrap();
    if calibrate {
        tracker.toggle_calibration();
    } else {
        tracker.toggle_tree();
        let hidden = !tracker.tree_overlay().visible;
        state.tree_pill.store(hidden, Ordering::SeqCst);
    }
    drop(tracker);
    emit_tree(&app);
}

/// Rebuild a Character's passives from the whole Client Log. Returns how many.
#[tauri::command]
async fn rebuild_passives(app: AppHandle, name: String) -> Result<usize, String> {
    let path = app
        .state::<AppState>()
        .tracker
        .lock()
        .unwrap()
        .log_path()
        .ok_or("no Client Log found yet")?;
    let who = name.clone();
    let passives = tauri::async_runtime::spawn_blocking(move || {
        let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        let records = poe_core::log::replay(std::io::BufReader::new(file));
        Ok::<_, String>(poe_core::tree::allocations_from_log(records, &who))
    })
    .await
    .map_err(|e| e.to_string())??;
    let count = passives.passives.len();
    app.state::<AppState>()
        .tracker
        .lock()
        .unwrap()
        .set_allocations(&name, passives);
    emit_tree(&app);
    Ok(count)
}

fn emit_view(app: &AppHandle) {
    let view = app.state::<AppState>().tracker.lock().unwrap().view();
    let _ = app.emit(TIMER_VIEW, view);
}

#[tauri::command]
fn get_timer_view(state: tauri::State<AppState>) -> TimerView {
    state.tracker.lock().unwrap().view()
}

#[tauri::command]
fn timer_command(app: AppHandle, state: tauri::State<AppState>, command: Command) {
    state.tracker.lock().unwrap().command(command);
    emit_view(&app);
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(app: AppHandle, state: tauri::State<AppState>, settings: Settings) {
    state.tracker.lock().unwrap().apply_settings(&settings);
    Storage::new(data_dir()).save_settings(&settings);
    let _ = app.emit("settings", &settings);
    *state.settings.lock().unwrap() = settings;
    emit_view(&app);
}

/// Change one Overlay's size, opacity (`clear_opacity` returns it to the
/// shared setting) or visibility.
#[tauri::command]
fn set_overlay_look(
    app: AppHandle,
    state: tauri::State<AppState>,
    label: String,
    scale: Option<f64>,
    opacity: Option<f64>,
    clear_opacity: Option<bool>,
    shown: Option<bool>,
) -> Settings {
    let mut settings = state.settings.lock().unwrap();
    let mut look = settings.look(&label);
    if let Some(s) = scale {
        look.scale = s.clamp(0.7, 1.5);
        overlay::apply_scale(&app, &label, look.scale);
    }
    if let Some(o) = opacity {
        look.opacity = Some(o.clamp(0.2, 1.0));
    }
    if clear_opacity == Some(true) {
        look.opacity = None;
    }
    if let Some(on) = shown {
        let flag = match label.as_str() {
            overlay::TIMER => Some(&state.timer_visible),
            overlay::GUIDE => Some(&state.guide_visible),
            overlay::GEMS => Some(&state.gems_visible),
            overlay::XP => Some(&state.xp_visible),
            overlay::NOTES => Some(&state.notes_visible),
            _ => None,
        };
        if let Some(flag) = flag {
            flag.store(on, Ordering::SeqCst);
            look.shown = Some(on);
        }
    }
    settings.overlays.insert(label, look);
    Storage::new(data_dir()).save_settings(&settings);
    let _ = app.emit("settings", &*settings);
    let out = settings.clone();
    drop(settings);
    emit_gems(&app);
    emit_notes(&app);
    out
}

/// Put every Overlay back where it starts out.
#[tauri::command]
fn reset_overlay_positions(app: AppHandle, state: tauri::State<AppState>) {
    let mut settings = state.settings.lock().unwrap();
    settings.reset_placements();
    Storage::new(data_dir()).save_settings(&settings);
    let _ = app.emit("settings", &*settings);
}

/// Which toggleable Overlays are showing right now.
#[tauri::command]
fn get_overlays_shown(
    state: tauri::State<AppState>,
) -> std::collections::BTreeMap<&'static str, bool> {
    [
        (overlay::TIMER, &state.timer_visible),
        (overlay::GUIDE, &state.guide_visible),
        (overlay::GEMS, &state.gems_visible),
        (overlay::XP, &state.xp_visible),
        (overlay::NOTES, &state.notes_visible),
    ]
    .into_iter()
    .map(|(l, f)| (l, f.load(Ordering::SeqCst)))
    .collect()
}

#[tauri::command]
fn get_hotkeys(state: tauri::State<AppState>) -> Vec<hotkeys::HotkeyRow> {
    hotkeys::rows(&state.settings.lock().unwrap().hotkeys)
}

/// Bind an action to a shortcut (`None` unbinds; `id` `None` resets all).
#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: Option<String>,
    shortcut: Option<String>,
) -> Result<Vec<hotkeys::HotkeyRow>, String> {
    let mut settings = state.settings.lock().unwrap();
    match id {
        Some(id) => hotkeys::rebind(&mut settings.hotkeys, &id, shortcut.as_deref())?,
        None => settings.hotkeys.clear(),
    }
    Storage::new(data_dir()).save_settings(&settings);
    *state.bindings.lock().unwrap() = hotkeys::bindings(&settings.hotkeys);
    state.hotkeys_changed.store(true, Ordering::SeqCst);
    let rows = hotkeys::rows(&settings.hotkeys);
    let _ = app.emit("hotkeys", &rows);
    Ok(rows)
}

#[tauri::command]
fn get_history(state: tauri::State<AppState>) -> HistoryView {
    HistoryView::build(state.tracker.lock().unwrap().history())
}

#[tauri::command]
fn delete_run(app: AppHandle, state: tauri::State<AppState>, started_at: i64) -> HistoryView {
    let mut tracker = state.tracker.lock().unwrap();
    tracker.delete_run(started_at);
    let view = HistoryView::build(tracker.history());
    drop(tracker);
    // Personal Best and best segments may have changed.
    emit_view(&app);
    view
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Characters {
    active: Option<String>,
    profiles: Vec<poe_core::profile::Profile>,
}

#[tauri::command]
fn get_characters(state: tauri::State<AppState>) -> Characters {
    let tracker = state.tracker.lock().unwrap();
    Characters {
        active: tracker.active_character(),
        profiles: tracker.profiles(),
    }
}

#[tauri::command]
fn select_character(app: AppHandle, state: tauri::State<AppState>, name: String) {
    state.tracker.lock().unwrap().select_character(&name);
    emit_view(&app);
}

#[tauri::command]
fn get_interact(state: tauri::State<AppState>) -> bool {
    state.interact.load(Ordering::SeqCst)
}

/// A key press anywhere. While the game is focused, the passive tree key
/// toggles the tree Overlay along with the game's tree, and Escape hides it.
fn on_key(app: &AppHandle, vk: u32) {
    let Some(game) = platform::find_game_window() else {
        return;
    };
    if platform::foreground_window() != game {
        return;
    }
    let state = app.state::<AppState>();
    let Some(tree_vk) = state
        .settings
        .lock()
        .unwrap()
        .tree_key
        .as_deref()
        .and_then(platform::vk_for)
    else {
        return;
    };
    let mut tracker = state.tracker.lock().unwrap();
    let visible = tracker.tree_overlay().visible;
    if vk == tree_vk || (vk == platform::VK_ESCAPE && visible) {
        tracker.toggle_tree();
        drop(tracker);
        // The game's tree opened or closed: no leftover "Show tree" button.
        state.tree_pill.store(false, Ordering::SeqCst);
        emit_tree(app);
    }
}

/// A middle-click anywhere. Aligns the tree Overlay when it is showing, the
/// game is focused and the click landed inside the game window.
fn on_middle_click(app: &AppHandle, x: i32, y: i32) {
    let Some(game) = platform::find_game_window() else {
        return;
    };
    if platform::foreground_window() != game {
        return;
    }
    let Some(r) = platform::game_rect(game) else {
        return;
    };
    let (gx, gy) = (x - r.x, y - r.y);
    if gx < 0 || gy < 0 || gx >= r.width || gy >= r.height {
        return;
    }
    let aligned = app.state::<AppState>().tracker.lock().unwrap().align_tree(
        f64::from(gx),
        f64::from(gy),
        f64::from(r.height),
    );
    if aligned {
        emit_tree(app);
    }
}

/// Background loop: find the game, follow its log, keep Overlays in place.
fn spawn_tracking_loop(app: AppHandle) {
    std::thread::spawn(move || {
        let shortcuts = |app: &AppHandle| -> Vec<Shortcut> {
            let state = app.state::<AppState>();
            let bindings = state.bindings.lock().unwrap();
            bindings.iter().map(|(s, _)| *s).collect()
        };
        let mut overlays = overlay::Follower::new(shortcuts(&app));
        let mut last_emit = Instant::now();
        let mut last_save = Instant::now();
        let mut last_status = None;
        let mut was_in_lab = false;
        loop {
            let state = app.state::<AppState>();
            let game = platform::find_game_window();
            let manual_path = state.settings.lock().unwrap().log_path.clone();

            let read_any = {
                let mut tracker = state.tracker.lock().unwrap();
                if let Some(path) = manual_path.or_else(|| game.and_then(platform::client_log_path))
                {
                    tracker.follow(path);
                }
                if last_save.elapsed() > Duration::from_secs(15) {
                    tracker.save_if_stale();
                    last_save = Instant::now();
                }
                tracker.poll()
            };
            if read_any || last_emit.elapsed() > Duration::from_secs(1) {
                emit_view(&app);
                emit_tree(&app);
                emit_guide(&app);
                emit_gems(&app);
                emit_notes(&app);
                emit_lab(&app);
                last_emit = Instant::now();
            }

            let status = Status {
                log_path: state
                    .tracker
                    .lock()
                    .unwrap()
                    .log_path()
                    .map(|p| p.display().to_string()),
                game_running: game.is_some(),
            };
            if last_status.as_ref().is_none_or(|s: &Status| {
                s.log_path != status.log_path || s.game_running != status.game_running
            }) {
                let _ = app.emit(STATUS, status.clone());
                last_status = Some(status);
            }

            lab_import::tick(
                &app,
                game.is_some_and(|g| platform::foreground_window() == g),
            );
            // A leftover "Show tree" button only lasts until the next area.
            if state.tracker.lock().unwrap().take_area_changed()
                && state.tree_pill.swap(false, Ordering::SeqCst)
            {
                emit_tree(&app);
            }
            // Final Izaro room: offer "Show ascendancy" for the plaque.
            if state.tracker.lock().unwrap().take_lab_end() {
                state.tree_pill.store(true, Ordering::SeqCst);
                emit_tree(&app);
            }
            let in_lab = state.tracker.lock().unwrap().lab_view().in_lab;
            if in_lab != was_in_lab {
                // Entering or leaving the lab returns the Overlay to automatic.
                *state.lab_manual.lock().unwrap() = None;
                was_in_lab = in_lab;
                emit_lab(&app);
            }
            let lab_shown = lab_visible(&app);

            let wanted = {
                let tracker = state.tracker.lock().unwrap();
                let tree = tracker.tree_overlay();
                let settings = state.settings.lock().unwrap();
                [
                    overlay::Wanted {
                        label: overlay::TIMER,
                        anchor: overlay::Anchor::At(settings.timer_overlay),
                        visible: state.timer_visible.load(Ordering::SeqCst),
                        clickable: false,
                    },
                    overlay::Wanted {
                        label: overlay::TREE,
                        anchor: overlay::Anchor::Cover,
                        visible: tree.visible,
                        clickable: tree.calibrating.is_some() || tree.editing,
                    },
                    // The Guide has buttons, so it takes clicks (never focus).
                    overlay::Wanted {
                        label: overlay::GUIDE,
                        anchor: overlay::Anchor::At(settings.guide_overlay),
                        visible: state.guide_visible.load(Ordering::SeqCst),
                        clickable: true,
                    },
                    overlay::Wanted {
                        label: overlay::XP,
                        anchor: overlay::Anchor::At(settings.xp_overlay),
                        visible: state.xp_visible.load(Ordering::SeqCst),
                        clickable: false,
                    },
                    overlay::Wanted {
                        label: overlay::TREEBAR,
                        anchor: overlay::Anchor::RightAt(settings.treebar_right),
                        // Collapses to a "Show tree" button when the tree
                        // Overlay is hidden, once it has been set up.
                        visible: tree.calibrating.is_none()
                            && (tree.visible || state.tree_pill.load(Ordering::SeqCst)),
                        clickable: true,
                    },
                    overlay::Wanted {
                        label: overlay::LAB,
                        anchor: overlay::Anchor::At(settings.lab_overlay),
                        visible: lab_shown,
                        clickable: true,
                    },
                    // Takes clicks for its skill set buttons (never focus).
                    overlay::Wanted {
                        label: overlay::GEMS,
                        anchor: overlay::Anchor::At(settings.gems_overlay),
                        visible: state.gems_visible.load(Ordering::SeqCst),
                        clickable: true,
                    },
                    // Takes clicks so the mouse wheel scrolls it (never focus).
                    overlay::Wanted {
                        label: overlay::NOTES,
                        anchor: overlay::Anchor::At(settings.notes_overlay),
                        visible: state.notes_visible.load(Ordering::SeqCst),
                        clickable: true,
                    },
                    overlay::Wanted {
                        label: overlay::ACT,
                        anchor: overlay::Anchor::At(settings.act_overlay),
                        visible: state.act_view.load(Ordering::SeqCst),
                        clickable: true,
                    },
                ]
            };
            if state.hotkeys_changed.swap(false, Ordering::SeqCst) {
                overlays.set_shortcuts(&app, shortcuts(&app));
            }
            let interact = state.interact.load(Ordering::SeqCst);
            let moved = overlays.sync(&app, game, interact, &wanted);
            {
                let mut failures = state.hotkey_failures.lock().unwrap();
                if *failures != overlays.failed {
                    failures.clone_from(&overlays.failed);
                    let _ = app.emit(HOTKEY_FAILURES, failures.clone());
                }
            }
            if !moved.is_empty() {
                let mut settings = state.settings.lock().unwrap();
                for (label, placement) in moved {
                    match label {
                        overlay::TIMER => settings.timer_overlay = placement,
                        overlay::GUIDE => settings.guide_overlay = placement,
                        overlay::ACT => settings.act_overlay = placement,
                        overlay::GEMS => settings.gems_overlay = placement,
                        overlay::LAB => settings.lab_overlay = placement,
                        overlay::TREEBAR => settings.treebar_right = placement,
                        overlay::XP => settings.xp_overlay = placement,
                        overlay::NOTES => settings.notes_overlay = placement,
                        _ => {}
                    }
                }
                Storage::new(data_dir()).save_settings(&settings);
            }

            std::thread::sleep(Duration::from_millis(250));
        }
    });
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> Status {
    Status {
        log_path: state
            .tracker
            .lock()
            .unwrap()
            .log_path()
            .map(|p| p.display().to_string()),
        game_running: platform::find_game_window().is_some(),
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show poe-overlay", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("bundled icon"))
        .tooltip("poe-overlay")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => {
                app.state::<AppState>()
                    .tracker
                    .lock()
                    .unwrap()
                    .save_session();
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let storage = Storage::new(data_dir());
    let settings = storage.load_settings();
    let tracker = Tracker::new(storage, &settings);
    let bindings = hotkeys::bindings(&settings.hotkeys);
    let shown = |label: &str, default: bool| settings.look(label).shown.unwrap_or(default);
    let notes_shown = shown(overlay::NOTES, false);
    let (gems_shown, xp_shown, timer_shown, guide_shown) = (
        shown(overlay::GEMS, false),
        shown(overlay::XP, true),
        shown(overlay::TIMER, true),
        shown(overlay::GUIDE, true),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let hotkey = app
                        .state::<AppState>()
                        .bindings
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|(s, _)| s == shortcut)
                        .map(|(_, h)| *h);
                    if let Some(hotkey) = hotkey {
                        on_hotkey(app, hotkey);
                    }
                })
                .build(),
        )
        .manage(AppState {
            tracker: Mutex::new(tracker),
            settings: Mutex::new(settings),
            interact: AtomicBool::new(false),
            act_view: AtomicBool::new(false),
            gems_visible: AtomicBool::new(gems_shown),
            regex_cursor: Mutex::new((Vec::new(), 0)),
            lab_import: Mutex::new(None),
            lab_manual: Mutex::new(None),
            act_browse: Mutex::new(None),
            hotkey_failures: Mutex::new(Vec::new()),
            tree_pill: AtomicBool::new(false),
            xp_visible: AtomicBool::new(xp_shown),
            timer_visible: AtomicBool::new(timer_shown),
            guide_visible: AtomicBool::new(guide_shown),
            notes_visible: AtomicBool::new(notes_shown),
            bindings: Mutex::new(bindings),
            hotkeys_changed: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_timer_view,
            timer_command,
            get_settings,
            set_settings,
            get_history,
            get_interact,
            get_status,
            get_characters,
            select_character,
            get_tree,
            get_tree_view,
            tree_click,
            toggle_tree,
            get_guide_view,
            guide_command,
            toggle_act_view,
            set_guide_options,
            get_gems_view,
            toggle_gems,
            copy_regex,
            import_build,
            set_group_levels,
            get_lab_view,
            lab_open,
            lab_load_clipboard,
            lab_toggle,
            set_lab_detour,
            browse_act,
            set_skill_set,
            set_tree_stage,
            cycle_tree_stage,
            cycle_skill_set,
            toggle_tree_edit,
            hotkey_failures,
            rebuild_passives,
            get_hotkeys,
            set_hotkey,
            set_overlay_look,
            reset_overlay_positions,
            get_overlays_shown,
            get_notes_view,
            toggle_notes,
            delete_run,
        ])
        .on_window_event(|window, event| {
            // Closing the main window sends it to the tray.
            if window.label() == "main"
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            setup_tray(app)?;
            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            overlay::create_overlays(app.handle(), |label| settings.look(label).scale)?;
            spawn_tracking_loop(app.handle().clone());
            let handle = app.handle().clone();
            platform::watch_middle_clicks(move |x, y| on_middle_click(&handle, x, y));
            let handle = app.handle().clone();
            platform::watch_keys(move |vk| on_key(&handle, vk));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
