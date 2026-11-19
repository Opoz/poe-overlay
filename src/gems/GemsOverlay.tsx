import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { autoSizeWindow } from "../autoSize";
import { onEvent } from "../events";
import { NextIcon, PrevIcon } from "../icons";
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
		<div
			class="overlay gems"
			classList={{ interact: interact() }}
			ref={(el) => onMount(() => autoSizeWindow(el))}
		>
			<header class="guide-bar">
				<span class="muted">
					{view()?.skillSet ?? "Gem links"} · level {view()?.level ?? "?"}
				</span>
				<Show when={(view()?.skillSets.length ?? 0) > 1}>
					<span class="guide-buttons">
						<button
							type="button"
							class="icon-button"
							title="Previous skill set"
							aria-label="Previous skill set"
							onClick={() => invoke("cycle_skill_set", { delta: -1 })}
						>
							<PrevIcon />
						</button>
						<button
							type="button"
							class="icon-button"
							title="Next skill set"
							aria-label="Next skill set"
							onClick={() => invoke("cycle_skill_set", { delta: 1 })}
						>
							<NextIcon />
						</button>
						<Show when={view()?.skillSetByHand}>
							<button
								type="button"
								title="Pick the skill set by level again"
								onClick={() => {
									const name = view()?.character;
									if (name) invoke("set_skill_set", { name, set: null });
								}}
							>
								Auto
							</button>
						</Show>
					</span>
				</Show>
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
