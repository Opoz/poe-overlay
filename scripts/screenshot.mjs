#!/usr/bin/env node
// Screenshots every window of the UI preview with headless Edge.
// Needs the dev server running (pnpm dev or pnpm tauri dev).
//
// Usage: node scripts/screenshot.mjs <out dir> [name filter]

import { execFile } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);
const EDGE = "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe";
const BASE = "http://localhost:1420/preview.html";

/**
 * [name, query, width, height]: the canvas, which holds the Overlay window
 * (its real size, outlined) 20px in from the top left, over a backdrop.
 * Headless Edge won't go narrower than about 500px.
 */
const SHOTS = [
	["main", "w=main", 960, 680],
	["main-characters", "w=main&page=characters", 960, 680],
	["main-build", "w=main&page=build", 960, 680],
	["main-overlays", "w=main&page=overlays", 960, 680],
	["main-history", "w=main&page=history", 960, 680],
	["main-shortcuts", "w=main&page=shortcuts", 960, 1100],
	["main-settings", "w=main&page=settings&conflict", 960, 680],
	["guide-dark", "w=guide", 520, 480],
	["guide-bright", "w=guide&bg=bright", 520, 480],
	["guide-faint", "w=guide&bg=bright&opacity=0.4", 520, 480],
	["guide-interact", "w=guide&interact", 520, 480],
	["act", "w=act&act=1", 560, 760],
	["timer-dark", "w=timer", 520, 380],
	["timer-bright", "w=timer&bg=bright", 520, 380],
	["gems-dark", "w=gems", 520, 460],
	["gems-bright", "w=gems&bg=bright", 520, 460],
	["xp-dark", "w=xp", 520, 150],
	["xp-bright", "w=xp&bg=bright", 520, 150],
	["notes-dark", "w=notes", 520, 620],
	["notes-bright", "w=notes&bg=bright", 520, 620],
	["lab", "w=lab", 820, 600],
	["treebar", "w=treebar", 900, 90],
	["treebar-hidden", "w=treebar&hidden", 900, 90],
	["tree", "w=tree", 1600, 900],
	["tree-fix", "w=tree&fix", 1600, 900],
];

const [outArg, filter] = process.argv.slice(2);
const out = resolve(outArg ?? "shots");
await mkdir(out, { recursive: true });

await Promise.all(
	SHOTS.filter(([name]) => !filter || name.includes(filter)).map(
		async ([name, query, w, h]) => {
			const file = join(out, `${name}.png`);
			await run(EDGE, [
				"--headless=new",
				"--disable-gpu",
				"--hide-scrollbars",
				"--force-device-scale-factor=1",
				`--user-data-dir=${join(out, ".edge", name)}`,
				`--window-size=${w},${h}`,
				"--virtual-time-budget=5000",
				`--screenshot=${file}`,
				`${BASE}?${query}`,
			]);
			console.log(file);
		},
	),
);
