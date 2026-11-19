#!/usr/bin/env node
// Every check CI runs, in order, stopping at the first failure: pnpm gate
// (--fix formats and applies safe lint fixes first; --quick stops after
// formatting, lint and types). The git hooks in .githooks/ run it.
// Each step's output is shown only when it fails.

import { spawnSync } from "node:child_process";

const fix = process.argv.includes("--fix");
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
	process.stdout.write(`> ${line} `);
	const r = spawnSync(cmd, args, {
		shell: true,
		encoding: "utf8",
		maxBuffer: 64 * 1024 * 1024,
	});
	if (r.status !== 0) {
		console.log("FAILED\n");
		process.stdout.write(r.stdout ?? "");
		process.stderr.write(r.stderr ?? "");
		console.error(`\nFailed: ${line}`);
		process.exit(r.status ?? 1);
	}
	console.log("ok");
}
console.log("All checks passed.");
