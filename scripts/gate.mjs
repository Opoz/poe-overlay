#!/usr/bin/env node
// Every check CI runs, in order, stopping at the first failure: pnpm gate
// (--fix formats and applies safe lint fixes first; --quick stops after
// formatting, lint and types). The git hooks in .githooks/ run it.

import { spawnSync } from "node:child_process";

const fix = process.argv.includes("--fix");
// --quick: formatting, lint and types only (seconds; the pre-commit hook).
const quick = process.argv.includes("--quick");

const steps = [
	...(fix
		? [
				["cargo", ["fmt", "--all"]],
				["pnpm", ["-s", "fix"]],
			]
		: []),
	["cargo", ["fmt", "--all", "--check"]],
	["pnpm", ["-s", "check"]],
	...(quick
		? []
		: [
				[
					"cargo",
					["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
				],
				["cargo", ["test", "--workspace", "-q"]],
				["pnpm", ["-s", "test"]],
			]),
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
