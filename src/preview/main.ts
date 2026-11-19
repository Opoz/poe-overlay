/**
 * UI preview: renders any window with realistic data in a plain browser, so
 * the interface can be screenshotted and reviewed without the game.
 *
 *   /preview.html?w=guide&bg=bright&act=1
 *
 * `w` is the window label, `bg` the backdrop (dark | bright | tree).
 * Fixtures come from `cargo run -p poe-core --example preview_fixtures`.
 */
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import fixtures from "./fixtures.json";
import "./preview.css";

const params = new URLSearchParams(location.search);
const label = params.get("w") ?? "main";
document.documentElement.dataset.backdrop = params.get("bg") ?? "dark";

const settings = {
	logPath: null,
	category: "SSF HC",
	pauseInHideout: true,
	timerOverlay: { x: 0.01, y: 0.25 },
	treeKey: "P",
	overlayOpacity: 1,
};

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
		logPath: "E:SteamLibrarysteamappscommonPath of ExilelogsClient.txt",
		gameRunning: true,
	},
	get_history: fixtures.history,
	get_interact: params.has("interact"),
	hotkey_failures: params.has("conflict") ? ["alt+KeyR"] : [],
};

mockWindows(label);
mockIPC(
	(cmd) => {
		if (cmd in responses) return responses[cmd];
		if (cmd === "plugin:window|outer_position") return { x: 0, y: 0 };
		if (cmd === "plugin:window|outer_size")
			return { width: window.innerWidth, height: window.innerHeight };
		if (cmd === "plugin:window|scale_factor") return 1;
		return null;
	},
	{ shouldMockEvents: true },
);

await import("../index");
