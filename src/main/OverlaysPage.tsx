import { invoke } from "@tauri-apps/api/core";
import { createResource, For, Show } from "solid-js";
import Key from "../Key";
import type { LayoutsPayload, OverlayLook, Settings } from "../types";
import CloneFramesSection from "./CloneFramesSection";

/** [window label, name, what decides when it shows] */
const OVERLAYS: [string, string, string | null][] = [
	["timer", "Timer", null],
	["guide", "Guide", null],
	["xp", "XP ruler", null],
	["gems", "Gem links", null],
	["notes", "Build notes", null],
	["layouts", "Zone layouts", null],
	["act", "Full act", "Opens from the guide"],
	["lab", "Lab map", "Shows in the Labyrinth"],
	["treebar", "Tree controls", "Shows with the tree overlay"],
];

const SIZES = [0.7, 0.8, 0.9, 1, 1.1, 1.2, 1.3, 1.4, 1.5];

export default function OverlaysPage(props: {
	settings: Settings;
	onSettings: (s: Settings) => void;
	save: (patch: Partial<Settings>) => void;
}) {
	const [shown, { mutate: setShown }] = createResource(() =>
		invoke<Record<string, boolean>>("get_overlays_shown"),
	);
	const [layouts] = createResource(() =>
		invoke<LayoutsPayload>("get_layouts_view"),
	);
	const look = (label: string): OverlayLook =>
		props.settings.overlays?.[label] ?? {
			scale: 1,
			opacity: null,
			shown: null,
		};

	const set = async (label: string, change: Record<string, unknown>) => {
		const next = await invoke<Settings>("set_overlay_look", {
			label,
			...change,
		});
		props.onSettings(next);
		if ("shown" in change)
			setShown({ ...(shown() ?? {}), [label]: change.shown as boolean });
	};

	return (
		<section>
			<h2>Overlays</h2>
			<p class="lede">
				Overlays show while the game is focused; run it in windowed fullscreen.
				Press <Key id="overlays.interact" /> for Interact Mode to drag them
				around, and again when you're done.
			</p>

			<div class="fields">
				<div class="label">Panel opacity</div>
				<div>
					<input
						type="range"
						min="0.2"
						max="1"
						step="0.05"
						value={props.settings.panelOpacity}
						aria-label="Panel opacity"
						onInput={(e) =>
							props.save({ panelOpacity: Number(e.currentTarget.value) })
						}
					/>{" "}
					{Math.round(props.settings.panelOpacity * 100)}%
					<div class="help">
						How solid the overlay backgrounds are. Text stays fully opaque. Each
						overlay can have its own below.
					</div>
				</div>
			</div>

			<table class="overlay-table">
				<thead>
					<tr>
						<th>Overlay</th>
						<th>Showing</th>
						<th>Size</th>
						<th>Opacity</th>
					</tr>
				</thead>
				<tbody>
					<For each={OVERLAYS}>
						{([label, name, auto]) => (
							<tr>
								<td>{name}</td>
								<td>
									<Show
										when={auto === null}
										fallback={<span class="muted small-text">{auto}</span>}
									>
										<input
											type="checkbox"
											aria-label={`Show the ${name.toLowerCase()}`}
											checked={shown()?.[label] ?? false}
											onChange={(e) =>
												set(label, { shown: e.currentTarget.checked })
											}
										/>
									</Show>
								</td>
								<td>
									<select
										aria-label={`${name} size`}
										value={look(label).scale}
										onChange={(e) =>
											set(label, { scale: Number(e.currentTarget.value) })
										}
									>
										<For each={SIZES}>
											{(s) => <option value={s}>{Math.round(s * 100)}%</option>}
										</For>
									</select>
								</td>
								<td class="opacity-cell">
									<Show
										when={look(label).opacity !== null}
										fallback={
											<>
												<span class="muted shared-label">
													Shared,{" "}
													{Math.round(props.settings.panelOpacity * 100)}%
												</span>
												<button
													type="button"
													class="small"
													onClick={() =>
														set(label, { opacity: props.settings.panelOpacity })
													}
												>
													Set its own
												</button>
											</>
										}
									>
										<input
											type="range"
											min="0.2"
											max="1"
											step="0.05"
											aria-label={`${name} opacity`}
											value={look(label).opacity ?? 1}
											onChange={(e) =>
												set(label, { opacity: Number(e.currentTarget.value) })
											}
										/>
										<span class="num-label">
											{Math.round((look(label).opacity ?? 1) * 100)}%
										</span>
										<button
											type="button"
											class="small"
											title="Use the shared opacity"
											onClick={() => set(label, { clearOpacity: true })}
										>
											Shared
										</button>
									</Show>
								</td>
							</tr>
						)}
					</For>
				</tbody>
			</table>
			<div class="buttons">
				<button
					type="button"
					onClick={() => {
						if (confirm("Move every overlay back to where it starts out?"))
							invoke("reset_overlay_positions");
					}}
				>
					Reset positions
				</button>
			</div>

			<CloneFramesSection settings={props.settings} />

			<h3>Zone layouts</h3>
			<p class="muted">
				Put layout images in the layouts folder, named by area ID with a number
				per variant (<code>1_1_2 1.jpg</code>, <code>1_1_2 2.jpg</code>), or in
				a folder named after the area ID. They show when you enter that area;
				click the one that matches your instance. <Key id="guide.layouts" />{" "}
				hides or shows them. The app doesn't come with images.
			</p>
			<Show when={layouts()}>
				{(l) => (
					<p class="muted small-text">
						{l().total === 0
							? "No images yet."
							: `${l().total} files in the folder.`}
					</p>
				)}
			</Show>
			<div class="buttons">
				<button type="button" onClick={() => invoke("open_layouts_folder")}>
					Open layouts folder
				</button>
			</div>

			<h3>Passive tree</h3>
			<p class="muted">
				Open the in-game tree, zoom all the way out and middle-click your class
				start: allocated passives are ringed and the next points numbered.
				Middle-click again after panning. If middle-click isn't available, use
				Calibrate and left-click the class start.
			</p>
			<div class="buttons">
				<button
					type="button"
					onClick={() => invoke("toggle_tree", { calibrate: false })}
				>
					Show or hide <Key id="tree.toggle" />
				</button>
				<button
					type="button"
					onClick={() => invoke("toggle_tree", { calibrate: true })}
				>
					Calibrate <Key id="tree.calibrate" />
				</button>
			</div>
		</section>
	);
}
