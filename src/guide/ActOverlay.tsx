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
import { guideCommand, toggleActView, useGuide } from "./useGuide";

const ACTS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/** Every Step of an act; click a Step to make it the current one. */
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

	const currentAct = () => guide()?.current?.act ?? 1;
	const shownAct = () => guide()?.act ?? currentAct();
	const browse = (act: number) =>
		invoke("browse_act", { act: act === currentAct() ? null : act });

	// Keep the current Step in view (when showing its act).
	let list: HTMLDivElement | undefined;
	const currentId = createMemo(() =>
		guide()?.actView && shownAct() === currentAct()
			? guide()?.current?.id
			: undefined,
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

	return (
		<div class="overlay act" classList={{ interact: interact() }}>
			<header class="guide-bar">
				<span class="act-tabs">
					<For each={ACTS}>
						{(act) => (
							<button
								type="button"
								classList={{
									on: act === shownAct(),
									here: act === currentAct(),
								}}
								title={
									act === currentAct() ? "Your current act" : `Show act ${act}`
								}
								onClick={() => browse(act)}
							>
								{act}
							</button>
						)}
					</For>
				</span>
				<button type="button" title="Close (Alt+G)" onClick={toggleActView}>
					✕
				</button>
			</header>
			<div class="muted act-hint">
				Click a step to make it your current step.
			</div>
			<div class="act-steps" ref={list}>
				<For each={guide()?.actSteps ?? []}>
					{(step) => (
						<button
							type="button"
							data-step={step.id}
							class="act-step"
							classList={{ current: step.id === guide()?.current?.id }}
							onClick={() => guideCommand({ kind: "jump", step: step.id })}
						>
							<StepView step={step} />
						</button>
					)}
				</For>
			</div>
		</div>
	);
}
