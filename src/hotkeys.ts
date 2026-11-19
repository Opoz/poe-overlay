import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";
import { formatShortcut } from "./format";

/** One rebindable action and its shortcut (backend form, empty = unbound). */
export interface HotkeyRow {
	id: string;
	label: string;
	group: string;
	shortcut: string;
	default: string;
}

const [rows, setRows] = createSignal<HotkeyRow[]>([]);
let loaded = false;

/** The hotkeys, kept current across windows. */
export function hotkeyRows() {
	if (!loaded) {
		loaded = true;
		invoke<HotkeyRow[]>("get_hotkeys").then(setRows);
		listen<HotkeyRow[]>("hotkeys", (e) => setRows(e.payload));
	}
	return rows();
}

export { setRows as setHotkeyRows };

/** An action's shortcut for display (`Alt+S`), or null when unbound. */
export function keyFor(id: string): string | null {
	const row = hotkeyRows().find((r) => r.id === id);
	return row?.shortcut ? formatShortcut(row.shortcut) : null;
}

/** `Title (Alt+S)`, or just the title when unbound. */
export function withKey(title: string, id: string): string {
	const k = keyFor(id);
	return k ? `${title} (${k})` : title;
}

const MODIFIER_CODES = new Set([
	"AltLeft",
	"AltRight",
	"ShiftLeft",
	"ShiftRight",
	"ControlLeft",
	"ControlRight",
	"MetaLeft",
	"MetaRight",
]);

/**
 * A pressed key combination as shortcut text (`alt+shift+KeyS`), or null
 * while only modifiers are down. Plain keys need a modifier (except F1-F24),
 * so typing in the game never fires an overlay.
 */
export function shortcutFromEvent(
	e: KeyboardEvent,
): string | null | "needs-modifier" {
	if (MODIFIER_CODES.has(e.code)) return null;
	const mods = [
		e.ctrlKey && "control",
		e.altKey && "alt",
		e.shiftKey && "shift",
	].filter(Boolean) as string[];
	const fKey = /^F\d{1,2}$/.test(e.code);
	if (mods.length === 0 && !fKey) return "needs-modifier";
	return [...mods, e.code].join("+");
}
