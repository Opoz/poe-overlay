import { invoke } from "@tauri-apps/api/core";
import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import type { BuildSummary, Characters, GuidePayload } from "../types";
import GroupLevels from "./GroupLevels";

/** Import a Path of Building build for the Active Character. */
export default function BuildCard() {
	const [characters, { refetch }] = createResource(() =>
		invoke<Characters>("get_characters"),
	);
	const poll = setInterval(refetch, 3000);
	onCleanup(() => clearInterval(poll));

	const [guide, setGuide] = createSignal<GuidePayload | null>(null);
	onEvent<GuidePayload>("guide-view", setGuide);
	invoke<GuidePayload>("get_guide_view").then(setGuide);

	const [input, setInput] = createSignal("");
	const [busy, setBusy] = createSignal(false);
	const [message, setMessage] = createSignal<string | null>(null);

	const active = () => {
		const c = characters();
		return c?.profiles.find((p) => p.name === c.active) ?? null;
	};

	const doImport = async () => {
		const name = active()?.name;
		if (!name || !input().trim()) return;
		setBusy(true);
		setMessage(null);
		try {
			const s = await invoke<BuildSummary>("import_build", {
				name,
				input: input(),
			});
			setMessage(
				`Imported ${s.ascendancy ?? s.class}: ${s.skillGroups} skill groups, ${s.treeStages} passive trees.`,
			);
			setInput("");
			refetch();
		} catch (e) {
			setMessage(`Could not import: ${e}`);
		} finally {
			setBusy(false);
		}
	};

	return (
		<section class="card">
			<h2>Build (Path of Building)</h2>
			<Show
				when={active()}
				fallback={
					<p class="muted">
						Level up once so the app knows your character, then import here.
					</p>
				}
			>
				{(p) => (
					<>
						<p>
							For <strong>{p().name}</strong>
							<Show when={p().build}>
								{(b) => (
									<span class="muted">
										{" "}
										· current build: {b().ascendancy ?? b().class},{" "}
										{b().skillGroups.length} skill groups,{" "}
										{b().treeStages.length} passive trees
									</span>
								)}
							</Show>
						</p>
						<div class="import-row">
							<input
								placeholder="Paste a PoB code, pobb.in or pastebin link"
								value={input()}
								onInput={(e) => setInput(e.currentTarget.value)}
								onKeyDown={(e) => e.key === "Enter" && doImport()}
							/>
							<button
								type="button"
								disabled={busy() || !input().trim()}
								onClick={doImport}
							>
								{busy() ? "Importing…" : "Import"}
							</button>
						</div>
						<Show when={p().build}>
							<GroupLevels profile={p()} onChange={refetch} />
						</Show>
						<Show when={message()}>
							<p class="muted">{message()}</p>
						</Show>
						<Show when={(guide()?.unobtainable.length ?? 0) > 0}>
							<p class="muted">
								Not available from quests or vendors for this class (find or
								trade for these):{" "}
								<For each={guide()?.unobtainable ?? []}>
									{(g) => <span class={`gem gem-${g.colour}`}>{g.name}</span>}
								</For>
							</p>
						</Show>
						<p class="muted">
							Gem rewards and purchases appear on the guide's hand-in steps.
							Alt+L shows the gem links for your level; Alt+R copies the vendor
							search for the current step.
						</p>
					</>
				)}
			</Show>
		</section>
	);
}
