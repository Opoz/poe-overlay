import { invoke } from "@tauri-apps/api/core";
import {
	createEffect,
	createResource,
	createSignal,
	onCleanup,
	Show,
} from "solid-js";
import { onEvent } from "../events";
import { keyFor } from "../hotkeys";
import type { AtlasView, TreeNode } from "../types";

/** Node sizes in tree units (the ring goes just outside). */
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
 * Covers the game window over the in-game atlas tree and rings every node of
 * the imported plan. Rings leave the nodes' centres clear: the game's lit and
 * unlit nodes show what's taken, which the log can't tell us.
 */
export default function AtlasOverlay() {
	const [nodes] = createResource(() => invoke<TreeNode[]>("get_atlas_tree"));
	const [view, setView] = createSignal<AtlasView | null>(null);
	onEvent<AtlasView>("atlas-view", setView);
	invoke<AtlasView>("get_atlas_view").then(setView);

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

	// Clicks only arrive while calibrating; otherwise they go to the game.
	const onClick = (e: MouseEvent) => {
		if (!view()?.calibrating) return;
		const dpr = window.devicePixelRatio;
		invoke("atlas_click", { x: e.clientX * dpr, y: e.clientY * dpr });
	};
	document.addEventListener("mousedown", onClick);
	onCleanup(() => document.removeEventListener("mousedown", onClick));

	const prompt = () => {
		const c = view()?.calibrating;
		if (!c) return null;
		const again = keyFor("atlas.calibrate") ?? "Calibrate";
		if (c.step === "quick")
			return `Zoom the atlas tree all the way out and click the middle of its start, in the centre. ${again} again for two-click calibration.`;
		if (c.step === "start")
			return "Zoom the atlas tree all the way out, then click the middle of its start, in the centre.";
		return `Now click the middle of the ${view()?.partner?.name ?? "topmost"} keystone, at the top.`;
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
						<strong>Calibrating the atlas:</strong> {text()}
					</div>
				)}
			</Show>
			<Show
				when={
					view()?.visible &&
					!view()?.calibrating &&
					(!view()?.transform || view()?.planned.length === 0)
				}
			>
				<div class="tree-prompt">
					{view()?.planned.length === 0
						? "No atlas plan yet: import one on the Atlas page of the main window."
						: `Calibrate first: ${keyFor("atlas.calibrate") ?? "Calibrate"}.`}
				</div>
			</Show>
		</div>
	);
}

function draw(
	canvas: HTMLCanvasElement | undefined,
	size: { w: number; h: number },
	nodes: TreeNode[] | undefined,
	view: AtlasView | null,
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
	if (!nodes || !view || !t || view.calibrating) return;

	const planned = new Set(view.planned);
	for (const n of nodes) {
		if (!planned.has(n.id)) continue;
		const x = n.x * t.scale + t.offsetX;
		const y = n.y * t.scale + t.offsetY;
		const r = Math.max(5, RADIUS[n.kind] * t.scale) + 3 * dpr;
		// A dark halo under the ring keeps it visible on bright art.
		ctx.beginPath();
		ctx.arc(x, y, r, 0, Math.PI * 2);
		ctx.lineWidth = 4.5 * dpr;
		ctx.strokeStyle = "rgb(0 0 0 / 0.55)";
		ctx.stroke();
		ctx.beginPath();
		ctx.arc(x, y, r, 0, Math.PI * 2);
		ctx.lineWidth = 2.2 * dpr;
		ctx.strokeStyle = n.kind === "mastery" ? "#8ad8ff" : "#f5cf5a";
		ctx.stroke();
	}
}
