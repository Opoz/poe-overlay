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
    run::{Command, Run},
    tree::{TreeNode, TreeView},
    view::TimerView,
};
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

use crate::{
    storage::{Settings, Storage, data_dir},
    tracker::Tracker,
};

pub struct AppState {
    tracker: Mutex<Tracker>,
    settings: Mutex<Settings>,
    /// Interact Mode: Overlays accept the mouse.
    interact: AtomicBool,
}

/// Event carrying a fresh [`TimerView`] to every window.
const TIMER_VIEW: &str = "timer-view";
/// Event carrying the [`Status`] to the main window.
const STATUS: &str = "status";
const INTERACT: &str = "interact-mode";
/// Event carrying a fresh [`TreeView`] to every window.
const TREE_VIEW: &str = "tree-view";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    log_path: Option<String>,
    game_running: bool,
}

fn hotkeys() -> [(Shortcut, Hotkey); 6] {
    let alt = Some(Modifiers::ALT);
    let alt_shift = Some(Modifiers::ALT | Modifiers::SHIFT);
    [
        (
            Shortcut::new(alt, Code::KeyP),
            Hotkey::Timer(Command::TogglePause),
        ),
        (
            Shortcut::new(alt, Code::KeyS),
            Hotkey::Timer(Command::Split),
        ),
        (
            Shortcut::new(alt_shift, Code::KeyS),
            Hotkey::Timer(Command::UndoSplit),
        ),
        (Shortcut::new(alt, Code::KeyE), Hotkey::InteractMode),
        (Shortcut::new(alt, Code::KeyT), Hotkey::TreeOverlay),
        (Shortcut::new(alt_shift, Code::KeyT), Hotkey::Calibrate),
    ]
}

#[derive(Debug, Clone, Copy)]
enum Hotkey {
    Timer(Command),
    InteractMode,
    TreeOverlay,
    Calibrate,
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
            state.tracker.lock().unwrap().toggle_tree();
            emit_tree(app);
        }
        Hotkey::Calibrate => {
            state.tracker.lock().unwrap().toggle_calibration();
            emit_tree(app);
        }
    }
}

fn emit_tree(app: &AppHandle) {
    let view = app.state::<AppState>().tracker.lock().unwrap().tree_view();
    let _ = app.emit(TREE_VIEW, view);
}

/// Main-tree nodes for drawing (the ascendancy panel is separate in-game).
#[tauri::command]
fn get_tree(state: tauri::State<AppState>) -> Vec<TreeNode> {
    let tracker = state.tracker.lock().unwrap();
    tracker
        .tree()
        .nodes
        .iter()
        .filter(|n| n.ascendancy.is_none())
        .cloned()
        .collect()
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
    emit_tree(&app);
}

#[tauri::command]
fn toggle_tree(app: AppHandle, state: tauri::State<AppState>, calibrate: bool) {
    let mut tracker = state.tracker.lock().unwrap();
    if calibrate {
        tracker.toggle_calibration();
    } else {
        tracker.toggle_tree();
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
        Ok::<_, String>(poe_core::tree::passives_from_log(records, &who))
    })
    .await
    .map_err(|e| e.to_string())??;
    let count = passives.len();
    app.state::<AppState>()
        .tracker
        .lock()
        .unwrap()
        .set_passives(&name, passives);
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
    *state.settings.lock().unwrap() = settings;
    emit_view(&app);
}

#[tauri::command]
fn get_history(state: tauri::State<AppState>) -> Vec<Run> {
    state.tracker.lock().unwrap().history().to_vec()
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

/// Background loop: find the game, follow its log, keep Overlays in place.
fn spawn_tracking_loop(app: AppHandle) {
    std::thread::spawn(move || {
        let mut overlays = overlay::Follower::new(hotkeys().map(|(s, _)| s).to_vec());
        let mut last_emit = Instant::now();
        let mut last_save = Instant::now();
        let mut last_status = None;
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

            let wanted = {
                let tracker = state.tracker.lock().unwrap();
                let tree = tracker.tree_overlay();
                overlay::Wanted {
                    timer_placement: state.settings.lock().unwrap().timer_overlay,
                    interact: state.interact.load(Ordering::SeqCst),
                    tree_visible: tree.visible,
                    tree_clickable: tree.calibrating.is_some(),
                }
            };
            if let Some(moved) = overlays.sync(&app, game, wanted) {
                let mut settings = state.settings.lock().unwrap();
                settings.timer_overlay = moved;
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

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if let Some((_, hotkey)) = hotkeys().into_iter().find(|(s, _)| s == shortcut) {
                        on_hotkey(app, hotkey);
                    }
                })
                .build(),
        )
        .manage(AppState {
            tracker: Mutex::new(tracker),
            settings: Mutex::new(settings),
            interact: AtomicBool::new(false),
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
            rebuild_passives,
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
            let opacity = app
                .state::<AppState>()
                .settings
                .lock()
                .unwrap()
                .overlay_opacity;
            overlay::create_overlays(app.handle(), opacity)?;
            spawn_tracking_loop(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
