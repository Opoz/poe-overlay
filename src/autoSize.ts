import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { onCleanup } from "solid-js";

/**
 * Size this Overlay window to its content (height up to `maxHeight`, and the
 * width too with `fitWidth`), so the invisible parts of a window don't block
 * clicks or get in the way of placing it.
 */
export function autoSizeWindow(
	el: HTMLElement,
	{
		maxHeight = 760,
		fitWidth = false,
	}: { maxHeight?: number; fitWidth?: boolean } = {},
) {
	let last = "";
	const resize = () => {
		const height = Math.min(Math.ceil(el.scrollHeight) + 10, maxHeight);
		const width = fitWidth ? Math.ceil(el.scrollWidth) + 10 : window.innerWidth;
		const key = `${width}x${height}`;
		if (key === last) return;
		last = key;
		getCurrentWindow().setSize(new LogicalSize(width, height));
	};
	const observer = new ResizeObserver(resize);
	observer.observe(el);
	const mutations = new MutationObserver(resize);
	mutations.observe(el, {
		childList: true,
		subtree: true,
		characterData: true,
	});
	resize();
	onCleanup(() => {
		observer.disconnect();
		mutations.disconnect();
	});
}
