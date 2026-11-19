import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal } from "solid-js";
import type { Settings } from "./types";

const [scale, setScale] = createSignal(1);

/** This Overlay's size setting (1 = 100%); the backend zooms the web view. */
export const overlayScale = scale;

/**
 * Overlay windows: follow their look settings. Only the panel's background
 * fades with opacity (the `--veil` custom property); text stays opaque.
 */
export function followLook() {
	const label = getCurrentWindow().label;
	const apply = (s: Settings) => {
		const look = s.overlays?.[label];
		const o = Math.min(
			1,
			Math.max(0.2, look?.opacity ?? s.panelOpacity ?? 0.85),
		);
		document.documentElement.style.setProperty("--veil", String(o));
		setScale(look?.scale ?? 1);
	};
	invoke<Settings>("get_settings").then(apply);
	listen<Settings>("settings", (e) => apply(e.payload));
}
