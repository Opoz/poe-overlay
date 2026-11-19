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
 */
export default function LayoutsOverlay() {
	const [view, setView] = createSignal<LayoutsPayload | null>(null);
	onEvent<LayoutsPayload>("layouts-view", setView);
	invoke<LayoutsPayload>("get_layouts_view").then(setView);

	// The pick is for this instance only: forget it on a new Area.
	const [picked, setPicked] = createSignal<number | null>(null);
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
					{(view()?.images.length ?? 0) > 1 && picked() === null
						? "Click the layout that matches"
						: "Zone layout"}
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
				<For each={view()?.images ?? []}>
					{(img, i) => (
						<button
							type="button"
							class="layout-pick"
							classList={{
								picked: picked() === i(),
								faded: picked() !== null && picked() !== i(),
							}}
							title={img.label || "Layout"}
							onClick={() => setPicked(picked() === i() ? null : i())}
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
