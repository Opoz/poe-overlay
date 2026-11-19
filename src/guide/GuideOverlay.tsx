import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { autoSizeWindow } from "../autoSize";
import { onEvent } from "../events";
import { withKey } from "../hotkeys";
import { FindIcon, ListIcon, NextIcon, PrevIcon, SearchIcon } from "../icons";
import type { RegexCopied } from "../types";
import StepView from "./StepView";
import { guideCommand, toggleActView, useGuide } from "./useGuide";

/** Current Step in full, the next few dimmed, with navigation buttons. */
export default function GuideOverlay() {
	const guide = useGuide();
	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);

	const [note, setNote] = createSignal<string | null>(null);
	let hideNote: number | undefined;
	onEvent<string>("guide-message", (m) => {
		setNote(m);
		clearTimeout(hideNote);
		hideNote = window.setTimeout(() => setNote(null), 4000);
	});

	const [copied, setCopied] = createSignal<RegexCopied | null>(null);
	let hideCopied: number | undefined;
	onEvent<RegexCopied>("regex-copied", (c) => {
		setCopied(c);
		clearTimeout(hideCopied);
		hideCopied = window.setTimeout(() => setCopied(null), 3500);
	});

	// In Interact Mode the panel is a drag handle; otherwise buttons work.
	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	return (
		<div
			class="overlay guide"
			classList={{ interact: interact() }}
			ref={(el) => onMount(() => autoSizeWindow(el))}
		>
			<header class="guide-bar">
				<span>
					<Show when={guide()} fallback="Guide">
						{(g) =>
							g().complete
								? "Campaign complete"
								: `Step ${g().position + 1} of ${g().total}`
						}
					</Show>
				</span>
				<span class="guide-buttons">
					<button
						type="button"
						class="icon-button"
						title={withKey("Previous step", "guide.previous")}
						aria-label="Previous step"
						onClick={() => guideCommand({ kind: "previous" })}
					>
						<PrevIcon />
					</button>
					<button
						type="button"
						class="icon-button"
						title={withKey("Next step", "guide.next")}
						aria-label="Next step"
						onClick={() => guideCommand({ kind: "next" })}
					>
						<NextIcon />
					</button>
					<button
						type="button"
						class="icon-button"
						title={withKey(
							"Find me: jump to the area you're in",
							"guide.findMe",
						)}
						aria-label="Find me"
						onClick={() => guideCommand({ kind: "findMe" })}
					>
						<FindIcon />
					</button>
					<Show when={(guide()?.vendorRegex.length ?? 0) > 0}>
						<button
							type="button"
							class="icon-button"
							title={withKey(
								"Copy the vendor search for the gems to buy here",
								"gems.vendorSearch",
							)}
							aria-label="Copy vendor search"
							onClick={() => invoke("copy_regex")}
						>
							<SearchIcon />
						</button>
					</Show>
					<button
						type="button"
						class="icon-button"
						title={withKey("Full act", "guide.act")}
						aria-label="Full act"
						classList={{ on: guide()?.actView }}
						onClick={toggleActView}
					>
						<ListIcon />
					</button>
				</span>
			</header>
			<Show when={note()}>
				<div class="toast">{note()}</div>
			</Show>
			<Show when={copied()}>
				{(c) => (
					<div class="toast">
						{c().text
							? `Vendor search copied${c().of > 1 ? ` (part ${c().part} of ${c().of})` : ""}: paste it into the vendor's search box`
							: "Nothing to buy at this step"}
					</div>
				)}
			</Show>
			<Show when={interact()}>
				<div class="interact-hint">
					Drag to move. {withKey("Lock", "overlays.interact")} when done.
				</div>
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
