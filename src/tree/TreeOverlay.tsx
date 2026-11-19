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

	createEffect(() => draw(canvas, size(), nodes(), view()));

	// Clicks are only delivered while calibrating (otherwise click-through).
	const onClick = (e: MouseEvent) => {
		if (!view()?.calibrating) return;
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
			classList={{ calibrating: !!view()?.calibrating }}
		>
			<canvas ref={canvas} />
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
			<Show when={view()?.stage && view()?.transform && !view()?.calibrating}>
				<div class="tree-stage">
					Working towards <strong>{view()?.stage}</strong> ·{" "}
					{view()?.planned.length ?? 0} points to go
				</div>
			</Show>
			<Show when={view() && !view()?.calibrating && !view()?.transform}>
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

	const t = view?.transform;
	if (!t || !nodes || view?.calibrating) return;
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

	// Next Points from the Build: dashed rings, the next few numbered.
	const NUMBERED = 10;
	ctx.setLineDash([Math.max(3, 12 * t.scale), Math.max(2, 8 * t.scale)]);
	ctx.strokeStyle = "rgba(242, 201, 76, 0.9)";
	ctx.lineWidth = Math.max(2, 6 * t.scale);
	view.planned.forEach((id) => {
		const n = byId.get(id);
		if (!n) return;
		const [x, y] = at(n);
		ctx.beginPath();
		ctx.arc(x, y, Math.max(4, RADIUS[n.kind] * t.scale), 0, Math.PI * 2);
		ctx.stroke();
	});
	ctx.setLineDash([]);
	const numPx = Math.max(11, 46 * t.scale);
	ctx.font = `700 ${numPx}px "Segoe UI", sans-serif`;
	ctx.textAlign = "center";
	ctx.textBaseline = "middle";
	view.planned.slice(0, NUMBERED).forEach((id, i) => {
		const n = byId.get(id);
		if (!n) return;
		const [x, y] = at(n);
		const r = Math.max(4, RADIUS[n.kind] * t.scale);
		const bx = x + r * 0.9;
		const by = y - r * 0.9;
		ctx.fillStyle = "rgba(14, 15, 18, 0.9)";
		ctx.beginPath();
		ctx.arc(bx, by, numPx * 0.7, 0, Math.PI * 2);
		ctx.fill();
		ctx.fillStyle = "rgba(242, 201, 76, 1)";
		ctx.fillText(String(i + 1), bx, by + numPx * 0.05);
	});

	// Planned mastery picks (blue) and ones already made (gold).
	drawMasteryBadges(
		ctx,
		view.plannedMasteries,
		byId,
		at,
		t.scale,
		"rgba(158, 197, 255, 1)",
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
) {
	const fontPx = Math.max(12, 70 * scale);
	ctx.font = `700 ${fontPx}px "Segoe UI", sans-serif`;
	ctx.textAlign = "center";
	ctx.textBaseline = "middle";
	for (const pick of picks) {
		const n = byId.get(pick.node);
		if (!n) continue;
		const [x, y] = at(n);
		ctx.fillStyle = "rgba(14, 15, 18, 0.85)";
		ctx.beginPath();
		ctx.arc(x, y, fontPx * 0.75, 0, Math.PI * 2);
		ctx.fill();
		ctx.fillStyle = colour;
		ctx.fillText(String(pick.option), x, y + fontPx * 0.05);
	}
}
