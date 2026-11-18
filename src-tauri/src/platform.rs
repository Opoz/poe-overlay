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

/// Make a window never take focus when clicked and stay out of Alt+Tab, so
/// the game keeps keyboard focus even while an Overlay is being dragged.
pub fn make_non_activating(window: RawHwnd) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    let h = hwnd(window);
    unsafe {
        let style = GetWindowLongPtrW(h, GWL_EXSTYLE);
        SetWindowLongPtrW(
            h,
            GWL_EXSTYLE,
            style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize,
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
