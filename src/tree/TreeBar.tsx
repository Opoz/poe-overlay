import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import type { TreeView } from "../types";

/** Controls for the tree Overlay: which PoB tree, points left, hide. */
export default function TreeBar() {
	const [view, setView] = createSignal<TreeView | null>(null);
	onEvent<TreeView>("tree-view", setView);
	invoke<TreeView>("get_tree_view").then(setView);

	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);
	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	const cycle = (delta: number) => invoke("cycle_tree_stage", { delta });
	const auto = () => {
		const name = view()?.character;
		if (name) invoke("set_tree_stage", { name, stage: null });
	};

	return (
		<div class="overlay treebar" classList={{ interact: interact() }}>
			<Show
				when={(view()?.stages.length ?? 0) > 0}
				fallback={
					<span class="muted">
						Passive tree overlay · import a PoB build for next points
					</span>
				}
			>
				<button
					type="button"
					title="Previous tree (Alt+Shift+[)"
					onClick={() => cycle(-1)}
				>
					◀
				</button>
				<span class="treebar-name" title={view()?.stage ?? ""}>
					{view()?.stage}
				</span>
				<button
					type="button"
					title="Next tree (Alt+Shift+])"
					onClick={() => cycle(1)}
				>
					▶
				</button>
				<span class="muted">· {view()?.planned.length ?? 0} points to go</span>
				<Show when={view()?.stageByHand}>
					<button
						type="button"
						title="Pick the tree by level again"
						onClick={auto}
					>
						Auto
					</button>
				</Show>
			</Show>
			<button
				type="button"
				class="treebar-hide"
				title="Hide the tree overlay (Alt+T)"
				onClick={() => invoke("toggle_tree", { calibrate: false })}
			>
				Hide
			</button>
		</div>
	);
}
