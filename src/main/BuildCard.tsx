import { invoke } from "@tauri-apps/api/core";
import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import Key from "../Key";
import type { BuildSummary, Characters, GuidePayload, Profile } from "../types";
import GroupLevels from "./GroupLevels";

/** Skill set titles in build order. */
function skillSets(p: Profile): string[] {
	const out: string[] = [];
	for (const g of p.build?.skillGroups ?? []) {
		if (g.skillSet && !out.includes(g.skillSet)) out.push(g.skillSet);
	}
	return out;
}

function treeTitles(p: Profile): string[] {
	return (p.build?.treeStages ?? []).map((t, i) => t.title ?? `Tree ${i + 1}`);
}

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
		<section>
			<h2>Build</h2>
			<p class="lede">
				Import a Path of Building build. The gems overlay{" "}
				<Key id="gems.links" /> shows its links for your level, the guide's
				hand-in steps show which gems to take and buy, the tree overlay numbers
				your next passives, and <Key id="notes.toggle" /> shows the build's
				notes.
			</p>
			<Show
				when={active()}
				fallback={
					<p class="empty">
						Level up once so the app knows your character, then import here.
					</p>
				}
			>
				{(p) => (
					<>
						<p>
							<Show
								when={p().build}
								fallback={
									<>
										<strong>{p().name}</strong> has no build yet.
									</>
								}
							>
								{(b) => (
									<>
										<strong>{p().name}</strong> follows a{" "}
										{b().ascendancy ?? b().class} build with{" "}
										{b().skillGroups.length} skill groups and{" "}
										{b().treeStages.length} passive trees.
									</>
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
								class="primary"
								disabled={busy() || !input().trim()}
								onClick={doImport}
							>
								{busy() ? "Importing…" : p().build ? "Replace build" : "Import"}
							</button>
						</div>
						<Show when={p().build}>
							<div class="pickers">
								<label>
									Skill set (gem links)
									<select
										value={p().skillSet ?? ""}
										onChange={(e) =>
											invoke("set_skill_set", {
												name: p().name,
												set: e.currentTarget.value || null,
											}).then(refetch)
										}
									>
										<option value="">Auto (by level)</option>
										<For each={skillSets(p())}>
											{(t) => <option value={t}>{t}</option>}
										</For>
									</select>
								</label>
								<label>
									Passive tree
									<select
										value={p().treeStage ?? ""}
										onChange={(e) =>
											invoke("set_tree_stage", {
												name: p().name,
												stage:
													e.currentTarget.value === ""
														? null
														: Number(e.currentTarget.value),
											}).then(refetch)
										}
									>
										<option value="">Auto (by level)</option>
										<For each={treeTitles(p())}>
											{(t, i) => <option value={i()}>{t}</option>}
										</For>
									</select>
								</label>
							</div>
							<GroupLevels profile={p()} onChange={refetch} />
						</Show>
						<Show when={message()}>
							<p class="muted">{message()}</p>
						</Show>
						<Show when={(guide()?.unobtainable.length ?? 0) > 0}>
							<h3>Find or trade for these</h3>
							<p class="muted">
								No quest or vendor gives these to your class:{" "}
								<For each={guide()?.unobtainable ?? []}>
									{(g) => <span class={`gem gem-${g.colour}`}>{g.name}</span>}
								</For>
							</p>
						</Show>
						<p class="muted">
							At a vendor, <Key id="gems.vendorSearch" /> copies a search for
							the gems to buy there.
						</p>
					</>
				)}
			</Show>
		</section>
	);
}
