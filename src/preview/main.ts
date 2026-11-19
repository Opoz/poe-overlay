/**
 * UI preview: renders any window with realistic data in a plain browser, so
 * the interface can be screenshotted and reviewed without the game.
 *
 *   /preview.html?w=guide&bg=bright&act=1
 *
 * `w` is the window label, `bg` the backdrop (dark | bright | tree). An
 * Overlay renders inside a frame the size of its real window (dashed outline),
 * which follows the window's own auto-sizing, over the backdrop.
 * Fixtures come from `cargo run -p poe-core --example preview_fixtures`.
 */
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import fixtures from "./fixtures.json";
import "./preview.css";

const params = new URLSearchParams(location.search);
const label = params.get("w") ?? "main";
const inner = params.has("inner");

/** Logical window sizes, as created by `src-tauri/src/overlay.rs`. */
const SIZES: Record<string, [number, number]> = {
	timer: [280, 340],
	guide: [380, 420],
	act: [520, 720],
	gems: [320, 420],
	lab: [780, 560],
	treebar: [860, 52],
	xp: [380, 110],
};

if (!inner && label in SIZES) {
	// The page is the game behind; the Overlay window is an iframe on top.
	document.documentElement.dataset.backdrop = params.get("bg") ?? "dark";
	const [w, h] = SIZES[label];
	const frame = document.createElement("iframe");
	frame.className = "preview-frame";
	frame.width = String(w);
	frame.height = String(h);
	frame.src = `${location.pathname}${location.search}&inner`;
	document.body.append(frame);
	window.addEventListener("message", (e) => {
		if (e.data?.kind !== "resize") return;
		frame.width = String(e.data.width);
		frame.height = String(e.data.height);
	});
} else {
	await boot();
}

async function boot() {
	// The main window paints its own background.
	if (label !== "main")
		document.documentElement.dataset.backdrop = inner
			? "none"
			: (params.get("bg") ?? "dark");

	const settings = {
		logPath: null,
		category: "SSF HC",
		pauseInHideout: true,
		timerOverlay: { x: 0.01, y: 0.25 },
		treeKey: "P",
		panelOpacity: Number(params.get("opacity") ?? 0.85),
		overlays: { gems: { scale: 1.2, opacity: 0.95, shown: true } },
	};

	// Mirrors src-tauri/src/hotkeys.rs (one rebound, one unset, for show).
	const hotkeyRows = [
		["timer.pause", "Pause or resume the timer", "Timer", "alt+KeyP"],
		["timer.split", "Split", "Timer", "alt+KeyS"],
		["timer.undoSplit", "Undo the last split", "Timer", "shift+alt+KeyS"],
		["guide.next", "Next step", "Guide", "alt+BracketRight"],
		["guide.previous", "Previous step", "Guide", "alt+BracketLeft"],
		[
			"guide.findMe",
			"Find me: jump to the area you're in",
			"Guide",
			"alt+KeyJ",
		],
		["guide.act", "Show the full act", "Guide", "alt+KeyG"],
		["gems.links", "Show gem links", "Gems", "alt+KeyL"],
		["gems.vendorSearch", "Copy the vendor search", "Gems", "alt+KeyV"],
		[
			"tree.toggle",
			"Show or hide the tree overlay",
			"Passive tree",
			"alt+KeyT",
		],
		[
			"tree.calibrate",
			"Calibrate the tree overlay",
			"Passive tree",
			"shift+alt+KeyT",
		],
		[
			"tree.previous",
			"Previous build tree",
			"Passive tree",
			"shift+alt+BracketLeft",
		],
		["tree.next", "Next build tree", "Passive tree", "shift+alt+BracketRight"],
		["lab.toggle", "Show or hide the lab map", "Labyrinth", "alt+KeyB"],
		["lab.import", "Open today's lab on poelab", "Labyrinth", "shift+alt+KeyB"],
		["xp.toggle", "Show or hide the XP ruler", "Overlays", "alt+KeyX"],
		[
			"overlays.interact",
			"Interact Mode: move and click overlays",
			"Overlays",
			"alt+KeyE",
		],
	].map(([id, label, group, def]) => ({
		id,
		label,
		group,
		default: def,
		shortcut: id === "xp.toggle" ? "" : id === "timer.split" ? "F2" : def,
	}));

	const responses: Record<string, unknown> = {
		get_timer_view: fixtures.timer,
		get_guide_view: { ...fixtures.guide, actView: params.has("act") },
		get_gems_view: { ...fixtures.gems, visible: true },
		get_tree: fixtures.treeNodes,
		get_tree_view: {
			...fixtures.tree,
			editing: params.has("fix"),
			visible: !params.has("hidden"),
		},
		get_lab_view: { ...fixtures.lab, visible: true },
		get_characters: fixtures.characters,
		get_settings: settings,
		get_status: {
			logPath: String.raw`E:\SteamLibrary\steamapps\common\Path of Exile\logs\Client.txt`,
			gameRunning: true,
		},
		get_history: fixtures.history,
		get_interact: params.has("interact"),
		hotkey_failures: params.has("conflict") ? ["alt+KeyR"] : [],
		get_hotkeys: hotkeyRows,
		get_overlays_shown: { timer: true, guide: true, xp: true, gems: false },
	};

	mockWindows(label);
	mockIPC(
		(cmd, args) => {
			if (cmd in responses) return responses[cmd];
			if (cmd === "plugin:window|outer_position") return { x: 0, y: 0 };
			if (cmd === "plugin:window|outer_size")
				return { width: window.innerWidth, height: window.innerHeight };
			if (cmd === "plugin:window|scale_factor") return 1;
			if (cmd === "plugin:window|set_size" && inner) {
				// biome-ignore lint/suspicious/noExplicitAny: Tauri's Size wire format
				const v = (args as any)?.value;
				const s = v?.size ?? v?.Logical ?? v?.Physical ?? v;
				if (s?.width)
					parent.postMessage(
						{ kind: "resize", width: s.width, height: s.height },
						"*",
					);
			}
			return null;
		},
		{ shouldMockEvents: true },
	);

	await import("../index");
}
