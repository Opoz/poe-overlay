#!/usr/bin/env node
// Every check CI runs, in order, stopping at the first failure. Run before
// each commit: pnpm gate (add --fix to format and apply safe lint fixes first).

import { spawnSync } from "node:child_process";

const fix = process.argv.includes("--fix");

const steps = [
	...(fix
		? [
				["cargo", ["fmt", "--all"]],
				["pnpm", ["-s", "fix"]],
			]
		: []),
	["cargo", ["fmt", "--all", "--check"]],
	["cargo", ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]],
	["cargo", ["test", "--workspace", "-q"]],
	["pnpm", ["-s", "check"]],
	["pnpm", ["-s", "test"]],
];

for (const [cmd, args] of steps) {
	const line = [cmd, ...args].join(" ");
	console.log(`\n> ${line}`);
	const r = spawnSync(cmd, args, { stdio: "inherit", shell: true });
	if (r.status !== 0) {
		console.error(`\nFailed: ${line}`);
		process.exit(r.status ?? 1);
	}
}
console.log("\nAll checks passed.");
