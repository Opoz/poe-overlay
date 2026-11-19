//! The app's global hotkeys: what each does, its default shortcut, and the
//! player's rebindings (stored in Settings as action id -> shortcut text).

use std::{collections::BTreeMap, str::FromStr};

use poe_core::{guide::GuideCommand, run::Command};
use serde::Serialize;
use tauri_plugin_global_shortcut::Shortcut;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hotkey {
    Timer(Command),
    InteractMode,
    TreeOverlay,
    Calibrate,
    Guide(GuideCommand),
    ActView,
    GemLinks,
    VendorRegex,
    LabOverlay,
    LabImport,
    XpRuler,
    /// Show the previous / next Tree Stage.
    TreeStage(i32),
}

/// One rebindable action.
pub struct Action {
    /// Stable id, the key in Settings.
    pub id: &'static str,
    pub hotkey: Hotkey,
    pub default: &'static str,
    /// What it does, for the shortcuts list.
    pub label: &'static str,
    pub group: &'static str,
}

const fn action(
    id: &'static str,
    hotkey: Hotkey,
    default: &'static str,
    label: &'static str,
    group: &'static str,
) -> Action {
    Action {
        id,
        hotkey,
        default,
        label,
        group,
    }
}

pub const ACTIONS: [Action; 17] = [
    action(
        "timer.pause",
        Hotkey::Timer(Command::TogglePause),
        "alt+KeyP",
        "Pause or resume the timer",
        "Timer",
    ),
    action(
        "timer.split",
        Hotkey::Timer(Command::Split),
        "alt+KeyS",
        "Split",
        "Timer",
    ),
    action(
        "timer.undoSplit",
        Hotkey::Timer(Command::UndoSplit),
        "alt+shift+KeyS",
        "Undo the last split",
        "Timer",
    ),
    action(
        "guide.next",
        Hotkey::Guide(GuideCommand::Next),
        "alt+BracketRight",
        "Next step",
        "Guide",
    ),
    action(
        "guide.previous",
        Hotkey::Guide(GuideCommand::Previous),
        "alt+BracketLeft",
        "Previous step",
        "Guide",
    ),
    action(
        "guide.findMe",
        Hotkey::Guide(GuideCommand::FindMe),
        "alt+KeyJ",
        "Find me: jump to the area you're in",
        "Guide",
    ),
    action(
        "guide.act",
        Hotkey::ActView,
        "alt+KeyG",
        "Show the full act",
        "Guide",
    ),
    action(
        "gems.links",
        Hotkey::GemLinks,
        "alt+KeyL",
        "Show gem links",
        "Gems",
    ),
    // Not Alt+R: the NVIDIA App takes it for its performance overlay.
    action(
        "gems.vendorSearch",
        Hotkey::VendorRegex,
        "alt+KeyV",
        "Copy the vendor search",
        "Gems",
    ),
    action(
        "tree.toggle",
        Hotkey::TreeOverlay,
        "alt+KeyT",
        "Show or hide the tree overlay",
        "Passive tree",
    ),
    action(
        "tree.calibrate",
        Hotkey::Calibrate,
        "alt+shift+KeyT",
        "Calibrate the tree overlay",
        "Passive tree",
    ),
    action(
        "tree.previous",
        Hotkey::TreeStage(-1),
        "alt+shift+BracketLeft",
        "Previous build tree",
        "Passive tree",
    ),
    action(
        "tree.next",
        Hotkey::TreeStage(1),
        "alt+shift+BracketRight",
        "Next build tree",
        "Passive tree",
    ),
    action(
        "lab.toggle",
        Hotkey::LabOverlay,
        "alt+KeyB",
        "Show or hide the lab map",
        "Labyrinth",
    ),
    action(
        "lab.import",
        Hotkey::LabImport,
        "alt+shift+KeyB",
        "Open today's lab on poelab",
        "Labyrinth",
    ),
    action(
        "xp.toggle",
        Hotkey::XpRuler,
        "alt+KeyX",
        "Show or hide the XP ruler",
        "Overlays",
    ),
    action(
        "overlays.interact",
        Hotkey::InteractMode,
        "alt+KeyE",
        "Unlock overlays for moving",
        "Overlays",
    ),
];

/// Parse a shortcut as stored (`alt+KeyS`) or as typed (`Alt+S`).
pub fn parse(text: &str) -> Option<Shortcut> {
    Shortcut::from_str(text).ok()
}

/// The shortcut text an action is bound to: the player's, else the default.
/// Empty means unbound.
pub fn binding<'a>(overrides: &'a BTreeMap<String, String>, action: &'a Action) -> &'a str {
    overrides
        .get(action.id)
        .map_or(action.default, String::as_str)
}

/// Every bound shortcut and what it does.
pub fn bindings(overrides: &BTreeMap<String, String>) -> Vec<(Shortcut, Hotkey)> {
    ACTIONS
        .iter()
        .filter_map(|a| Some((parse(binding(overrides, a))?, a.hotkey)))
        .collect()
}

/// One row of the shortcuts list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyRow {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    /// Canonical shortcut text (`alt+KeyS`), empty when unbound.
    pub shortcut: String,
    pub default: String,
}

pub fn rows(overrides: &BTreeMap<String, String>) -> Vec<HotkeyRow> {
    ACTIONS
        .iter()
        .map(|a| HotkeyRow {
            id: a.id,
            label: a.label,
            group: a.group,
            shortcut: parse(binding(overrides, a)).map_or_else(String::new, Shortcut::into_string),
            default: parse(a.default).map_or_else(String::new, Shortcut::into_string),
        })
        .collect()
}

/// Bind `id` to `text` (`None` unbinds it). Fails, naming the other action,
/// if the shortcut is already used.
pub fn rebind(
    overrides: &mut BTreeMap<String, String>,
    id: &str,
    text: Option<&str>,
) -> Result<(), String> {
    let action = ACTIONS
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| format!("unknown action {id}"))?;
    let shortcut = match text {
        None => None,
        Some(t) => Some(parse(t).ok_or_else(|| format!("{t} isn't a shortcut"))?),
    };
    if let Some(s) = shortcut
        && let Some(other) = ACTIONS
            .iter()
            .find(|o| o.id != id && parse(binding(overrides, o)) == Some(s))
    {
        return Err(format!("Already used for \"{}\"", other.label));
    }
    let stored = shortcut.map_or_else(String::new, Shortcut::into_string);
    let default = parse(action.default).map(Shortcut::into_string);
    if Some(&stored) == default.as_ref() {
        overrides.remove(id);
    } else {
        overrides.insert(id.to_owned(), stored);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_and_are_distinct() {
        let all = bindings(&BTreeMap::new());
        assert_eq!(all.len(), ACTIONS.len());
        for (i, (s, _)) in all.iter().enumerate() {
            assert!(all[i + 1..].iter().all(|(o, _)| o != s), "{s:?} twice");
        }
    }

    #[test]
    fn rebinding_refuses_duplicates_and_can_unbind() {
        let mut o = BTreeMap::new();
        let err = rebind(&mut o, "timer.split", Some("alt+KeyP")).unwrap_err();
        assert!(err.contains("Pause"), "{err}");
        rebind(&mut o, "timer.split", Some("Alt+F")).unwrap();
        assert_eq!(rows(&o)[1].shortcut, "alt+KeyF");
        rebind(&mut o, "timer.split", None).unwrap();
        assert_eq!(rows(&o)[1].shortcut, "");
        assert_eq!(bindings(&o).len(), ACTIONS.len() - 1);
        // Back to the default: no override stored.
        rebind(&mut o, "timer.split", Some("alt+KeyS")).unwrap();
        assert!(o.is_empty());
    }
}
