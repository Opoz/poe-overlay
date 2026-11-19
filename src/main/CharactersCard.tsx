import { invoke } from "@tauri-apps/api/core";
import { createResource, For, onCleanup, Show } from "solid-js";
import { formatLocalDate } from "../format";
import type { BanditChoice, Characters, GuideOptions, Profile } from "../types";

/** Profiles seen in the Client Log, and which one is assumed to be played. */
export default function CharactersCard() {
	const [characters, { refetch }] = createResource(() =>
		invoke<Characters>("get_characters"),
	);
	const poll = setInterval(refetch, 2000);
	onCleanup(() => clearInterval(poll));

	const select = async (name: string) => {
		await invoke("select_character", { name });
		refetch();
	};

	const rebuild = async (name: string) => {
		try {
			const count = await invoke<number>("rebuild_passives", { name });
			alert(`${name}: ${count} passives found in the Client Log.`);
		} catch (e) {
			alert(`Could not rebuild: ${e}`);
		}
		refetch();
	};

	const setGuide = async (name: string, options: GuideOptions) => {
		await invoke("set_guide_options", { name, options });
		refetch();
	};

	const sorted = () =>
		[...(characters()?.profiles ?? [])].sort((a, b) => b.lastSeen - a.lastSeen);

	return (
		<section class="card">
			<h2>Characters</h2>
			<p class="muted">
				The log only names a character when it levels up, so after character
				select the last one is assumed. Pick the right one here if needed.
			</p>
			<Show
				when={sorted().length > 0}
				fallback={
					<p class="muted">
						No characters seen yet. Level up once to appear here.
					</p>
				}
			>
				<table class="characters">
					<thead>
						<tr>
							<th>Character</th>
							<th>Run</th>
							<th>Passives</th>
							<th>Guide</th>
							<th />
						</tr>
					</thead>
					<tbody>
						<For each={sorted()}>
							{(p) => (
								<ProfileRow
									profile={p}
									active={characters()?.active === p.name}
									onSelect={() => select(p.name)}
									onRebuild={() => rebuild(p.name)}
									onGuide={(options) => setGuide(p.name, options)}
								/>
							)}
						</For>
					</tbody>
				</table>
			</Show>
		</section>
	);
}

function ProfileRow(props: {
	profile: Profile;
	active: boolean;
	onSelect: () => void;
	onRebuild: () => void;
	onGuide: (options: GuideOptions) => void;
}) {
	const options = () => props.profile.guideOptions;
	const runLabel = () => {
		const run = props.profile.run;
		if (!run) return "—";
		if (run.phase.phase === "finished") return "Finished";
		const done = run.splits.filter((s) => s !== null).length;
		return `Act ${done + 1}`;
	};
	return (
		<tr classList={{ active: props.active }}>
			<td
				title={
					props.profile.lastSeen
						? `Last level up ${formatLocalDate(props.profile.lastSeen)}`
						: ""
				}
			>
				<strong>{props.profile.name}</strong>
				<div class="muted">
					{props.profile.class ?? "?"} · level {props.profile.level}
				</div>
			</td>
			<td>{runLabel()}</td>
			<td>
				<span class="passives">
					{props.profile.passives.length}
					<button
						type="button"
						class="small"
						title="Rebuild this character's passives and masteries from the whole Client Log"
						onClick={props.onRebuild}
					>
						↻
					</button>
				</span>
			</td>
			<td>
				<div class="guide-options">
					<label title="Include league-start steps (early waypoints, side bosses, strongboxes)">
						<input
							type="checkbox"
							checked={options().leagueStart}
							onChange={(e) =>
								props.onGuide({
									...options(),
									leagueStart: e.currentTarget.checked,
								})
							}
						/>
						League start
					</label>
					<select
						title="Bandit choice"
						value={options().bandit}
						onChange={(e) =>
							props.onGuide({
								...options(),
								bandit: e.currentTarget.value as BanditChoice,
							})
						}
					>
						<option value="killAll">Kill all bandits</option>
						<option value="alira">Help Alira</option>
						<option value="kraityn">Help Kraityn</option>
						<option value="oak">Help Oak</option>
					</select>
				</div>
			</td>
			<td>
				<Show
					when={!props.active}
					fallback={<span class="playing">Playing</span>}
				>
					<button type="button" onClick={props.onSelect}>
						Playing this
					</button>
				</Show>
			</td>
		</tr>
	);
}
