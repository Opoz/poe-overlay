import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import type { Settings } from "../types";

/**
 * A screen copy's window. Windows draws the copy itself; this page only
 * shows an outline and the copy's name in Interact Mode, for moving it.
 */
export default function CloneWindow() {
	const label = getCurrentWindow().label;
	const slot = Number(label.replace("clone-", ""));
	const [name, setName] = createSignal("");
	const apply = (s: Settings) =>
		setName(s.cloneFrames?.find((f) => f.slot === slot)?.name ?? "");
	invoke<Settings>("get_settings").then(apply);
	onEvent<Settings>("settings", apply);

	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);
	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	return (
		<Show when={interact()}>
			<div class="clone-frame">
				<span>{name()}</span>
			</div>
		</Show>
	);
}
