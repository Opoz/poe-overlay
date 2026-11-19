import { invoke } from "@tauri-apps/api/core";
import { createResource, For, onCleanup, Show } from "solid-js";
import { formatLocalDate } from "../format";
import { RefreshIcon } from "../icons";
import type { BanditChoice, Characters, GuideOptions, Profile } from "../types";

/**
 * Profiles seen in the Client Log, and which one is assumed to be played.
 * The guide's options apply to the one being played, so they sit below.
 */
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
			alert(`Couldn't rebuild: ${e}`);
		}
		refetch();
	};

	const setGuide = async (name: string, options: GuideOptions) => {
		await invoke("set_guide_options", { name, options });
		refetch();
	};

	const sorted = () =>
		[...(characters()?.profiles ?? [])].sort((a, b) => b.lastSeen - a.lastSeen);
	const active = () => sorted().find((p) => p.name === characters()?.active);

	return (
		<section>
			<h2>Characters</h2>
			<p class="lede">
				The game log only names a character when it levels up, so after
				character select the last one seen is assumed. If that's wrong, pick the
				one you're playing here.
			</p>
			<Show
				when={sorted().length > 0}
				fallback={
					<p class="empty">
						No characters yet. Level up once and yours appears here.
					</p>
				}
			>
				<table class="characters">
					<thead>
						<tr>
							<th>Character</th>
							<th>Run</th>
							<th class="num">Passives</th>
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
								/>
							)}
						</For>
					</tbody>
				</table>

				<Show when={active()}>
					{(p) => (
						<GuideOptionsForm
							profile={p()}
							onChange={(o) => setGuide(p().name, o)}
						/>
					)}
				</Show>
			</Show>
		</section>
	);
}

function ProfileRow(props: {
	profile: Profile;
	active: boolean;
	onSelect: () => void;
	onRebuild: () => void;
}) {
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
				<div class="char-name">
					<strong>{props.profile.name}</strong>
					<span>
						Level {props.profile.level} {props.profile.class ?? ""}
					</span>
				</div>
			</td>
			<td>{runLabel()}</td>
			<td class="num">
				<span class="passives">
					{props.profile.passives.length}
					<button
						type="button"
						class="small"
						title="Rebuild this character's passives and masteries from the whole Client Log"
						aria-label="Rebuild passives from the log"
						onClick={props.onRebuild}
					>
						<RefreshIcon />
					</button>
				</span>
			</td>
			<td class="num">
				<Show
					when={!props.active}
					fallback={<span class="playing">Playing</span>}
				>
					<button type="button" onClick={props.onSelect}>
						Play this one
					</button>
				</Show>
			</td>
		</tr>
	);
}

function GuideOptionsForm(props: {
	profile: Profile;
	onChange: (o: GuideOptions) => void;
}) {
	const options = () => props.profile.guideOptions;
	return (
		<>
			<h3>Guide for {props.profile.name}</h3>
			<div class="fields">
				<div class="label">League start</div>
				<div>
					<label class="check">
						<input
							type="checkbox"
							checked={options().leagueStart}
							onChange={(e) =>
								props.onChange({
									...options(),
									leagueStart: e.currentTarget.checked,
								})
							}
						/>
						Include league-start steps
					</label>
					<div class="help">
						Early waypoints, side bosses and strongboxes worth doing on a fresh
						character.
					</div>
				</div>
				<div class="label">Bandits</div>
				<div>
					<select
						aria-label="Bandit choice"
						value={options().bandit}
						onChange={(e) =>
							props.onChange({
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
					<div class="help">
						Importing a build sets this from the build, unless you've picked one
						here.
					</div>
				</div>
			</div>
		</>
	);
}
