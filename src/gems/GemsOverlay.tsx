import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, For, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import type { GemsPayload } from "../types";

/** The Build's Skill Groups for the Character's current level. */
export default function GemsOverlay() {
	const [view, setView] = createSignal<GemsPayload | null>(null);
	onEvent<GemsPayload>("gems-view", setView);
	invoke<GemsPayload>("get_gems_view").then(setView);

	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);
	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	const range = (g: GemsPayload["groups"][number]) => {
		if (!g.levels) return "";
		return g.levels.to
			? `lvl ${g.levels.from}–${g.levels.to}`
			: `lvl ${g.levels.from}+`;
	};

	return (
		<div class="overlay gems" classList={{ interact: interact() }}>
			<header class="guide-bar">
				<span class="muted">Gem links · level {view()?.level ?? "?"}</span>
				<span class="muted">Alt+L</span>
			</header>
			<Show
				when={view()?.hasBuild}
				fallback={
					<p class="muted">
						Import a Path of Building build in the main window.
					</p>
				}
			>
				<Show
					when={(view()?.groups.length ?? 0) > 0}
					fallback={<p class="muted">No skill groups for this level.</p>}
				>
					<For each={view()?.groups ?? []}>
						{(g) => (
							<div class="gem-group">
								<div class="gem-group-title">
									{g.label ?? g.skillSet ?? g.slot ?? "Skill group"}
									<span class="muted"> {range(g)}</span>
								</div>
								<div class="gem-links">
									<For each={g.gems}>
										{(gem) => (
											<span
												class={`gem gem-${gem.colour}`}
												classList={{
													support: gem.support,
													"too-high": gem.tooHigh,
												}}
												title={
													gem.tooHigh
														? "Your level is too low for this gem yet"
														: ""
												}
											>
												{gem.name}
											</span>
										)}
									</For>
								</div>
							</div>
						)}
					</For>
				</Show>
			</Show>
		</div>
	);
}
