import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
	createEffect,
	createSignal,
	For,
	on,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import { autoSizeWindow } from "../autoSize";
import { onEvent } from "../events";
import { withKey } from "../hotkeys";
import { CloseIcon } from "../icons";
import type { LayoutsPayload } from "../types";

/**
 * The player's layout images for the current Area, side by side. Clicking
 * the one that matches this instance keeps it large and fades the rest.
 *
 * Variants can have follow-up steps, named after them: `1_1_2 1` is a
 * variant, `1_1_2 1_1` and `1_1_2 1_2` what to look for next within it. Only
 * variants show at first; picking one shows it with its next steps.
 */
export default function LayoutsOverlay() {
	const [view, setView] = createSignal<LayoutsPayload | null>(null);
	onEvent<LayoutsPayload>("layouts-view", setView);
	invoke<LayoutsPayload>("get_layouts_view").then(setView);

	// The pick is for this instance only: forget it on a new Area.
	const [picked, setPicked] = createSignal<string | null>(null);
	const images = () => view()?.images ?? [];
	const children = (label: string) =>
		images().filter(
			(i) =>
				i.label.startsWith(`${label}_`) &&
				!i.label.slice(label.length + 1).includes("_"),
		);
	const parentOf = (label: string) =>
		label.includes("_") ? label.slice(0, label.lastIndexOf("_")) : null;
	const siblings = (label: string) => {
		const parent = parentOf(label);
		return parent === null
			? images().filter((i) => !i.label.includes("_"))
			: children(parent);
	};
	/** What to show: variants, or a pick with its next steps. */
	const shown = () => {
		const p = picked();
		const all = images();
		if (p === null) {
			const top = all.filter((i) => !i.label.includes("_"));
			return top.length > 0 ? top : all;
		}
		const next = children(p);
		const self = all.filter((i) => i.label === p);
		return next.length > 0 ? [...self, ...next] : siblings(p);
	};
	const isLeafPick = () => {
		const p = picked();
		return p !== null && children(p).length === 0;
	};
	createEffect(
		on(
			() => view()?.areaId,
			() => setPicked(null),
		),
	);

	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);
	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	return (
		<div
			class="overlay layouts"
			classList={{ interact: interact() }}
			ref={(el) => onMount(() => autoSizeWindow(el, { fitWidth: true }))}
		>
			<header class="guide-bar">
				<span>
					{picked() === null
						? shown().length > 1
							? "Click the layout that matches"
							: "Zone layout"
						: isLeafPick()
							? "Zone layout"
							: "Then look for:"}
					<Show when={picked() !== null}>
						{" "}
						<button type="button" class="small" onClick={() => setPicked(null)}>
							All layouts
						</button>
					</Show>
				</span>
				<button
					type="button"
					class="icon-button"
					title={withKey("Hide", "guide.layouts")}
					aria-label="Hide"
					onClick={() =>
						invoke("set_overlay_look", { label: "layouts", shown: false })
					}
				>
					<CloseIcon />
				</button>
			</header>
			<div class="layout-row">
				<For each={shown()}>
					{(img) => (
						<button
							type="button"
							class="layout-pick"
							classList={{
								picked: picked() === img.label,
								faded: isLeafPick() && picked() !== img.label,
							}}
							title={img.label || "Layout"}
							onClick={() =>
								setPicked(
									picked() === img.label ? parentOf(img.label) : img.label,
								)
							}
						>
							<img src={img.src} alt={img.label || "Zone layout"} />
							<Show when={img.label}>
								<span>{img.label}</span>
							</Show>
						</button>
					)}
				</For>
			</div>
		</div>
	);
}
