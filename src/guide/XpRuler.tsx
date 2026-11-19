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
 * One cell per character level around the zone, coloured by the experience
 * that level gets here; the character's own level stands out.
 */
export default function XpRuler(props: { xp: XpView }) {
	const verdict = () => {
		const { playerLevel: lvl, multiplier: m, fullFrom, fullTo } = props.xp;
		if (lvl == null || m == null) return null;
		if (m >= 0.9999) return `You're level ${lvl}: full XP.`;
		const side = lvl < fullFrom ? "under" : "over";
		const by = lvl < fullFrom ? fullFrom - lvl : lvl - fullTo;
		return `You're level ${lvl}: ${pct(m)} XP, ${by} ${by === 1 ? "level" : "levels"} ${side}.`;
	};

	return (
		<section class="xp" aria-label={`Experience in ${props.xp.areaName}`}>
			<div class="xp-head">
				<span>
					{props.xp.upcoming ? "Next zone: " : ""}
					{props.xp.areaName}
				</span>
				<span class="muted">monsters lvl {props.xp.monsterLevel}</span>
			</div>
			<div class="xp-ruler">
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
			</div>
			<div class="xp-text">
				Full XP at levels {props.xp.fullFrom}–{props.xp.fullTo}.
				<Show when={verdict()}>
					{(v) => (
						<span classList={{ "xp-off": (props.xp.multiplier ?? 1) < 0.9999 }}>
							{" "}
							{v()}
						</span>
					)}
				</Show>
			</div>
		</section>
	);
}
