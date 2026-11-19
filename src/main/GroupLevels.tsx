import { invoke } from "@tauri-apps/api/core";
import { For, Show } from "solid-js";
import type { LevelRange, Profile } from "../types";

/**
 * Which character levels each Skill Group is for. Read from PoB labels when
 * possible; set by hand here otherwise.
 */
export default function GroupLevels(props: {
	profile: Profile;
	onChange: () => void;
}) {
	const groups = () => props.profile.build?.skillGroups ?? [];

	const set = async (
		index: number,
		levels: LevelRange | null,
		reset = false,
	) => {
		await invoke("set_group_levels", {
			name: props.profile.name,
			index,
			levels,
			reset,
		});
		props.onChange();
	};

	const effective = (index: number): LevelRange | null => {
		const key = String(index);
		if (key in props.profile.groupLevels) return props.profile.groupLevels[key];
		return groups()[index]?.levels ?? null;
	};

	const parse = (v: string) => {
		const n = Number.parseInt(v, 10);
		return Number.isFinite(n) && n > 0 ? n : null;
	};

	return (
		<details class="group-levels">
			<summary>Skill group levels ({groups().length} groups)</summary>
			<p class="muted">
				The gem links overlay shows groups whose range includes your level.
				Ranges come from PoB labels like "lvl 1-12" or "Act 2"; set them here
				when a label doesn't say.
			</p>
			<table>
				<thead>
					<tr>
						<th>Group</th>
						<th>From</th>
						<th>To</th>
						<th />
					</tr>
				</thead>
				<tbody>
					<For each={groups()}>
						{(g, i) => {
							const range = () => effective(i());
							const byHand = () => String(i()) in props.profile.groupLevels;
							return (
								<tr classList={{ disabled: !g.enabled }}>
									<td>
										{g.label ??
											g.gems
												.slice(0, 3)
												.map((x) => x.name)
												.join(", ")}
										<Show when={g.skillSet}>
											<span class="muted group-set">{g.skillSet}</span>
										</Show>
									</td>
									<td>
										<input
											class="level-input"
											inputmode="numeric"
											placeholder="any"
											value={range()?.from ?? ""}
											onChange={(e) => {
												const from = parse(e.currentTarget.value);
												set(
													i(),
													from ? { from, to: range()?.to ?? null } : null,
												);
											}}
										/>
									</td>
									<td>
										<input
											class="level-input"
											inputmode="numeric"
											placeholder="any"
											value={range()?.to ?? ""}
											onChange={(e) => {
												const to = parse(e.currentTarget.value);
												set(i(), { from: range()?.from ?? 1, to });
											}}
										/>
									</td>
									<td>
										<Show
											when={byHand()}
											fallback={<span class="muted">from label</span>}
										>
											<button
												type="button"
												onClick={() => set(i(), null, true)}
											>
												Use label
											</button>
										</Show>
									</td>
								</tr>
							);
						}}
					</For>
				</tbody>
			</table>
		</details>
	);
}
