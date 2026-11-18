# Plan

Feature-parity target: Exile UI's levelling tools, rebuilt independently (see [ADR 0004](adr/0004-clean-room-from-exile-ui.md)). Vocabulary: [GLOSSARY.md](../GLOSSARY.md).

## Scope

- Path of Exile 1 only. A greyed-out "PoE2 (coming later)" option in settings; game data kept in per-game files so PoE2 can slot in.
- Windows only. Platform-specific code (window styles, process detection) behind a `platform` module.
- Personal tool first; public-ready later (signed releases, updater in M5).

## Architecture

- `crates/poe-core`: pure Rust domain logic, no Tauri ([ADR 0003](adr/0003-domain-logic-in-poe-core-crate.md)).
- `src-tauri`: Tauri v2 shell: log tailing, hotkeys, windows, tray, persistence.
- `src/`: SolidJS + TypeScript frontend; renders state only.
- Game state from the Client Log only ([ADR 0002](adr/0002-client-log-is-the-only-game-state.md)).
- Overlays are separate click-through windows; borderless game mode required ([ADR 0005](adr/0005-overlays-are-separate-click-through-windows.md)).
- Storage under `%APPDATA%\poe-overlay`: `settings.toml`, run history / PBs / profiles as JSON.

## Behaviour decisions

- **Hotkeys and Overlays** only active while PoE is focused; settings window behaves normally.
- **Main window** lives in the tray; close → tray; quit from tray; no autostart.
- **Overlay positions** stored relative to the PoE window rect.
- **Client Log path** auto-detected from the running PoE process, with manual override.

### Timer

- Auto-start: a new level-1 Character enters The Twilight Strand.
- Splits on first entry to each act's Town (10 splits); ends on reaching the epilogue (Oriath after Kitava). Optional extra split on first map.
- Real time only (no load removal). Automatic Pause after login and (optionally) in Hideout; resumes on entering a campaign Area unless manually paused.
- Manual hotkeys: pause, split, undo split, reset.
- PBs and Best Possible Time per Run Category; every Run saved; Runs tab with table + per-act chart.

### Guide

- Converted at build time from exile-leveling Routes into our own JSON Guide format; script to pull upstream updates.
- Compact Overlay: Current Step + next 2–3. Auto-advance forward only, matching within a small look-ahead window; never auto-jumps backward. Manual forward/back hotkeys; full act view on hotkey.
- Bandit Choice alters Steps. Direction Hints as arrows; Area layout images deferred.
- Custom/overridden Guides: format is swappable from day one; editor UI later.

### Build (PoB)

- Import PoB code, pobb.in or pastebin link.
- Skill Groups mapped to level ranges via PoB labels, else manually assigned.
- Quest Rewards / Vendor Rewards injected into Guide Steps for the Character's class.
- Vendor Regex hotkey: gems for the Current Step's vendor; second press = all Build gems this vendor still sells. ~250 char limit.

### Passive tree

- Rendered from GGG's skill-tree export on a pan/zoom canvas.
- Picks the Tree Stage for current level; highlights diff from previous stage; numbered Next Points via shortest path.
- Aligned mode: two-click Calibration (class start + one named node) over the in-game tree, arrow-key nudge, +/- scale. Panel mode as fallback.

### Lab

- Player copies poelab's daily lab-layout link; app reads clipboard on hotkey, downloads JSON, renders rooms. No scraping.
- Trial completion tracked from the Client Log.

## Default hotkeys (until rebinding in M5)

| Action                           | Key                               |
| -------------------------------- | --------------------------------- |
| Guide step forward / back        | `Alt+]` / `Alt+[`                 |
| Full act view                    | `Alt+G`                           |
| Timer pause / split / undo split | `Alt+P` / `Alt+S` / `Alt+Shift+S` |
| Gem links panel                  | `Alt+L`                           |
| Tree overlay / calibrate         | `Alt+T` / `Alt+Shift+T`           |
| Copy Vendor Regex                | `Alt+R`                           |
| Lab overlay / paste lab link     | `Alt+B` / `Alt+Shift+B`           |
| Interact Mode                    | `Alt+E`                           |

## Milestones

- [ ] **M1**: shell, settings, Client Log tailer + parser, Replay, Area tracking, Character/Profile detection, timer Overlay with splits/PBs.
- [ ] **M1.5**: passive tree POC: render export, Calibration over the in-game tree.
- [ ] **M2**: Guide Overlay, auto-advance, Bandit Choice, Direction Hints.
- [ ] **M3**: PoB import, gem links panel, Quest/Vendor Rewards in Guide, Vendor Regex, full tree Overlay.
- [ ] **M4**: Lab Overlay, Trial tracking.
- [ ] **M5**: hotkey rebinding, layout persistence, Runs tab polish, signed releases + updater.
