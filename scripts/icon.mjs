#!/usr/bin/env node
// Renders assets/icon.svg to assets/icon.png (1024px, transparent) with
// headless Edge, then regenerates the app's icon set from it (Windows/desktop
// sizes only; the mobile folders `tauri icon` also writes are removed).

import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const EDGE = "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe";
const svg = pathToFileURL(resolve("assets/icon.svg")).href;
const png = resolve("assets/icon.png");

const dir = mkdtempSync(join(tmpdir(), "poe-overlay-icon-"));
const page = join(dir, "icon.html");
writeFileSync(
	page,
	`<!doctype html><style>html,body{margin:0;background:transparent}img{display:block;width:1024px;height:1024px}</style><img src="${svg}">`,
);
const shot = spawnSync(EDGE, [
	"--headless=new",
	"--disable-gpu",
	"--hide-scrollbars",
	"--force-device-scale-factor=1",
	"--default-background-color=00000000",
	`--user-data-dir=${join(dir, "edge")}`,
	"--window-size=1024,1024",
	`--screenshot=${png}`,
	pathToFileURL(page).href,
]);
rmSync(dir, { recursive: true, force: true });
if (shot.status !== 0) {
	console.error("Edge couldn't render the icon");
	process.exit(1);
}
console.log(`Rendered ${png}`);

const icons = spawnSync("pnpm", ["tauri", "icon", "assets/icon.png"], {
	stdio: "inherit",
	shell: true,
});
for (const mobile of ["android", "ios"])
	rmSync(join("src-tauri", "icons", mobile), { recursive: true, force: true });
process.exit(icons.status ?? 1);
