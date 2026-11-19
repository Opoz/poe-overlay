import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { autoSizeWindow } from "../autoSize";
import { onEvent } from "../events";
import { NextIcon, PrevIcon } from "../icons";
import type { TreeView } from "../types";

/**
 * Controls for the tree Overlay (which lets clicks through to the game).
 * Collapses to a "Show tree" button while the tree Overlay is hidden.
 */
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

	const toggle = () => invoke("toggle_tree", { calibrate: false });
	const cycle = (delta: number) => invoke("cycle_tree_stage", { delta });
	const auto = () => {
		const name = view()?.character;
		if (name) invoke("set_tree_stage", { name, stage: null });
	};

	return (
		<div
			class="overlay treebar"
			classList={{ interact: interact() }}
			ref={(el) =>
				onMount(() => autoSizeWindow(el, { fitWidth: true, keepRight: true }))
			}
		>
			<Show
				when={view()?.visible}
				fallback={
					<button type="button" onClick={toggle}>
						Show tree <kbd>Alt+T</kbd>
					</button>
				}
			>
				<Show
					when={(view()?.stages.length ?? 0) > 0}
					fallback={
						<span class="muted">Import a PoB build for next points</span>
					}
				>
					<button
						type="button"
						class="icon-button"
						title="Previous tree (Alt+Shift+[)"
						aria-label="Previous tree"
						onClick={() => cycle(-1)}
					>
						<PrevIcon />
					</button>
					<span class="treebar-name" title={view()?.stage ?? ""}>
						{view()?.stage}
					</span>
					<button
						type="button"
						class="icon-button"
						title="Next tree (Alt+Shift+])"
						aria-label="Next tree"
						onClick={() => cycle(1)}
					>
						<NextIcon />
					</button>
					<span class="muted">{view()?.planned.length ?? 0} points to go</span>
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
					classList={{ on: view()?.editing }}
					title="The log can't see points you cancelled in game: click nodes to add or remove them"
					onClick={() => invoke("toggle_tree_edit")}
				>
					{view()?.editing ? "Done fixing" : "Fix points"}
				</button>
				<button type="button" class="treebar-hide" onClick={toggle}>
					Hide <kbd>Alt+T</kbd>
				</button>
			</Show>
		</div>
	);
}
