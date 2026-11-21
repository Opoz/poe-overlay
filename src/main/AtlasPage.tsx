import { invoke } from "@tauri-apps/api/core";
import { createResource, createSignal, Show } from "solid-js";
import { onEvent } from "../events";
import Key from "../Key";
import type { AtlasView } from "../types";

/** Import an atlas plan and set up its overlay. */
export default function AtlasPage() {
	const [view, { mutate }] = createResource(() =>
		invoke<AtlasView>("get_atlas_view"),
	);
	onEvent<AtlasView>("atlas-view", (v) => mutate(v));
	const [input, setInput] = createSignal("");
	const [message, setMessage] = createSignal<string | null>(null);

	const doImport = async () => {
		if (!input().trim()) return;
		try {
			const n = await invoke<number>("import_atlas", { text: input() });
			setMessage(`Imported a plan with ${n} atlas passives.`);
			setInput("");
		} catch (e) {
			setMessage(`Couldn't import: ${e}`);
		}
	};

	return (
		<section>
			<h2>Atlas</h2>
			<p class="lede">
				Rings your planned atlas passives over the in-game atlas tree. The game
				log doesn't record atlas points, so the overlay doesn't know what you've
				taken: planned nodes the game shows lit are done, dark ones are still to
				take.
			</p>
			<div class="import-row">
				<input
					placeholder="Paste a pathofexile.com or poeplanner atlas tree link"
					value={input()}
					onInput={(e) => setInput(e.currentTarget.value)}
					onKeyDown={(e) => e.key === "Enter" && doImport()}
				/>
				<button
					type="button"
					class="primary"
					disabled={!input().trim()}
					onClick={doImport}
				>
					{(view()?.planned.length ?? 0) > 0 ? "Replace plan" : "Import"}
				</button>
			</div>
			<p class="muted small-text">
				Official atlas tree links and poeplanner atlas links work, as do
				planners' "open on pathofexile.com" links.
			</p>
			<Show when={message()}>
				<p class="muted">{message()}</p>
			</Show>
			<Show when={(view()?.planned.length ?? 0) > 0}>
				<p>
					Your plan has <strong>{view()?.planned.length}</strong> atlas
					passives.{" "}
					<button
						type="button"
						class="small danger"
						onClick={() => invoke("clear_atlas_plan")}
					>
						Clear plan
					</button>
				</p>
			</Show>

			<h3>Overlay</h3>
			<p class="muted">
				Open the atlas tree in game and zoom all the way out. The first time,
				calibrate with two clicks: the atlas start in the centre, then the
				keystone the prompt names. After that one click on the start, or a
				middle-click, lines it up again.{" "}
				<Show when={view()?.zoomKnown}>The zoom is already measured.</Show>
			</p>
			<div class="buttons">
				<button
					type="button"
					onClick={() => invoke("toggle_atlas", { calibrate: false })}
				>
					Show or hide <Key id="atlas.toggle" />
				</button>
				<button
					type="button"
					onClick={() => invoke("toggle_atlas", { calibrate: true })}
				>
					Calibrate <Key id="atlas.calibrate" />
				</button>
			</div>
		</section>
	);
}
