//! Windows-specific helpers: finding the game window and its process.
//! Read-only queries; nothing here touches the game (ADR 0002).

use std::path::PathBuf;

use windows::{
    Win32::{
        Foundation::{CloseHandle, POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            FindWindowW, GetClientRect, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        },
    },
    core::{PWSTR, w},
};

/// A raw window handle as an integer, so it can cross threads.
pub type RawHwnd = isize;

/// The game's client area in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

fn hwnd(raw: RawHwnd) -> windows::Win32::Foundation::HWND {
    windows::Win32::Foundation::HWND(raw as *mut _)
}

/// The Path of Exile 1 game window, if running.
pub fn find_game_window() -> Option<RawHwnd> {
    let h = unsafe { FindWindowW(w!("POEWindowClass"), None) }.ok()?;
    (!h.is_invalid()).then_some(h.0 as RawHwnd)
}

pub fn foreground_window() -> RawHwnd {
    unsafe { GetForegroundWindow() }.0 as RawHwnd
}

/// The game's client rect, or `None` if minimised.
pub fn game_rect(game: RawHwnd) -> Option<GameRect> {
    let h = hwnd(game);
    if unsafe { IsIconic(h) }.as_bool() {
        return None;
    }
    let mut rect = RECT::default();
    unsafe { GetClientRect(h, &mut rect) }.ok()?;
    let mut origin = POINT::default();
    if !unsafe { ClientToScreen(h, &mut origin) }.as_bool() {
        return None;
    }
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    (width > 0 && height > 0).then_some(GameRect {
        x: origin.x,
        y: origin.y,
        width,
        height,
    })
}

/// `<install dir>/logs/Client.txt` for the process owning the game window.
pub fn client_log_path(game: RawHwnd) -> Option<PathBuf> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd(game), Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }
    result.ok()?;
    let exe = PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
    let dir = exe.parent()?.join("logs");
    ["Client.txt", "KakaoClient.txt"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|p| p.exists())
}

/// Turn a window into an Overlay: layered (so it can be click-through and
/// faded), never takes focus when clicked, and stays out of Alt+Tab.
///
/// Done directly rather than through Tauri: Tauri re-applies its own idea of
/// the window's state (including "hidden", since we show it without
/// activating) whenever it changes these styles itself.
pub fn make_overlay(window: RawHwnd) {
    use windows::Win32::UI::WindowsAndMessaging::{
        WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    };
    update_ex_style(
        window,
        WS_EX_LAYERED.0 | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TRANSPARENT.0,
        0,
    );
    // Fully opaque: panels fade their own backgrounds, so text stays crisp.
    set_opacity(window, 1.0);
}

/// Click-through on: the mouse goes to the game underneath.
pub fn set_click_through(window: RawHwnd, on: bool) {
    use windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT;
    if on {
        update_ex_style(window, WS_EX_TRANSPARENT.0, 0);
    } else {
        update_ex_style(window, 0, WS_EX_TRANSPARENT.0);
    }
}

pub fn set_opacity(window: RawHwnd, opacity: f64) {
    use windows::Win32::{
        Foundation::COLORREF,
        UI::WindowsAndMessaging::{LWA_ALPHA, SetLayeredWindowAttributes},
    };
    let alpha = (opacity.clamp(0.1, 1.0) * 255.0).round() as u8;
    unsafe {
        let _ = SetLayeredWindowAttributes(hwnd(window), COLORREF(0), alpha, LWA_ALPHA);
    }
}

fn update_ex_style(window: RawHwnd, add: u32, remove: u32) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW,
    };
    let h = hwnd(window);
    unsafe {
        let style = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(h, GWL_EXSTYLE, ((style | add) & !remove) as isize);
    }
}

/// Put a topmost window above the other topmost windows, without activating it.
pub fn raise(window: RawHwnd) {
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };
    unsafe {
        let _ = SetWindowPos(
            hwnd(window),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Show or hide without activating (never steals focus from the game).
pub fn set_visible(window: RawHwnd, visible: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow};
    let cmd = if visible { SW_SHOWNOACTIVATE } else { SW_HIDE };
    unsafe {
        let _ = ShowWindow(hwnd(window), cmd);
    }
}

/// Watch for middle-clicks anywhere on screen and report their screen
/// position (physical pixels). Read-only: every click still reaches whatever
/// is under the cursor, the game included. Runs a low-level mouse hook on its
/// own thread, which needs a message loop.
pub fn watch_middle_clicks(on_click: impl Fn(i32, i32) + Send + 'static) {
    use std::sync::OnceLock;

    use windows::Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, HC_ACTION, MSG, MSLLHOOKSTRUCT, SetWindowsHookExW,
            WH_MOUSE_LL, WM_MBUTTONDOWN,
        },
    };

    type Callback = Box<dyn Fn(i32, i32) + Send + Sync>;
    static CALLBACK: OnceLock<Callback> = OnceLock::new();

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && wparam.0 == WM_MBUTTONDOWN as usize {
            // SAFETY: for WH_MOUSE_LL, lparam points to an MSLLHOOKSTRUCT.
            let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
            if let Some(cb) = CALLBACK.get() {
                cb(info.pt.x, info.pt.y);
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    // The callback only queues work; it must return quickly or Windows drops the hook.
    let (tx, rx) = std::sync::mpsc::channel::<(i32, i32)>();
    let tx = std::sync::Mutex::new(tx);
    if CALLBACK
        .set(Box::new(move |x, y| {
            let _ = tx.lock().map(|t| t.send((x, y)));
        }))
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || {
        for (x, y) in rx {
            on_click(x, y);
        }
    });
    std::thread::spawn(|| unsafe {
        let module = GetModuleHandleW(None).ok();
        let Ok(_hook) = SetWindowsHookExW(WH_MOUSE_LL, Some(hook), module.map(Into::into), 0)
        else {
            return;
        };
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    });
}

/// Watch key presses anywhere (virtual-key codes). Read-only, like
/// [`watch_middle_clicks`]: every key still reaches the game.
pub fn watch_keys(on_key: impl Fn(u32) + Send + 'static) {
    use std::sync::OnceLock;

    use windows::Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, SetWindowsHookExW,
            WH_KEYBOARD_LL, WM_KEYDOWN,
        },
    };

    type Callback = Box<dyn Fn(u32) + Send + Sync>;
    static CALLBACK: OnceLock<Callback> = OnceLock::new();

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && wparam.0 == WM_KEYDOWN as usize {
            // SAFETY: for WH_KEYBOARD_LL, lparam points to a KBDLLHOOKSTRUCT.
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            if let Some(cb) = CALLBACK.get() {
                cb(info.vkCode);
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    let (tx, rx) = std::sync::mpsc::channel::<u32>();
    let tx = std::sync::Mutex::new(tx);
    if CALLBACK
        .set(Box::new(move |vk| {
            let _ = tx.lock().map(|t| t.send(vk));
        }))
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || {
        for vk in rx {
            on_key(vk);
        }
    });
    std::thread::spawn(|| unsafe {
        let module = GetModuleHandleW(None).ok();
        let Ok(_hook) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), module.map(Into::into), 0)
        else {
            return;
        };
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    });
}

/// Virtual-key code for a single letter or digit ("P" -> 0x50).
pub fn vk_for(key: &str) -> Option<u32> {
    let mut chars = key.trim().chars();
    let c = chars.next()?.to_ascii_uppercase();
    (chars.next().is_none() && c.is_ascii_alphanumeric()).then_some(c as u32)
}

pub const VK_ESCAPE: u32 = 0x1B;
