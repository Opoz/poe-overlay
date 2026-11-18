# Domain logic lives in a Tauri-free `poe-core` crate

Log parsing, Run/Split timing, Guide progression, Build import and Vendor Regex generation live in `crates/poe-core`, a plain Rust library with no Tauri, window or OS dependencies. `src-tauri` is a thin shell that wires `poe-core` to the filesystem, hotkeys, windows and the frontend. This keeps the core fast to test (including Replay of recorded Client Logs) and keeps game rules out of UI code; the frontend only renders state it is sent.
