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
}
