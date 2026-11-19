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
const EDGE =
	"C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const BASE = "http://localhost:1420/preview.html";

/** [name, query, width, height] */
const SHOTS = [
	["main", "w=main", 1000, 2200],
	["guide-dark", "w=guide", 400, 560],
	["guide-bright", "w=guide&bg=bright", 400, 560],
	["guide-interact", "w=guide&interact", 400, 560],
	["act", "w=act&act=1", 540, 740],
	["timer-dark", "w=timer", 300, 360],
	["timer-bright", "w=timer&bg=bright", 300, 360],
	["gems-dark", "w=gems", 340, 420],
	["gems-bright", "w=gems&bg=bright", 340, 420],
	["xp-dark", "w=xp", 400, 130],
	["xp-bright", "w=xp&bg=bright", 400, 130],
	["lab", "w=lab", 800, 620],
	["treebar", "w=treebar", 880, 70],
	["treebar-hidden", "w=treebar&hidden", 880, 70],
	["tree", "w=tree", 1600, 900],
	["tree-fix", "w=tree&fix", 1600, 900],
];

const [outArg, filter] = process.argv.slice(2);
const out = resolve(outArg ?? "shots");
await mkdir(out, { recursive: true });

for (const [name, query, w, h] of SHOTS) {
	if (filter && !name.includes(filter)) continue;
	const file = join(out, `${name}.png`);
	await run(EDGE, [
		"--headless=new",
		"--disable-gpu",
		"--hide-scrollbars",
		"--force-device-scale-factor=1",
		`--window-size=${w},${h}`,
		"--virtual-time-budget=4000",
		`--screenshot=${file}`,
		`${BASE}?${query}`,
	]);
	console.log(file);
}
