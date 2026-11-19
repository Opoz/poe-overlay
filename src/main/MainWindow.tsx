import { invoke } from "@tauri-apps/api/core";
import {
	createResource,
	createSignal,
	For,
	type JSX,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import { onEvent } from "../events";
import { formatDelta, formatDuration, formatShortcut } from "../format";
import Key from "../Key";
import {
	type HistoryView,
	PAUSE_LABEL,
	type Settings,
	type Status,
	type TimerCommand,
} from "../types";
import { useTimerView } from "../useTimerView";
import BuildCard from "./BuildCard";
import CharactersCard from "./CharactersCard";
import HistoryPage from "./HistoryPage";
import Mark from "./Mark";
import OverlaysPage from "./OverlaysPage";
import ShortcutsPage from "./ShortcutsPage";

const command = (c: TimerCommand) => invoke("timer_command", { command: c });

const PAGES = [
	["run", "Run"],
	["characters", "Characters"],
	["build", "Build"],
	["overlays", "Overlays"],
	["shortcuts", "Shortcuts"],
	["history", "History"],
	["settings", "Settings"],
] as const;
type Page = (typeof PAGES)[number][0];

function initialPage(): Page {
	const fromUrl = new URLSearchParams(location.search).get("page");
	let saved: string | null = fromUrl;
	if (!saved) {
		try {
			saved = localStorage.getItem("page");
		} catch {
			saved = null;
		}
	}
	return PAGES.some(([id]) => id === saved) ? (saved as Page) : "run";
}

export default function MainWindow() {
	const live = useTimerView();
	const { view, realMs, loadRemovedMs } = live;
	const [page, setPage] = createSignal<Page>(initialPage());
	const go = (p: Page) => {
		setPage(p);
		try {
			localStorage.setItem("page", p);
		} catch {
			// Remembering the page is a convenience only.
		}
	};

	const [status, setStatus] = createSignal<Status | null>(null);
	const [settings, { mutate: setSettingsLocal }] = createResource(() =>
		invoke<Settings>("get_settings"),
	);
	const [history, { refetch: refetchHistory, mutate: setHistory }] =
		createResource(() => invoke<HistoryView>("get_history"));

	onEvent<Status>("status", setStatus);
	const [hotkeyFailures, setHotkeyFailures] = createSignal<string[]>([]);
	onEvent<string[]>("hotkey-failures", setHotkeyFailures);
	invoke<string[]>("hotkey_failures").then(setHotkeyFailures);
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

	const runLine = () => {
		const v = view();
		if (!v?.phase) return "No run yet: a new character's first area starts one";
		const parts = [v.class, v.currentAct ? `Act ${v.currentAct}` : null];
		if (v.phase.phase === "paused") parts.push(PAUSE_LABEL[v.phase.reason]);
		if (v.phase.phase === "finished") parts.push("finished");
		return parts.filter(Boolean).join(", ");
	};

	return (
		<div class="app">
			<nav class="nav" aria-label="Sections">
				<div class="brand">
					<Mark />
					poe-overlay
				</div>
				<For each={PAGES}>
					{([id, title]) => (
						<a
							href={`#${id}`}
							aria-current={page() === id ? "page" : undefined}
							onClick={(e) => {
								e.preventDefault();
								go(id);
							}}
						>
							{title}
						</a>
					)}
				</For>
				<div class="game-state">
					<span class="dot" classList={{ on: status()?.gameRunning }} />
					{status()?.gameRunning ? "Game running" : "Game not running"}
				</div>
			</nav>

			<div class="content">
				<header class="runstrip">
					<div class="who">
						<strong>{view()?.character ?? "No character yet"}</strong>
						<span>{runLine()}</span>
					</div>
					<div class="clock" data-phase={view()?.phase?.phase ?? "idle"}>
						{formatDuration(realMs())}
						<small>{formatDuration(loadRemovedMs())} without loads</small>
					</div>
				</header>

				<main class="page">
					<Show when={hotkeyFailures().length > 0}>
						<div class="callout">
							Another app has these shortcuts, so they won't work here:{" "}
							<strong>{hotkeyFailures().map(formatShortcut).join(", ")}</strong>
							. Free them in that app (the NVIDIA App takes Alt+R, for example).
						</div>
					</Show>
					<Switch>
						<Match when={page() === "run"}>
							<RunPage live={live} />
						</Match>
						<Match when={page() === "characters"}>
							<CharactersCard />
						</Match>
						<Match when={page() === "build"}>
							<BuildCard />
						</Match>
						<Match when={page() === "overlays"}>
							<Show when={settings()}>
								{(s) => (
									<OverlaysPage
										settings={s()}
										save={save}
										onSettings={setSettingsLocal}
									/>
								)}
							</Show>
						</Match>
						<Match when={page() === "shortcuts"}>
							<Show when={settings()}>
								{(s) => <ShortcutsPage settings={s()} save={save} />}
							</Show>
						</Match>
						<Match when={page() === "history"}>
							<HistoryPage
								view={history()}
								currentCategory={settings()?.category ?? ""}
								onChange={setHistory}
							/>
						</Match>
						<Match when={page() === "settings"}>
							<Show when={settings()}>
								{(s) => (
									<SettingsPage
										settings={s()}
										save={save}
										logPath={status()?.logPath ?? null}
									/>
								)}
							</Show>
						</Match>
					</Switch>
				</main>
			</div>
		</div>
	);
}

function RunPage(props: { live: ReturnType<typeof useTimerView> }) {
	const view = () => props.live.view();
	const reset = async () => {
		if (
			confirm(
				"Abandon the current run? It stays in history but stops counting.",
			)
		) {
			await command("reset");
		}
	};
	return (
		<section>
			<h2>Run</h2>
			<p class="lede">
				The timer starts in a new character's first area and splits as you enter
				each act. It pauses at character select, and in hideouts if you choose.
			</p>
			<div class="buttons">
				<button type="button" onClick={() => command("togglePause")}>
					{view()?.phase?.phase === "paused" ? "Resume" : "Pause"}{" "}
					<Key id="timer.pause" />
				</button>
				<button type="button" onClick={() => command("split")}>
					Split <Key id="timer.split" />
				</button>
				<button type="button" onClick={() => command("undoSplit")}>
					Undo split <Key id="timer.undoSplit" />
				</button>
				<button type="button" onClick={() => command("start")}>
					Start now
				</button>
				<button type="button" class="danger" onClick={reset}>
					Abandon run
				</button>
			</div>
			<Show when={(view()?.splits.length ?? 0) > 0}>
				<h3>Splits</h3>
				<table class="split-table">
					<thead>
						<tr>
							<th>Act</th>
							<th class="num">Versus best</th>
							<th class="num">Time</th>
						</tr>
					</thead>
					<tbody>
						<For each={view()?.splits ?? []}>
							{(row) => (
								<tr
									classList={{
										here: view()?.currentAct === row.act,
										pending: row.time === null,
									}}
								>
									<td>Act {row.act}</td>
									<td
										class="num delta"
										classList={{
											ahead: (row.deltaRealMs ?? 0) < 0,
											behind: (row.deltaRealMs ?? 0) > 0,
											gold: row.bestSegment,
										}}
									>
										{row.deltaRealMs !== null
											? formatDelta(row.deltaRealMs)
											: ""}
									</td>
									<td class="num">
										{row.time
											? formatDuration(row.time.realMs)
											: row.pbRealMs !== null
												? formatDuration(row.pbRealMs)
												: "—"}
									</td>
								</tr>
							)}
						</For>
					</tbody>
				</table>
				<Show when={view()?.bestPossibleRealMs}>
					{(ms) => (
						<p class="muted">
							Best possible finish from here: {formatDuration(ms())}. Times in
							grey are your best run's.
						</p>
					)}
				</Show>
			</Show>
		</section>
	);
}

function Field(props: { label: string; children: JSX.Element }) {
	return (
		<>
			<div class="label">{props.label}</div>
			<div>{props.children}</div>
		</>
	);
}

function SettingsPage(props: {
	settings: Settings;
	save: (patch: Partial<Settings>) => void;
	logPath: string | null;
}) {
	const s = () => props.settings;
	return (
		<section>
			<h2>Settings</h2>
			<p class="lede">Changes save as you make them.</p>
			<div class="fields">
				<Field label="Run category">
					<input
						value={s().category}
						onChange={(e) =>
							props.save({
								category: e.currentTarget.value.trim() || "Default",
							})
						}
					/>
					<div class="help">
						New runs are filed under this, and only compared with runs in the
						same category.
					</div>
				</Field>
				<Field label="Hideouts">
					<label class="check">
						<input
							type="checkbox"
							checked={s().pauseInHideout}
							onChange={(e) =>
								props.save({ pauseInHideout: e.currentTarget.checked })
							}
						/>
						Pause the timer in hideouts
					</label>
				</Field>
				<Field label="Client log">
					<input
						value={s().logPath ?? ""}
						placeholder={props.logPath ?? "Found automatically"}
						onChange={(e) =>
							props.save({ logPath: e.currentTarget.value.trim() || null })
						}
					/>
					<div class="help">
						Leave empty to find it from the running game. Reading:{" "}
						{props.logPath ?? "waiting for the game"}.
					</div>
				</Field>
				<Field label="Game">
					<label class="check disabled" title="Coming later">
						<input type="checkbox" checked disabled />
						Path of Exile 1 (Path of Exile 2 later)
					</label>
				</Field>
			</div>
		</section>
	);
}
