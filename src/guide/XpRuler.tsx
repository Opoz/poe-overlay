import { For, Show } from "solid-js";
import type { XpView } from "../types";

const FULL = [76, 195, 138];
const REDUCED = [224, 168, 75];
const HEAVY = [229, 83, 75];

/** Green at full XP, through amber to red as the penalty grows. */
function cellColour(m: number): string {
	const mix = (a: number[], b: number[], t: number) =>
		a.map((v, i) => Math.round(v + (b[i] - v) * t));
	const [r, g, b] =
		m >= 0.9999
			? FULL
			: m >= 0.5
				? mix(REDUCED, FULL, (m - 0.5) / 0.5)
				: mix(HEAVY, REDUCED, m / 0.5);
	return `rgb(${r} ${g} ${b} / ${m >= 0.9999 ? 1 : 0.35 + m * 0.6})`;
}

const pct = (m: number) => `${Math.round(m * 100)}%`;

/**
 * The zone's experience at a glance: the verdict for your level as the
 * headline, then one cell per character level (yours raised), with the
 * full-XP band marked underneath.
 */
export default function XpRuler(props: { xp: XpView }) {
	const full = () => (props.xp.multiplier ?? 1) >= 0.9999;
	const verdict = () => {
		const m = props.xp.multiplier;
		if (props.xp.playerLevel == null || m == null) return null;
		return full() ? "Full XP" : `${pct(m)} XP`;
	};
	const offBy = () => {
		const { playerLevel: lvl, fullFrom, fullTo } = props.xp;
		if (lvl == null || full()) return null;
		const by = lvl < fullFrom ? fullFrom - lvl : lvl - fullTo;
		return `${by} ${by === 1 ? "level" : "levels"} ${lvl < fullFrom ? "under" : "over"}`;
	};
	// Grid columns of the full-XP band, for its bracket under the cells.
	const band = () => {
		const levels = props.xp.cells.map(([l]) => l);
		const from = levels.indexOf(props.xp.fullFrom);
		const to = levels.indexOf(props.xp.fullTo);
		return from < 0 || to < 0 ? null : { from: from + 1, to: to + 2 };
	};

	return (
		<section class="xp" aria-label={`Experience in ${props.xp.areaName}`}>
			<div class="xp-head">
				<span class="xp-area">
					{props.xp.upcoming ? "Next: " : ""}
					{props.xp.areaName}
					<span class="muted"> area level {props.xp.monsterLevel}</span>
				</span>
				<Show when={verdict()}>
					{(v) => (
						<span class="xp-verdict" classList={{ "xp-off": !full() }}>
							{v()}
							<Show when={offBy()}>
								{(o) => <span class="xp-by"> {o()}</span>}
							</Show>
						</span>
					)}
				</Show>
			</div>
			<div
				class="xp-ruler"
				style={{
					"grid-template-columns": `repeat(${props.xp.cells.length}, 1fr)`,
				}}
			>
				<For each={props.xp.cells}>
					{([level, m]) => {
						const you = () => level === props.xp.playerLevel;
						return (
							<div
								class="xp-cell"
								classList={{ you: you() }}
								style={{ background: cellColour(m) }}
								title={`Level ${level}: ${pct(m)} XP`}
							>
								<Show when={you()}>
									<span class="xp-you">{level}</span>
								</Show>
							</div>
						);
					}}
				</For>
				<Show when={band()}>
					{(b) => (
						<div
							class="xp-band"
							style={{ "grid-column": `${b().from} / ${b().to}` }}
							title="Levels that get full XP here"
						>
							<span>{props.xp.fullFrom}</span>
							<span>full XP</span>
							<span>{props.xp.fullTo}</span>
						</div>
					)}
				</Show>
			</div>
		</section>
	);
}
