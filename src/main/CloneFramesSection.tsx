import { invoke } from "@tauri-apps/api/core";
import { createSignal, For, Show } from "solid-js";
import Key from "../Key";
import type { CloneFrameSettings, Settings } from "../types";

const SIZES = [0.5, 0.75, 1, 1.25, 1.5, 2, 2.5, 3, 4];

/** Screen copies: live copies of parts of the game, shown elsewhere. */
export default function CloneFramesSection(props: { settings: Settings }) {
	const frames = () =>
		[...(props.settings.cloneFrames ?? [])].sort((a, b) => a.slot - b.slot);
	const [hint, setHint] = createSignal<string | null>(null);

	const add = async () => {
		try {
			await invoke("add_clone_frame");
			setHint(
				"Switch to the game and drag a box around what to copy. Right-click cancels.",
			);
		} catch (e) {
			setHint(String(e));
		}
	};
	const set = (f: CloneFrameSettings, change: Record<string, unknown>) =>
		invoke("set_clone_frame", { slot: f.slot, ...change });

	return (
		<>
			<h3>Screen copies</h3>
			<p class="muted">
				Copy part of the game somewhere easier to see: your flasks next to your
				character, a buff or cooldown bigger. Windows draws the copy; the app
				never reads or acts on it. Move copies in Interact Mode (
				<Key id="overlays.interact" />
				).
			</p>
			<Show when={frames().length > 0}>
				<table class="overlay-table">
					<thead>
						<tr>
							<th>Copy</th>
							<th>Showing</th>
							<th>Size</th>
							<th>Opacity</th>
							<th />
						</tr>
					</thead>
					<tbody>
						<For each={frames()}>
							{(f) => (
								<tr>
									<td>
										<input
											class="name-input"
											value={f.name}
											aria-label="Name"
											onChange={(e) => set(f, { name: e.currentTarget.value })}
										/>
										<Show when={!f.source[2]}>
											<div class="muted small-text">
												Waiting for you to drag out its area in the game.
											</div>
										</Show>
									</td>
									<td>
										<input
											type="checkbox"
											aria-label={`Show ${f.name}`}
											checked={f.enabled}
											onChange={(e) =>
												set(f, { enabled: e.currentTarget.checked })
											}
										/>
									</td>
									<td>
										<select
											aria-label={`${f.name} size`}
											value={f.scale}
											onChange={(e) =>
												set(f, { scale: Number(e.currentTarget.value) })
											}
										>
											<For each={SIZES}>
												{(s) => (
													<option value={s}>{Math.round(s * 100)}%</option>
												)}
											</For>
										</select>
									</td>
									<td class="opacity-cell">
										<input
											type="range"
											min="0.2"
											max="1"
											step="0.05"
											aria-label={`${f.name} opacity`}
											value={f.opacity}
											onChange={(e) =>
												set(f, { opacity: Number(e.currentTarget.value) })
											}
										/>
										<span class="num-label">
											{Math.round(f.opacity * 100)}%
										</span>
									</td>
									<td class="row-actions">
										<button
											type="button"
											class="small"
											onClick={() => {
												invoke("repick_clone_frame", { slot: f.slot });
												setHint(
													"Switch to the game and drag a new box. Right-click cancels.",
												);
											}}
										>
											Pick area
										</button>{" "}
										<button
											type="button"
											class="small danger"
											onClick={() =>
												invoke("delete_clone_frame", { slot: f.slot })
											}
										>
											Delete
										</button>
									</td>
								</tr>
							)}
						</For>
					</tbody>
				</table>
			</Show>
			<Show when={hint()}>
				<p class="callout-info">{hint()}</p>
			</Show>
			<div class="buttons">
				<button type="button" onClick={add}>
					Add screen copy
				</button>
			</div>
		</>
	);
}
