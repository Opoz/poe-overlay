import { invoke } from "@tauri-apps/api/core";
import {
	createEffect,
	createResource,
	createSignal,
	onCleanup,
	Show,
} from "solid-js";
import { onEvent } from "../events";
import type { TreeNode, TreeView } from "../types";

const RADIUS: Record<TreeNode["kind"], number> = {
	normal: 26,
	notable: 38,
	keystone: 54,
	mastery: 30,
	jewel: 34,
	classStart: 60,
	ascendancyStart: 30,
};

/**
 * Covers the game window. Once calibrated, rings every allocated passive at
 * its position on the in-game tree, so misalignment is easy to spot.
 */
export default function TreeOverlay() {
	const [nodes] = createResource(() => invoke<TreeNode[]>("get_tree"));
	const [view, setView] = createSignal<TreeView | null>(null);
	onEvent<TreeView>("tree-view", setView);
	invoke<TreeView>("get_tree_view").then(setView);

	let canvas: HTMLCanvasElement | undefined;
	const [size, setSize] = createSignal({
		w: window.innerWidth,
		h: window.innerHeight,
	});
	const onResize = () =>
		setSize({ w: window.innerWidth, h: window.innerHeight });
	window.addEventListener("resize", onResize);
	onCleanup(() => window.removeEventListener("resize", onResize));

	// A slow "breathing" glow on Next Points: redraw ~30 times a second while
	// there are any to show.
	// (A timer rather than requestAnimationFrame: the WebView may pause
	// animation frames in a transparent overlay window.)
	const [pulse, setPulse] = createSignal(0);
	const pulseTimer = setInterval(() => {
		const v = view();
		if (
			v?.visible &&
			v.transform &&
			(v.planned.length > 0 || v.plannedMasteries.length > 0)
		) {
			setPulse((Math.sin(performance.now() / 450) + 1) / 2);
		}
	}, 50);
	onCleanup(() => clearInterval(pulseTimer));

	createEffect(() => draw(canvas, size(), nodes(), view(), pulse()));

	// Clicks are only delivered while calibrating or fixing points
	// (otherwise the overlay lets them through to the game).
	const onClick = (e: MouseEvent) => {
		if (!view()?.calibrating && !view()?.editing) return;
		// Buttons on the overlay (Done fixing) aren't clicks on the tree.
		if ((e.target as HTMLElement | null)?.closest("button")) return;
		const dpr = window.devicePixelRatio;
		invoke("tree_click", { x: e.clientX * dpr, y: e.clientY * dpr });
	};
	document.addEventListener("mousedown", onClick);
	onCleanup(() => document.removeEventListener("mousedown", onClick));

	const prompt = () => {
		const v = view();
		if (!v?.calibrating) return null;
		if (!v.classStart)
			return "Level up once so the class is known, then calibrate again.";
		if (v.ascendancy) {
			return `Click the centre of your ascendancy start (${v.ascendancyName ?? "ascendancy"}), in the circle under your class art. Alt+Shift+T cancels.`;
		}
		const start = className(v.classStart.name);
		if (v.calibrating.step === "quick") {
			return `Zoom the passive tree all the way out and click the centre of your class start (${start}). Alt+Shift+T again for full two-click calibration.`;
		}
		if (v.calibrating.step === "classStart") {
			return `Zoom the passive tree all the way out, then click the centre of your class start (${start}).`;
		}
		return `Now click the centre of the ${className(v.partner?.name ?? "")} start${v.partner?.classStart === 0 ? " in the middle of the tree" : " at the top"}.`;
	};

	return (
		<div
			class="tree-overlay"
			classList={{ calibrating: !!view()?.calibrating || !!view()?.editing }}
		>
			<canvas ref={canvas} />
			<Show when={view()?.editing && !view()?.calibrating}>
				<div class="tree-prompt">
					<strong>Fixing points</strong> Click a node to add or remove it (for
					points you cancelled in game, which the log can't see).{" "}
					<button type="button" onClick={() => invoke("toggle_tree_edit")}>
						Done fixing
					</button>
				</div>
			</Show>
			<Show when={prompt()}>
				{(text) => (
					<div class="tree-prompt">
						<strong>Calibrating</strong> {text()}
						<Show when={view()?.calibrating?.step !== "quick"}>
							<span class="muted"> Alt+Shift+T cancels.</span>
						</Show>
					</div>
				)}
			</Show>
			<Show
				when={
					view()?.ascendancy &&
					!view()?.ascendancyTransform &&
					!view()?.calibrating
				}
			>
				<div class="tree-prompt">
					<strong>Ascendancy</strong> Align the main tree first: zoom out and
					middle-click your class start. The ascendancy follows it.
				</div>
			</Show>
			<Show
				when={
					view() &&
					!view()?.ascendancy &&
					!view()?.calibrating &&
					!view()?.transform
				}
			>
				<div class="tree-prompt">
					Zoom the passive tree all the way out and middle-click your class
					start to align.
				</div>
			</Show>
		</div>
	);
}

/** GGG's export names class starts oddly ("Seven", "SIX"); use the class. */
const START_NAMES: Record<string, string> = { SEVEN: "Scion", SIX: "Shadow" };
function className(raw: string): string {
	const upper = raw.toUpperCase();
	if (START_NAMES[upper]) return START_NAMES[upper];
	return upper.charAt(0) + upper.slice(1).toLowerCase();
}

function draw(
	canvas: HTMLCanvasElement | undefined,
	size: { w: number; h: number },
	nodes: TreeNode[] | undefined,
	view: TreeView | null,
	pulse: number,
) {
	if (!canvas) return;
	const dpr = window.devicePixelRatio;
	canvas.width = Math.round(size.w * dpr);
	canvas.height = Math.round(size.h * dpr);
	canvas.style.width = `${size.w}px`;
	canvas.style.height = `${size.h}px`;
	const ctx = canvas.getContext("2d");
	if (!ctx) return;
	ctx.clearRect(0, 0, canvas.width, canvas.height);

	if (!nodes || !view || view.calibrating) return;
	if (view.ascendancy) {
		drawAscendancy(ctx, nodes, view);
		return;
	}
	const t = view.transform;
	if (!t) return;
	// Ascendancy nodes live elsewhere in the data; the main tree leaves them out.
	nodes = nodes.filter((n) => !n.ascendancy);
	const at = (n: TreeNode) =>
		[n.x * t.scale + t.offsetX, n.y * t.scale + t.offsetY] as const;
	const allocated = new Set(view.allocated);
	const byId = new Map(nodes.map((n) => [n.id, n]));

	// Paths between allocated nodes.
	ctx.strokeStyle = "rgba(76, 195, 138, 0.9)";
	ctx.lineWidth = Math.max(2, 10 * t.scale);
	ctx.beginPath();
	for (const id of allocated) {
		const a = byId.get(id);
		if (!a) continue;
		for (const o of a.out) {
			const b = byId.get(o);
			if (!b || !(allocated.has(o) || b.kind === "classStart")) continue;
			const [ax, ay] = at(a);
			const [bx, by] = at(b);
			ctx.moveTo(ax, ay);
			ctx.lineTo(bx, by);
		}
	}
	ctx.stroke();

	// Fixing points: show every node faintly so unallocated ones can be clicked.
	if (view.editing) {
		ctx.fillStyle = "rgba(230, 227, 220, 0.35)";
		for (const n of nodes) {
			if (allocated.has(n.id) || n.kind === "classStart") continue;
			const [x, y] = at(n);
			ctx.beginPath();
			ctx.arc(
				x,
				y,
				Math.max(2, RADIUS[n.kind] * t.scale * 0.5),
				0,
				Math.PI * 2,
			);
			ctx.fill();
		}
	}

	// Rings on allocated nodes; the class start is marked as the anchor.
	ctx.lineWidth = Math.max(2, 6 * t.scale);
	for (const n of nodes) {
		const isStart = n.id === view.classStart?.id;
		if (!allocated.has(n.id) && !isStart) continue;
		const [x, y] = at(n);
		ctx.strokeStyle = isStart
			? "rgba(200, 164, 90, 0.95)"
			: "rgba(76, 195, 138, 0.95)";
		ctx.beginPath();
		ctx.arc(x, y, Math.max(4, RADIUS[n.kind] * t.scale), 0, Math.PI * 2);
		ctx.stroke();
	}

	// Next Points from the Build. The next few are gold numbered badges
	// centred on the node with a breathing glow; the rest are dashed rings.
	const NUMBERED = 10;
	const numPx = Math.max(17, 46 * t.scale);
	const badgeR = numPx * 0.72;
	ctx.setLineDash([Math.max(3, 12 * t.scale), Math.max(2, 8 * t.scale)]);
	ctx.lineWidth = Math.max(2, 6 * t.scale);
	ctx.strokeStyle = "rgba(242, 201, 76, 0.5)";
	for (const id of view.planned.slice(NUMBERED)) {
		const n = byId.get(id);
		if (!n) continue;
		const [x, y] = at(n);
		ctx.beginPath();
		ctx.arc(x, y, Math.max(4, RADIUS[n.kind] * t.scale), 0, Math.PI * 2);
		ctx.stroke();
	}
	ctx.setLineDash([]);

	const next = view.planned
		.slice(0, NUMBERED)
		.map((id, i) => ({ n: byId.get(id), i }))
		.filter((x): x is { n: TreeNode; i: number } => !!x.n);
	// Glow first (added light, brightening the tree under it)...
	ctx.globalCompositeOperation = "lighter";
	for (const { n } of next) {
		const [x, y] = at(n);
		const outer = badgeR * (2 + pulse * 1.6);
		const glow = ctx.createRadialGradient(x, y, badgeR * 0.8, x, y, outer);
		glow.addColorStop(0, `rgba(255, 206, 84, ${0.35 + pulse * 0.5})`);
		glow.addColorStop(1, "rgba(255, 206, 84, 0)");
		ctx.fillStyle = glow;
		ctx.beginPath();
		ctx.arc(x, y, outer, 0, Math.PI * 2);
		ctx.fill();
	}
	ctx.globalCompositeOperation = "source-over";
	// ...then a pulsing ring and the solid numbered badge on top.
	ctx.font = `700 ${numPx}px "Segoe UI", sans-serif`;
	ctx.textAlign = "center";
	ctx.textBaseline = "middle";
	for (const { n, i } of next) {
		const [x, y] = at(n);
		ctx.strokeStyle = `rgba(255, 220, 120, ${0.5 + pulse * 0.5})`;
		ctx.lineWidth = 2 + pulse * 3;
		ctx.beginPath();
		ctx.arc(x, y, badgeR + 3 + pulse * 4, 0, Math.PI * 2);
		ctx.stroke();
		// Solid gold badge, dark number: unlike the outlined mastery numbers.
		ctx.fillStyle = "rgba(242, 201, 76, 1)";
		ctx.beginPath();
		ctx.arc(x, y, badgeR, 0, Math.PI * 2);
		ctx.fill();
		ctx.fillStyle = "rgba(18, 19, 22, 1)";
		ctx.fillText(String(i + 1), x, y + numPx * 0.05);
	}

	// Planned mastery picks (blue) and ones already made (gold).
	drawMasteryBadges(
		ctx,
		view.plannedMasteries,
		byId,
		at,
		t.scale,
		"rgba(158, 197, 255, 1)",
		pulse,
	);

	// Mastery picks: which option down the in-game list was chosen.
	drawMasteryBadges(
		ctx,
		view.masteries,
		byId,
		at,
		t.scale,
		"rgba(242, 201, 76, 1)",
	);
}

function drawMasteryBadges(
	ctx: CanvasRenderingContext2D,
	picks: { node: number; option: number }[],
	byId: Map<number, TreeNode>,
	at: (n: TreeNode) => readonly [number, number],
	scale: number,
	colour: string,
	/** Breathing glow strength (planned picks still to make), or null. */
	pulse: number | null = null,
) {
	const fontPx = Math.max(18, 70 * scale);
	const r = fontPx * 0.75;
	if (pulse !== null) {
		ctx.globalCompositeOperation = "lighter";
		for (const pick of picks) {
			const n = byId.get(pick.node);
			if (!n) continue;
			const [x, y] = at(n);
			const outer = r * (2 + pulse * 1.6);
			const glow = ctx.createRadialGradient(x, y, r * 0.8, x, y, outer);
			glow.addColorStop(0, `rgba(110, 170, 255, ${0.35 + pulse * 0.5})`);
			glow.addColorStop(1, "rgba(110, 170, 255, 0)");
			ctx.fillStyle = glow;
			ctx.beginPath();
			ctx.arc(x, y, outer, 0, Math.PI * 2);
			ctx.fill();
		}
		ctx.globalCompositeOperation = "source-over";
	}
	ctx.font = `700 ${fontPx}px "Segoe UI", sans-serif`;
	ctx.textAlign = "center";
	ctx.textBaseline = "middle";
	for (const pick of picks) {
		const n = byId.get(pick.node);
		if (!n) continue;
		const [x, y] = at(n);
		// Outlined (a mastery option), unlike the solid Next Point badges.
		ctx.fillStyle = "rgba(14, 15, 18, 0.85)";
		ctx.strokeStyle = colour;
		ctx.lineWidth = pulse !== null ? 2 + pulse * 3 : 2;
		ctx.beginPath();
		ctx.arc(x, y, r, 0, Math.PI * 2);
		ctx.fill();
		ctx.stroke();
		ctx.fillStyle = colour;
		ctx.fillText(String(pick.option), x, y + fontPx * 0.05);
	}
}

/** The ascendancy panel: taken nodes ringed, planned ones numbered. */
function drawAscendancy(
	ctx: CanvasRenderingContext2D,
	nodes: TreeNode[],
	view: TreeView,
) {
	const t = view.ascendancyTransform;
	const name = view.ascendancyName;
	if (!t || !name) return;
	const asc = nodes.filter(
		(n) => n.ascendancy?.toLowerCase() === name.toLowerCase(),
	);
	const byId = new Map(asc.map((n) => [n.id, n]));
	const at = (n: TreeNode) =>
		[n.x * t.scale + t.offsetX, n.y * t.scale + t.offsetY] as const;
	const startId = view.ascendancyStart?.id;
	const taken = new Set(view.ascendancyAllocated);
	if (startId !== undefined) taken.add(startId);

	ctx.strokeStyle = "rgba(76, 195, 138, 0.9)";
	ctx.lineWidth = Math.max(2, 10 * t.scale);
	ctx.beginPath();
	for (const id of taken) {
		const a = byId.get(id);
		if (!a) continue;
		for (const o of a.out) {
			const b = byId.get(o);
			if (!b || !taken.has(o)) continue;
			const [ax, ay] = at(a);
			const [bx, by] = at(b);
			ctx.moveTo(ax, ay);
			ctx.lineTo(bx, by);
		}
	}
	ctx.stroke();

	ctx.lineWidth = Math.max(2, 6 * t.scale);
	for (const id of taken) {
		const n = byId.get(id);
		if (!n) continue;
		const [x, y] = at(n);
		ctx.strokeStyle =
			id === startId ? "rgba(200, 164, 90, 0.95)" : "rgba(76, 195, 138, 0.95)";
		ctx.beginPath();
		ctx.arc(x, y, Math.max(5, RADIUS[n.kind] * t.scale), 0, Math.PI * 2);
		ctx.stroke();
	}

	// Planned ascendancy nodes: a quiet dashed ring and a small number (it's a
	// small area; the order matters less than on the main tree).
	const numPx = Math.max(12, 30 * t.scale);
	ctx.setLineDash([4, 3]);
	ctx.lineWidth = 2;
	ctx.strokeStyle = "rgba(242, 201, 76, 0.75)";
	ctx.font = `600 ${numPx}px "Segoe UI", sans-serif`;
	ctx.textAlign = "center";
	ctx.textBaseline = "middle";
	view.ascendancyPlanned.forEach((id, i) => {
		const n = byId.get(id);
		if (!n) return;
		const [x, y] = at(n);
		const r = Math.max(6, RADIUS[n.kind] * t.scale);
		ctx.beginPath();
		ctx.arc(x, y, r, 0, Math.PI * 2);
		ctx.stroke();
		ctx.fillStyle = "rgba(242, 201, 76, 0.9)";
		ctx.fillText(String(i + 1), x + r + numPx * 0.5, y - r);
	});
	ctx.setLineDash([]);
}
