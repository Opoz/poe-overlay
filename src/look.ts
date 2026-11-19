import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Settings } from "./types";

/**
 * Overlay windows: follow the Panel opacity setting. Only the panel's
 * background fades (the `--veil` custom property); text stays fully opaque.
 */
export function followPanelOpacity() {
	const apply = (s: Settings) => {
		const o = Math.min(1, Math.max(0.2, s.panelOpacity ?? 0.85));
		document.documentElement.style.setProperty("--veil", String(o));
	};
	invoke<Settings>("get_settings").then(apply);
	listen<Settings>("settings", (e) => apply(e.payload));
}
