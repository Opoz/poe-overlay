import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
} from "solid-js";
import { onEvent } from "../events";
import StepView from "./StepView";
import { toggleActView, useGuide } from "./useGuide";

/** Every Step of the current act, the current one highlighted. */
export default function ActOverlay() {
	const guide = useGuide();
	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);

	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	// Keep the current Step in view.
	let list: HTMLDivElement | undefined;
	const currentId = createMemo(() =>
		guide()?.actView ? guide()?.current?.id : undefined,
	);
	createEffect(() => {
		const id = currentId();
		if (id === undefined) return;
		queueMicrotask(() =>
			list
				?.querySelector(`[data-step="${id}"]`)
				?.scrollIntoView({ block: "center" }),
		);
	});

	const act = () => guide()?.current?.act ?? 1;

	return (
		<div class="overlay act" classList={{ interact: interact() }}>
			<header class="guide-bar">
				<strong>Act {act()}</strong>
				<button type="button" title="Close (Alt+G)" onClick={toggleActView}>
					✕
				</button>
			</header>
			<div class="act-steps" ref={list}>
				<For each={guide()?.actSteps ?? []}>
					{(step) => (
						<div
							data-step={step.id}
							class="act-step"
							classList={{ current: step.id === guide()?.current?.id }}
						>
							<StepView step={step} />
						</div>
					)}
				</For>
			</div>
		</div>
	);
}
