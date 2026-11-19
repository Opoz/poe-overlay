import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, For, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import StepView from "./StepView";
import { guideCommand, toggleActView, useGuide } from "./useGuide";

/** Current Step in full, the next few dimmed, with navigation buttons. */
export default function GuideOverlay() {
	const guide = useGuide();
	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);

	// In Interact Mode the panel is a drag handle; otherwise buttons work.
	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	return (
		<div class="overlay guide" classList={{ interact: interact() }}>
			<header class="guide-bar">
				<span class="muted">
					<Show when={guide()} fallback="Guide">
						{(g) => `Step ${g().position + 1}/${g().total}`}
					</Show>
				</span>
				<span class="guide-buttons">
					<button
						type="button"
						title="Previous step (Alt+[)"
						onClick={() => guideCommand("previous")}
					>
						◀
					</button>
					<button
						type="button"
						title="Next step (Alt+])"
						onClick={() => guideCommand("next")}
					>
						▶
					</button>
					<button
						type="button"
						title="Find me: jump to the area you're in (Alt+J)"
						onClick={() => guideCommand("findMe")}
					>
						⌖
					</button>
					<button
						type="button"
						title="Full act (Alt+G)"
						classList={{ on: guide()?.actView }}
						onClick={toggleActView}
					>
						☰
					</button>
				</span>
			</header>
			<Show when={interact()}>
				<div class="hint">drag · Alt+E to lock</div>
			</Show>
			<Show
				when={guide()?.current}
				fallback={
					<p class="muted">Enter a campaign area to start the guide.</p>
				}
			>
				{(step) => <StepView step={step()} />}
			</Show>
			<div class="upcoming">
				<For each={guide()?.upcoming ?? []}>
					{(step) => <StepView step={step} compact />}
				</For>
			</div>
		</div>
	);
}
