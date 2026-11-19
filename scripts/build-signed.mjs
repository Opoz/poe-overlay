#!/usr/bin/env node
// A local release build, signed with the updater key at ~/.tauri/poe-overlay.key
// (see docs/RELEASING.md). Output: target/release/bundle/nsis/.

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const keyPath = join(homedir(), ".tauri", "poe-overlay.key");
if (!existsSync(keyPath)) {
	console.error(`No updater key at ${keyPath}. See docs/RELEASING.md.`);
	process.exit(1);
}

const r = spawnSync("pnpm", ["tauri", "build"], {
	stdio: "inherit",
	shell: true,
	env: {
		...process.env,
		TAURI_SIGNING_PRIVATE_KEY: readFileSync(keyPath, "utf8"),
		TAURI_SIGNING_PRIVATE_KEY_PASSWORD:
			process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
	},
});
process.exit(r.status ?? 1);
