import { For, Show } from "solid-js";
import type { Fragment, GemPick, GuideStep } from "../types";

/** One Guide Step: the Area, then its instructions with notes. */
export default function StepView(props: {
	step: GuideStep;
	compact?: boolean;
}) {
	return (
		<div class="step" classList={{ compact: props.compact }}>
			<div class="step-area">
				<span class="step-act">A{props.step.act}</span> {props.step.areaName}
			</div>
			<ul class="step-lines">
				<For
					each={props.compact ? props.step.lines.slice(0, 2) : props.step.lines}
				>
					{(line) => (
						<li>
							<Parts parts={line.parts} />
							<Show when={!props.compact && line.notes.length > 0}>
								<ul class="step-notes">
									<For each={line.notes}>
										{(note) => (
											<li>
												<Parts parts={note} />
											</li>
										)}
									</For>
								</ul>
							</Show>
						</li>
					)}
				</For>
				<Show when={props.compact && props.step.lines.length > 2}>
					<li class="more">…</li>
				</Show>
			</ul>
		</div>
	);
}

function Parts(props: { parts: Fragment[] }) {
	return <For each={props.parts}>{(part) => <Part part={part} />}</For>;
}

function Part(props: { part: Fragment }) {
	const p = props.part;
	switch (p.kind) {
		case "text":
			return <>{p.text}</>;
		case "enter":
			return <span class="t-area">{p.name}</span>;
		case "area":
			return <span class="t-area">{p.name}</span>;
		case "waypoint":
			return p.name ? (
				<span class="t-waypoint">
					Waypoint ➞ <span class="t-area">{p.name}</span>
				</span>
			) : (
				<span class="t-waypoint">waypoint</span>
			);
		case "waypointGet":
			return <span class="t-waypoint">Get the waypoint</span>;
		case "logout":
			return <span class="t-logout">Log out</span>;
		case "portalSet":
			return <span class="t-waypoint">portal</span>;
		case "portalUse":
			return <span class="t-waypoint">the portal</span>;
		case "quest":
			return (
				<>
					<span class="t-quest">{p.name}</span>
					{p.npc && (
						<>
							{" "}
							to <span class="t-npc">{p.npc}</span>
						</>
					)}
					{p.book === "skill" && (
						<span class="t-book" title="Book of Skill">
							+1 passive
						</span>
					)}
					{p.book === "regrets" && (
						<span
							class="t-book regrets"
							title="Book of Regrets: passive refund points"
						>
							respec
						</span>
					)}
					{(p.take?.length ?? 0) > 0 && (
						<span class="gem-line">
							take <Gems gems={p.take ?? []} />
						</span>
					)}
					{(p.buy?.length ?? 0) > 0 && (
						<span class="gem-line">
							buy <Gems gems={p.buy ?? []} showNpc />
						</span>
					)}
				</>
			);
		case "questItem":
			return <span class="t-item">{p.text}</span>;
		case "kill":
			return <span class="t-kill">{p.name}</span>;
		case "arena":
			return <span class="t-area">{p.name}</span>;
		case "trial":
			return <span class="t-trial">the Trial of Ascendancy</span>;
		case "generic":
			return <span class="t-generic">{p.text}</span>;
		case "crafting":
			return <span class="t-generic">the crafting recipe</span>;
		case "ascend":
			return <span class="t-trial">Ascend ({p.difficulty})</span>;
		case "dir":
			return (
				<span class="t-dir" title={`${p.degrees}°`}>
					<svg
						viewBox="0 0 24 24"
						aria-hidden="true"
						style={{ transform: `rotate(${p.degrees}deg)` }}
					>
						<path d="M12 3 L19 12 H14.5 V21 H9.5 V12 H5 Z" />
					</svg>
				</span>
			);
	}
}

/** Gems as coloured chips; purchases grouped by who sells them. */
function Gems(props: { gems: GemPick[]; showNpc?: boolean }) {
	const byNpc = () => {
		const groups = new Map<string, GemPick[]>();
		for (const g of props.gems) {
			const key = props.showNpc ? (g.npc ?? "") : "";
			groups.set(key, [...(groups.get(key) ?? []), g]);
		}
		return [...groups.entries()];
	};
	return (
		<For each={byNpc()}>
			{([npc, gems]) => (
				<>
					<For each={gems}>
						{(g) => <span class={`gem gem-${g.colour}`}>{g.name}</span>}
					</For>
					{npc && (
						<>
							{" "}
							from <span class="t-npc">{npc}</span>{" "}
						</>
					)}
				</>
			)}
		</For>
	);
}
