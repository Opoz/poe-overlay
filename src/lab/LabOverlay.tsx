import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, For, onCleanup, Show } from "solid-js";
import { onEvent } from "../events";
import type { LabImportStatus, LabPayload, LabRoom } from "../types";

const W = 540;
const H = 300;
const PAD = 22;

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
	const project = () => {
		const r = rooms();
		const xs = r.map((x) => x.x);
		const ys = r.map((x) => x.y);
		const [minX, maxX] = [Math.min(...xs), Math.max(...xs)];
		const [minY, maxY] = [Math.min(...ys), Math.max(...ys)];
		const sx = (W - 2 * PAD) / Math.max(1, maxX - minX);
		const sy = (H - 2 * PAD) / Math.max(1, maxY - minY);
		const s = Math.min(sx, sy);
		return (room: LabRoom) => ({
			x: PAD + (room.x - minX) * s,
			y: PAD + (room.y - minY) * s,
		});
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

	const title = () => {
		const l = view()?.layout;
		if (!l) return "Labyrinth";
		return `${l.difficulty ?? "Lab"} · ${l.date ?? "?"}`;
	};

	return (
		<div class="overlay lab" classList={{ interact: interact() }}>
			<header class="guide-bar">
				<strong>{title()}</strong>
				<span class="guide-buttons">
					<button
						type="button"
						title="Open poelab and pick up the compass file you copy"
						onClick={() => invoke("lab_open")}
					>
						Open today's lab (Alt+Shift+B)
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
						title="Hide (Alt+B)"
						onClick={() => invoke("lab_toggle")}
					>
						✕
					</button>
				</span>
			</header>

			<Show when={status()}>
				{(s) => <div class="toast">{s().message}</div>}
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
						and copy its compass file link; when it opens, copy the text
						(Ctrl+A, Ctrl+C).
					</p>
				}
			>
				<svg
					class="lab-map"
					viewBox={`0 0 ${W} ${H}`}
					width={W}
					height={H}
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
										x={pos().x - 9}
										y={pos().y - 9}
										width="18"
										height="18"
										rx="3"
									/>
									<text x={pos().x} y={pos().y - 13} class="lab-name">
										{room.izaro
											? "Izaro"
											: titleCase(room.name.split(" ").pop() ?? "")}
									</text>
									<text x={pos().x} y={pos().y + 22} class="lab-marks">
										{room.contents.darkshrine ? "D" : ""}
										{room.contents.silverKey ? "K" : ""}
										{room.contents.silverDoor ? "🔒" : ""}
										{room.contents.argus ? "A" : ""}
										{room.contents.gauntlet ? "G" : ""}
										{room.contents.puzzle ? "P" : ""}
									</text>
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
					<span class="muted">
						D darkshrine · K key · 🔒 door · A Argus · G gauntlet · P puzzle
					</span>
				</div>
			</Show>
		</div>
	);
}

/** poelab writes room names in lowercase ("estate path"). */
function titleCase(s: string): string {
	return s.replace(/\b\w/g, (c) => c.toUpperCase());
}
