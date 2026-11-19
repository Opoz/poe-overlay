import { invoke } from "@tauri-apps/api/core";
import { createSignal, For, onCleanup, Show } from "solid-js";
import { formatShortcut } from "../format";
import {
	type HotkeyRow,
	hotkeyRows,
	setHotkeyRows,
	shortcutFromEvent,
} from "../hotkeys";
import type { Settings } from "../types";

/** Rebind the global hotkeys; they only work while the game is focused. */
export default function ShortcutsPage(props: {
	settings: Settings;
	save: (patch: Partial<Settings>) => void;
}) {
	const [capturing, setCapturing] = createSignal<string | null>(null);
	const [error, setError] = createSignal<{ id: string; text: string } | null>(
		null,
	);

	const groups = () => {
		const out = new Map<string, HotkeyRow[]>();
		for (const r of hotkeyRows())
			out.set(r.group, [...(out.get(r.group) ?? []), r]);
		return [...out.entries()];
	};

	const bind = async (id: string | null, shortcut: string | null) => {
		try {
			setHotkeyRows(await invoke<HotkeyRow[]>("set_hotkey", { id, shortcut }));
			setError(null);
		} catch (e) {
			if (id) setError({ id, text: String(e) });
		}
	};

	const onKey = (e: KeyboardEvent) => {
		const id = capturing();
		if (!id) return;
		e.preventDefault();
		e.stopPropagation();
		if (e.code === "Escape") {
			setCapturing(null);
			return;
		}
		const s = shortcutFromEvent(e);
		if (s === null) return;
		if (s === "needs-modifier") {
			setError({
				id,
				text: "Hold Alt, Ctrl or Shift with the key (F-keys work alone), so typing in the game can't trigger it.",
			});
			return;
		}
		setCapturing(null);
		void bind(id, s);
	};
	window.addEventListener("keydown", onKey, true);
	onCleanup(() => window.removeEventListener("keydown", onKey, true));

	return (
		<section>
			<h2>Shortcuts</h2>
			<p class="lede">
				Shortcuts only work while the game is focused, so they never take keys
				from other apps. Click a shortcut, then press the new combination.
				Escape cancels.
			</p>
			<table class="shortcuts">
				<For each={groups()}>
					{([group, rows]) => (
						<tbody>
							<tr class="group">
								<th colspan="3">{group}</th>
							</tr>
							<For each={rows}>
								{(r) => (
									<tr>
										<td>
											{r.label}
											<Show when={error()?.id === r.id}>
												<div class="warn-text small-text">{error()?.text}</div>
											</Show>
										</td>
										<td class="binding">
											<button
												type="button"
												class="keycap-button"
												classList={{ capturing: capturing() === r.id }}
												onClick={() => {
													setError(null);
													setCapturing(capturing() === r.id ? null : r.id);
												}}
											>
												<Show
													when={capturing() !== r.id}
													fallback="Press keys…"
												>
													<Show
														when={r.shortcut}
														fallback={<span class="muted">Not set</span>}
													>
														<kbd>{formatShortcut(r.shortcut)}</kbd>
													</Show>
												</Show>
											</button>
										</td>
										<td class="row-actions">
											<Show when={r.shortcut}>
												<button
													type="button"
													class="small reveal"
													title="Turn this shortcut off"
													onClick={() => bind(r.id, null)}
												>
													Clear
												</button>
											</Show>
											<Show when={r.shortcut !== r.default}>
												<button
													type="button"
													class="small"
													title={`Back to ${formatShortcut(r.default)}`}
													onClick={() => bind(r.id, r.default)}
												>
													Default
												</button>
											</Show>
										</td>
									</tr>
								)}
							</For>
						</tbody>
					)}
				</For>
				<tbody>
					<tr class="group">
						<th colspan="3">Game keys</th>
					</tr>
					<tr>
						<td>
							Your passive tree key
							<div class="muted small-text">
								Pressing it shows the tree overlay too, and Escape hides it.
								Leave empty to turn this off.
							</div>
						</td>
						<td class="binding">
							<input
								class="level-input"
								maxLength={1}
								value={props.settings.treeKey ?? ""}
								aria-label="Passive tree key"
								onChange={(e) =>
									props.save({
										treeKey: e.currentTarget.value.trim().toUpperCase() || null,
									})
								}
							/>
						</td>
						<td />
					</tr>
				</tbody>
			</table>
			<div class="buttons">
				<button
					type="button"
					class="danger"
					onClick={() => {
						if (confirm("Put every shortcut back to its default?"))
							bind(null, null);
					}}
				>
					Reset all shortcuts
				</button>
			</div>
		</section>
	);
}
