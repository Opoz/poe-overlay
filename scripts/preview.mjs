#!/usr/bin/env node
// Regenerates the UI preview's fixtures from the real core, then screenshots
// every window. Needs the dev server running (pnpm dev).
//
// Usage: pnpm preview:shots [out dir] [name filter]
//   --no-fixtures  skip regenerating src/preview/fixtures.json

import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const args = process.argv.slice(2);
const skipFixtures = args.includes("--no-fixtures");
const [out = "shots", filter] = args.filter((a) => !a.startsWith("--"));

if (!skipFixtures) {
	const r = spawnSync(
		"cargo",
		["run", "-q", "-p", "poe-core", "--example", "preview_fixtures"],
		{ encoding: "utf8", maxBuffer: 64 * 1024 * 1024, shell: true },
	);
	if (r.status !== 0) {
		process.stderr.write(r.stderr);
		process.exit(r.status ?? 1);
	}
	writeFileSync("src/preview/fixtures.json", r.stdout);
	console.log("Wrote src/preview/fixtures.json");
}

const shots = spawnSync(
	"node",
	["scripts/screenshot.mjs", out, ...(filter ? [filter] : [])],
	{ stdio: "inherit", shell: true },
);
process.exit(shots.status ?? 0);
