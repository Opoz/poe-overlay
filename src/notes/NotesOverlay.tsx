import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { autoSizeWindow } from "../autoSize";
import { onEvent } from "../events";
import { withKey } from "../hotkeys";
import { CloseIcon } from "../icons";
import type { NotesPayload } from "../types";

/** The Build's notes from Path of Building, in their own colours. */
export default function NotesOverlay() {
	const [view, setView] = createSignal<NotesPayload | null>(null);
	onEvent<NotesPayload>("notes-view", setView);
	invoke<NotesPayload>("get_notes_view").then(setView);

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
			class="overlay notes"
			classList={{ interact: interact() }}
			ref={(el) => onMount(() => autoSizeWindow(el, { maxHeight: 640 }))}
		>
			<header class="guide-bar">
				<span>Build notes</span>
				<button
					type="button"
					class="icon-button"
					title={withKey("Hide", "notes.toggle")}
					aria-label="Hide"
					onClick={() => invoke("toggle_notes")}
				>
					<CloseIcon />
				</button>
			</header>
			<Show
				when={(view()?.lines.length ?? 0) > 0}
				fallback={
					<p class="muted">
						{view()?.character
							? "This build has no notes. Import one with notes from Path of Building in the main window."
							: "Import a Path of Building build in the main window."}
					</p>
				}
			>
				<div class="notes-body">
					<For each={view()?.lines ?? []}>
						{(line) => (
							<Show when={line.length > 0} fallback={<div class="notes-gap" />}>
								<p>
									<For each={line}>
										{(span) => (
											<span
												style={span.colour ? { color: span.colour } : undefined}
											>
												{span.text}
											</span>
										)}
									</For>
								</p>
							</Show>
						)}
					</For>
				</div>
			</Show>
		</div>
	);
}
