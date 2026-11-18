import { invoke } from "@tauri-apps/api/core";
import { createSignal, For, Show } from "solid-js";
import { onEvent } from "../events";
import { formatDelta, formatDuration } from "../format";
import type { SplitRow } from "../types";
import { useTimerView } from "../useTimerView";

export default function TimerOverlay() {
	const { view, realMs, loadRemovedMs } = useTimerView();
	const [interact, setInteract] = createSignal(false);

	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);

	const phase = () => view()?.phase?.phase ?? "idle";
	const statusLabel = () => {
		const p = view()?.phase;
		if (!p) return "No run: enter The Twilight Strand";
		if (p.phase === "paused") return `Paused (${p.reason})`;
		if (p.phase === "finished") return "Finished";
		return view()?.currentAct ? `Act ${view()?.currentAct}` : "";
	};

	return (
		<div
			class="overlay timer"
			classList={{ interact: interact() }}
			data-phase={phase()}
			data-tauri-drag-region={interact() ? true : undefined}
		>
			<header data-tauri-drag-region={interact() ? true : undefined}>
				<span class="status">{statusLabel()}</span>
				<Show when={interact()}>
					<span class="hint">drag · Alt+E to lock</span>
				</Show>
			</header>
			<div class="clock">{formatDuration(realMs())}</div>
			<div class="clock-sub">
				{formatDuration(loadRemovedMs())} without loads
			</div>
			<ol class="splits">
				<For each={view()?.splits ?? []}>
					{(row) => (
						<SplitLine row={row} current={view()?.currentAct === row.act} />
					)}
				</For>
			</ol>
			<Show when={view()?.bestPossibleRealMs}>
				{(ms) => <div class="footer">Best possible {formatDuration(ms())}</div>}
			</Show>
		</div>
	);
}

function SplitLine(props: { row: SplitRow; current: boolean }) {
	const delta = () => props.row.deltaRealMs;
	const shown = () => {
		if (props.row.time) return formatDuration(props.row.time.realMs);
		if (props.row.pbRealMs !== null) return formatDuration(props.row.pbRealMs);
		return "—";
	};
	return (
		<li classList={{ current: props.current, done: props.row.time !== null }}>
			<span class="act">Act {props.row.act}</span>
			<span
				class="delta"
				classList={{
					ahead: (delta() ?? 0) < 0,
					behind: (delta() ?? 0) > 0,
					gold: props.row.bestSegment,
				}}
			>
				{delta() !== null ? formatDelta(delta() as number) : ""}
			</span>
			<span class="time">{shown()}</span>
		</li>
	);
}
