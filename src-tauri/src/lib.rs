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

use poe_core::{run::Command, run::Run, view::TimerView};
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    log_path: Option<String>,
    game_running: bool,
}

fn hotkeys() -> [(Shortcut, Hotkey); 4] {
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
    ]
}

#[derive(Debug, Clone, Copy)]
enum Hotkey {
    Timer(Command),
    InteractMode,
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
            overlay::set_interact(app, on);
            let _ = app.emit(INTERACT, on);
        }
    }
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

            let placement = state.settings.lock().unwrap().timer_overlay;
            let interact = state.interact.load(Ordering::SeqCst);
            if let Some(moved) = overlays.sync(&app, game, placement, interact) {
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
            overlay::create_timer_overlay(app.handle(), opacity)?;
            spawn_tracking_loop(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
