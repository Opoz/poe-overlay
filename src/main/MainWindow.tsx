import { invoke } from "@tauri-apps/api/core";
import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import { formatDuration, formatLocalDate } from "../format";
import type { Run, Settings, Status, TimerCommand } from "../types";
import { useTimerView } from "../useTimerView";
import CharactersCard from "./CharactersCard";

const command = (c: TimerCommand) => invoke("timer_command", { command: c });

export default function MainWindow() {
	const { view, realMs } = useTimerView();
	const [status, setStatus] = createSignal<Status | null>(null);
	const [settings, { mutate: setSettingsLocal }] = createResource(() =>
		invoke<Settings>("get_settings"),
	);
	const [history, { refetch: refetchHistory }] = createResource(() =>
		invoke<Run[]>("get_history"),
	);

	onEvent<Status>("status", setStatus);
	invoke<Status>("get_status").then(setStatus);

	// A finished run lands in history; refresh when the phase flips.
	let lastPhase: string | undefined;
	const phaseWatcher = setInterval(() => {
		const phase = view()?.phase?.phase;
		if (phase !== lastPhase) {
			lastPhase = phase;
			refetchHistory();
		}
	}, 1000);
	onCleanup(() => clearInterval(phaseWatcher));

	const save = async (patch: Partial<Settings>) => {
		const current = settings();
		if (!current) return;
		const next = { ...current, ...patch };
		setSettingsLocal(next);
		await invoke("set_settings", { settings: next });
	};

	const reset = async () => {
		if (
			confirm(
				"Abandon the current run? It stays in history but stops counting.",
			)
		) {
			await command("reset");
			refetchHistory();
		}
	};

	return (
		<main class="main">
			<h1>poe-overlay</h1>

			<section class="card">
				<h2>Game</h2>
				<p>
					<span class="dot" classList={{ on: status()?.gameRunning }} />
					{status()?.gameRunning
						? "Path of Exile is running"
						: "Path of Exile not detected"}
				</p>
				<p class="muted">
					Client log: {status()?.logPath ?? "waiting for the game…"}
				</p>
				<p class="muted">
					Run the game in windowed fullscreen. Overlays show while the game is
					focused; Alt+E unlocks them for moving.
				</p>
			</section>

			<section class="card">
				<h2>Current run</h2>
				<Show
					when={view()?.phase}
					fallback={<p class="muted">No run in progress.</p>}
				>
					<p>
						<strong>{view()?.character ?? "New character"}</strong>{" "}
						{view()?.class ?? ""} · Act {view()?.currentAct ?? "—"} ·{" "}
						{formatDuration(realMs())}
					</p>
				</Show>
				<div class="buttons">
					<button type="button" onClick={() => command("start")}>
						Start
					</button>
					<button type="button" onClick={() => command("togglePause")}>
						Pause / resume <kbd>Alt+P</kbd>
					</button>
					<button type="button" onClick={() => command("split")}>
						Split <kbd>Alt+S</kbd>
					</button>
					<button type="button" onClick={() => command("undoSplit")}>
						Undo split <kbd>Alt+Shift+S</kbd>
					</button>
					<button type="button" class="danger" onClick={reset}>
						Reset
					</button>
				</div>
			</section>

			<section class="card">
				<h2>Passive tree overlay (preview)</h2>
				<p class="muted">
					With the overlay showing (Alt+T), zoom the in-game passive tree all
					the way out and middle-click your class start: your allocated passives
					are ringed on top. Middle-click again after panning. Alt+Shift+T is a
					fallback: left-click the class start, or press it twice for a
					two-click calibration that also measures the zoom.
				</p>
				<div class="buttons">
					<button
						type="button"
						onClick={() => invoke("toggle_tree", { calibrate: false })}
					>
						Show / hide <kbd>Alt+T</kbd>
					</button>
					<button
						type="button"
						onClick={() => invoke("toggle_tree", { calibrate: true })}
					>
						Calibrate <kbd>Alt+Shift+T</kbd>
					</button>
				</div>
			</section>

			<CharactersCard />

			<section class="card">
				<h2>Settings</h2>
				<Show when={settings()}>
					{(s) => (
						<div class="form">
							<label>
								Run category
								<input
									value={s().category}
									onChange={(e) =>
										save({
											category: e.currentTarget.value.trim() || "Default",
										})
									}
								/>
							</label>
							<label class="check">
								<input
									type="checkbox"
									checked={s().pauseInHideout}
									onChange={(e) =>
										save({ pauseInHideout: e.currentTarget.checked })
									}
								/>
								Pause the timer in hideouts
							</label>
							<label>
								Client log path (leave empty to detect automatically)
								<input
									value={s().logPath ?? ""}
									placeholder="…\Path of Exile\logs\Client.txt"
									onChange={(e) =>
										save({ logPath: e.currentTarget.value.trim() || null })
									}
								/>
							</label>
							<label class="check disabled" title="Coming later">
								<input type="checkbox" disabled />
								Path of Exile 2 (coming later)
							</label>
						</div>
					)}
				</Show>
			</section>

			<section class="card">
				<h2>Runs</h2>
				<Show
					when={(history() ?? []).length > 0}
					fallback={<p class="muted">No saved runs yet.</p>}
				>
					<table>
						<thead>
							<tr>
								<th>Started</th>
								<th>Character</th>
								<th>Category</th>
								<th>Reached</th>
								<th>Time</th>
							</tr>
						</thead>
						<tbody>
							<For each={[...(history() ?? [])].reverse()}>
								{(run) => <RunRow run={run} />}
							</For>
						</tbody>
					</table>
				</Show>
			</section>
		</main>
	);
}

function RunRow(props: { run: Run }) {
	const last = () => {
		const idx = props.run.splits.findLastIndex((s) => s !== null);
		const time = idx < 0 ? null : props.run.splits[idx];
		return time ? { act: idx + 1, time } : null;
	};
	const reached = () => {
		if (props.run.phase.phase === "finished") return "Finished";
		const l = last();
		return l ? `Act ${l.act + 1}` : "Act 1";
	};
	return (
		<tr>
			<td>{formatLocalDate(props.run.startedAt)}</td>
			<td>
				{props.run.character ?? "?"}{" "}
				<span class="muted">{props.run.class ?? ""}</span>
			</td>
			<td>{props.run.category ?? ""}</td>
			<td>{reached()}</td>
			<td>{last() ? formatDuration(last()?.time.realMs ?? 0) : "—"}</td>
		</tr>
	);
}
