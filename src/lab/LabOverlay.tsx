import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { autoSizeWindow } from "../autoSize";
import { onEvent } from "../events";
import { titleCase } from "../format";
import { withKey } from "../hotkeys";
import { CloseIcon } from "../icons";
import Key from "../Key";
import type { LabImportStatus, LabPayload, LabRoom } from "../types";

const DOORS: Record<string, { words: string; angle: number }> = {
	N: { words: "top", angle: 0 },
	NE: { words: "top right", angle: 45 },
	E: { words: "right", angle: 90 },
	SE: { words: "bottom right", angle: 135 },
	S: { words: "bottom", angle: 180 },
	SW: { words: "bottom left", angle: 225 },
	W: { words: "left", angle: 270 },
	NW: { words: "top left", angle: 315 },
};

const W = 760;
const H = 420;
const PAD = 34;
/** Room for the marks under the lowest rooms. */
const MARKS_BELOW = 40;

/** The day's Labyrinth: rooms, where you are, and the way to Izaro. */
export default function LabOverlay() {
	const [view, setView] = createSignal<LabPayload | null>(null);
	onEvent<LabPayload>("lab-view", setView);
	invoke<LabPayload>("get_lab_view").then(setView);

	const [status, setStatus] = createSignal<LabImportStatus | null>(null);
	let hide: number | undefined;
	onEvent<LabImportStatus>("lab-import", (s) => {
		setStatus(s);
		clearTimeout(hide);
		if (s.state === "loaded" || s.state === "stopped" || s.state === "failed") {
			hide = window.setTimeout(() => setStatus(null), 6000);
		}
	});

	const [interact, setInteract] = createSignal(false);
	onEvent<boolean>("interact-mode", setInteract);
	invoke<boolean>("get_interact").then(setInteract);
	const startDrag = (e: MouseEvent) => {
		if (interact() && e.button === 0) getCurrentWindow().startDragging();
	};
	document.addEventListener("mousedown", startDrag);
	onCleanup(() => document.removeEventListener("mousedown", startDrag));

	const rooms = () => view()?.layout?.rooms ?? [];
	// Fit the layout's coordinates into the panel.
	// The map is as tall as the layout needs at the width's scale (no dead
	// space under wide, flat layouts).
	const project = () => {
		const r = rooms();
		const xs = r.map((x) => x.x);
		const ys = r.map((x) => x.y);
		const [minX, maxX] = [Math.min(...xs), Math.max(...xs)];
		const [minY, maxY] = [Math.min(...ys), Math.max(...ys)];
		const sx = (W - 2 * PAD) / Math.max(1, maxX - minX);
		const sy = (H - 2 * PAD) / Math.max(1, maxY - minY);
		const s = Math.min(sx, sy);
		// Centre horizontally when height limits the scale.
		const dx = (W - 2 * PAD - (maxX - minX) * s) / 2;
		return (room: LabRoom) => ({
			x: PAD + dx + (room.x - minX) * s,
			y: PAD + (room.y - minY) * s,
		});
	};
	const mapHeight = () => {
		const ys = rooms().map((x) => project()(x).y);
		return Math.ceil(Math.max(...ys) + MARKS_BELOW);
	};
	const routeSet = () => new Set(view()?.route ?? []);
	const visitedSet = () => new Set(view()?.visited ?? []);
	const edges = () => {
		const out: [number, number][] = [];
		rooms().forEach((r, i) => {
			for (const e of r.exits) out.push([i, e]);
		});
		return out;
	};
	const onRoute = (a: number, b: number) => {
		const route = [view()?.current ?? -1, ...(view()?.route ?? [])];
		return route.some(
			(x, i) =>
				(x === a && route[i + 1] === b) || (x === b && route[i + 1] === a),
		);
	};

	// The next room's door, as a direction you can see on the minimap.
	const nextDoor = () => {
		const v = view();
		const code = v?.nextDoor;
		const to = v?.route[0];
		if (!code || to == null || !v?.layout) return null;
		const d = DOORS[code];
		if (!d) return null;
		return { code, ...d, to: titleCase(v.layout.rooms[to]?.name ?? "") };
	};

	const title = () => {
		const l = view()?.layout;
		if (!l) return "Labyrinth";
		const kind = l.difficulty ? `${titleCase(l.difficulty)} lab` : "Lab";
		return l.date ? `${kind}, ${l.date}` : kind;
	};

	return (
		<div
			class="overlay lab"
			classList={{ interact: interact() }}
			ref={(el) => onMount(() => autoSizeWindow(el, { maxHeight: 900 }))}
		>
			<header class="guide-bar">
				<strong>{title()}</strong>
				<span class="guide-buttons">
					<button
						type="button"
						title="Open poelab and pick up the compass file you copy"
						onClick={() => invoke("lab_open")}
					>
						Open today's lab <Key id="lab.import" />
					</button>
					<button
						type="button"
						title="Load a layout already on the clipboard"
						onClick={() => invoke("lab_load_clipboard")}
					>
						Load copied
					</button>
					<button
						type="button"
						class="icon-button"
						title={withKey("Hide", "lab.toggle")}
						aria-label="Hide"
						onClick={() => invoke("lab_toggle")}
					>
						<CloseIcon />
					</button>
				</span>
			</header>

			<Show when={status()}>
				{(s) => <div class="toast">{s().message}</div>}
			</Show>
			<Show when={nextDoor()}>
				{(d) => (
					<div class="lab-door">
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							style={{ transform: `rotate(${d().angle}deg)` }}
						>
							<path d="M12 2 L20 12 H15 V22 H9 V12 H4 Z" />
						</svg>
						<span>
							Next door: <strong>{d().words}</strong> ({d().code}) to {d().to}
						</span>
					</div>
				)}
			</Show>
			<Show when={!nextDoor() && (view()?.routeDoors.length ?? 0) > 0}>
				<div class="lab-doors">
					<span class="muted">
						{view()?.current == null
							? "Doors from the entrance:"
							: "Doors on the way:"}
					</span>
					<For each={view()?.routeDoors ?? []}>
						{(d) => {
							const info = d.door ? DOORS[d.door] : undefined;
							const to = () =>
								titleCase(view()?.layout?.rooms[d.to]?.name ?? "");
							return (
								<span class="lab-door-chip" title={`to ${to()}`}>
									<Show when={info} fallback="?">
										{(i) => (
											<>
												<svg
													viewBox="0 0 24 24"
													aria-hidden="true"
													style={{ transform: `rotate(${i().angle}deg)` }}
												>
													<path d="M12 2 L20 12 H15 V22 H9 V12 H4 Z" />
												</svg>
												{i().words}
											</>
										)}
									</Show>
								</span>
							);
						}}
					</For>
				</div>
			</Show>
			<Show when={view()?.outdated}>
				<div class="toast warn">
					This layout is for {view()?.layout?.date}, not today. Load today's.
				</div>
			</Show>
			<Show when={view()?.wrongDifficulty}>
				<div class="toast warn">
					This layout is {view()?.layout?.difficulty}, but you're in the{" "}
					{view()?.running} lab.
				</div>
			</Show>

			<Show
				when={rooms().length > 0}
				fallback={
					<p class="muted">
						No layout loaded. Click "Open today's lab", open the lab on poelab
						and copy its compass file link. When the file opens, copy its text:
						in Firefox use Raw Data → Copy; elsewhere Ctrl+A, Ctrl+C.
					</p>
				}
			>
				<svg
					class="lab-map"
					viewBox={`0 0 ${W} ${mapHeight()}`}
					width={W}
					height={mapHeight()}
					role="img"
					aria-label="Labyrinth map"
				>
					<For each={edges()}>
						{([a, b]) => {
							const p = project();
							const [pa, pb] = [p(rooms()[a]), p(rooms()[b])];
							return (
								<line
									x1={pa.x}
									y1={pa.y}
									x2={pb.x}
									y2={pb.y}
									class="lab-edge"
									classList={{ route: onRoute(a, b) }}
								/>
							);
						}}
					</For>
					<For each={rooms()}>
						{(room, i) => {
							const pos = () => project()(room);
							return (
								<g
									class="lab-room"
									classList={{
										current: view()?.current === i(),
										visited: visitedSet().has(i()),
										route: routeSet().has(i()),
										izaro: room.izaro,
									}}
								>
									<title>
										{titleCase(room.name)}
										{room.contents.raw.length
											? ` — ${room.contents.raw.join(", ")}`
											: ""}
									</title>
									<rect
										x={pos().x - 13}
										y={pos().y - 13}
										width="26"
										height="26"
										rx="3"
									/>
									<text x={pos().x} y={pos().y - 19} class="lab-name">
										{room.izaro
											? "Izaro"
											: titleCase(room.name.split(" ").pop() ?? "")}
									</text>
									<Marks
										x={pos().x}
										y={pos().y + 30}
										contents={room.contents}
									/>
								</g>
							);
						}}
					</For>
				</svg>
				<div class="lab-footer">
					<label>
						<input
							type="checkbox"
							checked={view()?.darkshrineDetour ?? false}
							onChange={(e) =>
								invoke("set_lab_detour", { on: e.currentTarget.checked })
							}
						/>
						Darkshrine detour
					</label>
					<span class="lab-legend" title="Map key">
						<span>
							<b>D</b> darkshrine
						</span>
						<span>
							<svg class="lab-lock gold" viewBox="0 0 12 12" aria-hidden="true">
								<KeyGlyph />
							</svg>
							<svg class="lab-lock gold" viewBox="0 0 12 12" aria-hidden="true">
								<DoorGlyph />
							</svg>{" "}
							golden key, door
						</span>
						<span>
							<svg
								class="lab-lock silver"
								viewBox="0 0 12 12"
								aria-hidden="true"
							>
								<KeyGlyph />
							</svg>
							<svg
								class="lab-lock silver"
								viewBox="0 0 12 12"
								aria-hidden="true"
							>
								<DoorGlyph />
							</svg>{" "}
							silver key, door
						</span>
						<span>
							<b>A</b> Argus
						</span>
						<span>
							<b>G</b> gauntlet
						</span>
						<span>
							<b>P</b> puzzle
						</span>
					</span>
				</div>
			</Show>
		</div>
	);
}

type Mark = { kind: "letter" | "key" | "door"; text?: string; tone: string };

/** What's in a room, under it: letters, plus drawn keys and locked doors in
 * silver or gold. */
function Marks(props: { x: number; y: number; contents: LabRoom["contents"] }) {
	const marks = (): Mark[] => {
		const c = props.contents;
		const out: Mark[] = [];
		if (c.darkshrine) out.push({ kind: "letter", text: "D", tone: "plain" });
		if (c.goldKey) out.push({ kind: "key", tone: "gold" });
		if (c.goldDoor) out.push({ kind: "door", tone: "gold" });
		if (c.silverKey) out.push({ kind: "key", tone: "silver" });
		if (c.silverDoor) out.push({ kind: "door", tone: "silver" });
		if (c.argus) out.push({ kind: "letter", text: "A", tone: "plain" });
		if (c.gauntlet) out.push({ kind: "letter", text: "G", tone: "plain" });
		if (c.puzzle) out.push({ kind: "letter", text: "P", tone: "plain" });
		return out;
	};
	const STEP = 13;
	return (
		<For each={marks()}>
			{(m, i) => {
				const cx = () => props.x + (i() - (marks().length - 1) / 2) * STEP;
				return m.kind === "letter" ? (
					<text x={cx()} y={props.y} class="lab-marks">
						{m.text}
					</text>
				) : (
					<g
						class={`lab-lock ${m.tone}`}
						transform={`translate(${cx() - 6} ${props.y - 11})`}
					>
						<title>{`${m.tone === "gold" ? "Golden" : "Silver"} ${m.kind}`}</title>
						{m.kind === "key" ? <KeyGlyph /> : <DoorGlyph />}
					</g>
				);
			}}
		</For>
	);
}

/** A key, 12x12. */
const KeyGlyph = () => (
	<>
		<circle cx="3.6" cy="6" r="2.6" />
		<path d="M6.2 6h5M9.4 6v2.4M11 6v1.8" />
	</>
);

/** A padlock, for a locked door, 12x12. */
const DoorGlyph = () => (
	<>
		<rect x="2" y="5.4" width="8" height="6" rx="1" class="fill" />
		<path d="M3.8 5.4V3.8a2.2 2.2 0 0 1 4.4 0v1.6" />
	</>
);
