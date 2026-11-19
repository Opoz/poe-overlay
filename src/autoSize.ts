import {
	getCurrentWindow,
	LogicalSize,
	PhysicalPosition,
} from "@tauri-apps/api/window";
import { createEffect, onCleanup } from "solid-js";
import { overlayScale } from "./look";

/**
 * Size this Overlay window to its content (height up to `maxHeight`, and the
 * width too with `fitWidth`), so the invisible parts of a window don't block
 * clicks or get in the way of placing it. With `keepRight`, the right edge
 * stays put as the width changes (no jump while the app catches up).
 *
 * Measurements are in CSS pixels; the window is that times the Overlay's
 * size setting (the backend zooms the web view by it).
 */
export function autoSizeWindow(
	el: HTMLElement,
	{
		maxHeight = 760,
		fitWidth = false,
		keepRight = false,
	}: { maxHeight?: number; fitWidth?: boolean; keepRight?: boolean } = {},
) {
	let last = "";
	const resize = async () => {
		const zoom = overlayScale();
		const height = Math.min(Math.ceil(el.scrollHeight) + 10, maxHeight) * zoom;
		const width =
			(fitWidth ? Math.ceil(el.scrollWidth) + 10 : window.innerWidth) * zoom;
		const key = `${width}x${height}`;
		if (key === last) return;
		last = key;
		const win = getCurrentWindow();
		if (keepRight) {
			const [pos, size, scale] = await Promise.all([
				win.outerPosition(),
				win.outerSize(),
				win.scaleFactor(),
			]);
			const newWidth = Math.round(width * scale);
			await win.setSize(new LogicalSize(width, height));
			await win.setPosition(
				new PhysicalPosition(pos.x + size.width - newWidth, pos.y),
			);
		} else {
			await win.setSize(new LogicalSize(width, height));
		}
	};
	const observer = new ResizeObserver(() => void resize());
	observer.observe(el);
	const mutations = new MutationObserver(() => void resize());
	mutations.observe(el, {
		childList: true,
		subtree: true,
		characterData: true,
	});
	// A new size setting: measure again.
	createEffect(() => {
		overlayScale();
		last = "";
		void resize();
	});
	onCleanup(() => {
		observer.disconnect();
		mutations.disconnect();
	});
}
