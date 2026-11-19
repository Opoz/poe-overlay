import { invoke } from "@tauri-apps/api/core";
import { createMemo, createSignal, For, Index, Show } from "solid-js";
import { formatDelta, formatDuration, formatLocalDate } from "../format";
import type { HistoryRun, HistoryView } from "../types";

/**
 * Past Runs, one bar each: the bar is the run's length, split into acts.
 * Fastest-ever acts are gold. Click a run for its splits against the
 * Personal Best of its category.
 */
export default function HistoryPage(props: {
	view: HistoryView | undefined;
	currentCategory: string;
	onChange: (v: HistoryView) => void;
}) {
	const categories = () => props.view?.categories ?? [];
	const [picked, setPicked] = createSignal<string | null>(null);
	const category = () => {
		const p = picked();
		if (p !== null) return p;
		return categories().includes(props.currentCategory)
			? props.currentCategory
			: (categories()[0] ?? "");
	};
	const runs = createMemo(() =>
		(props.view?.runs ?? []).filter((r) => r.category === category()),
	);
	// Bars share one time scale: the longest run shown fills the width.
	const longest = createMemo(() =>
		Math.max(1, ...runs().map((r) => r.timeMs ?? 0)),
	);
	const [open, setOpen] = createSignal<number | null>(null);

	const remove = async (run: HistoryRun) => {
		if (
			!confirm(
				`Delete ${run.character ?? "this"}'s run from ${formatLocalDate(run.startedAt)}? This can't be undone.`,
			)
		)
			return;
		props.onChange(
			await invoke<HistoryView>("delete_run", { startedAt: run.startedAt }),
		);
		setOpen(null);
	};

	return (
		<section class="history">
			<h2>History</h2>
			<p class="lede">
				Each bar is one run, split into its ten acts on the same time scale.
				Gold acts are your fastest ever in this category. Click a run for its
				splits.
			</p>
			<Show
				when={runs().length > 0}
				fallback={
					<p class="empty">Runs land here when they finish or are abandoned.</p>
				}
			>
				<Show when={categories().length > 1}>
					<div class="segmented" role="tablist" aria-label="Run category">
						<For each={categories()}>
							{(c) => (
								<button
									type="button"
									role="tab"
									aria-selected={c === category()}
									onClick={() => {
										setPicked(c);
										setOpen(null);
									}}
								>
									{c}
								</button>
							)}
						</For>
					</div>
				</Show>
				<ol class="runs">
					<For each={runs()}>
						{(run) => (
							<li
								classList={{
									pb: run.personalBest,
									open: open() === run.startedAt,
								}}
							>
								<button
									type="button"
									class="run-row"
									aria-expanded={open() === run.startedAt}
									onClick={() =>
										setOpen(open() === run.startedAt ? null : run.startedAt)
									}
								>
									<span class="run-who">
										<strong>{run.character ?? "Unknown"}</strong>
										<span>
											{run.class ?? ""}{" "}
											{formatLocalDate(run.startedAt).slice(0, 10)}
										</span>
									</span>
									<span class="run-bar" aria-hidden="true">
										<Index each={run.segments}>
											{(seg, i) => (
												<Show when={seg()}>
													{(ms) => (
														<span
															class="seg"
															classList={{ best: run.bestSegments[i] }}
															style={{
																width: `${(ms() / longest()) * 100}%`,
															}}
															title={`Act ${i + 1}: ${formatDuration(ms())}${run.bestSegments[i] ? " (fastest)" : ""}`}
														/>
													)}
												</Show>
											)}
										</Index>
									</span>
									<span class="run-time">
										<strong>
											{run.timeMs !== null ? formatDuration(run.timeMs) : "—"}
										</strong>
										<span>
											<Show
												when={run.finished}
												fallback={`Ended in act ${Math.min(10, run.actsDone + 1)}`}
											>
												{run.personalBest ? "Personal best" : "Finished"}
											</Show>
										</span>
									</span>
								</button>
								<Show when={open() === run.startedAt}>
									<RunDetail run={run} onDelete={() => remove(run)} />
								</Show>
							</li>
						)}
					</For>
				</ol>
			</Show>
		</section>
	);
}

function RunDetail(props: { run: HistoryRun; onDelete: () => void }) {
	let total = 0;
	const rows = props.run.segments.map((seg, i) => {
		if (seg !== null) total += seg;
		return {
			act: i + 1,
			seg,
			split: seg !== null ? total : null,
			delta: props.run.deltas[i],
			best: props.run.bestSegments[i],
		};
	});
	return (
		<div class="run-detail">
			<table>
				<thead>
					<tr>
						<th>Act</th>
						<th class="num">Act time</th>
						<th class="num">Split</th>
						<th class="num">Versus best run</th>
					</tr>
				</thead>
				<tbody>
					<For each={rows.filter((r) => r.seg !== null)}>
						{(r) => (
							<tr>
								<td>Act {r.act}</td>
								<td class="num" classList={{ "best-text": r.best }}>
									{formatDuration(r.seg ?? 0)}
									{r.best ? " fastest" : ""}
								</td>
								<td class="num">{formatDuration(r.split ?? 0)}</td>
								<td
									class="num delta"
									classList={{
										ahead: (r.delta ?? 0) < 0,
										behind: (r.delta ?? 0) > 0,
									}}
								>
									{props.run.personalBest
										? ""
										: r.delta !== null
											? formatDelta(r.delta)
											: ""}
								</td>
							</tr>
						)}
					</For>
				</tbody>
			</table>
			<div class="buttons">
				<button type="button" class="danger" onClick={props.onDelete}>
					Delete run
				</button>
			</div>
		</div>
	);
}
