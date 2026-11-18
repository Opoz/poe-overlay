# Tauri v2 + SolidJS for the app shell

The app runs several always-on visible overlays next to a demanding game, so memory and CPU cost matter. We chose Tauri v2 (Rust backend, system WebView2) with a SolidJS + TypeScript frontend over Electron (too heavy, bundles Chromium) and pure-Rust egui (lightest, but transparent multi-window overlays and rich styling are painful). Solid's fine-grained reactivity avoids virtual-DOM work in frequently updating overlays like the timer.

## Considered Options

- **Electron**: rejected for memory footprint.
- **egui / iced**: rejected for styling cost and awkward transparent windows.
- **React / Svelte on Tauri**: React heavier for many small windows; Svelte less functional in style.
