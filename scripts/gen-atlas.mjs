#!/usr/bin/env node
// Builds crates/poe-core/data/atlas.json: the atlas passive tree with
// absolute node positions, in the same shape as tree.json (no classes, no
// Client Log ids: the log doesn't record atlas points).
//
// Source (see THIRD_PARTY.md): GGG's official atlas tree export.
//
// Usage: node scripts/gen-atlas.mjs

import { writeFile } from "node:fs/promises";

const GGG =
	"https://raw.githubusercontent.com/grindinggear/atlastree-export/master/data.json";
const OUT = new URL("../crates/poe-core/data/atlas.json", import.meta.url);

// Same uneven orbits as the passive tree.
const ORBIT_ANGLES = {
	16: [
		0, 30, 45, 60, 90, 120, 135, 150, 180, 210, 225, 240, 270, 300, 315, 330,
	],
	40: [
		0, 10, 20, 30, 40, 45, 50, 60, 70, 80, 90, 100, 110, 120, 130, 135, 140,
		150, 160, 170, 180, 190, 200, 210, 220, 225, 230, 240, 250, 260, 270, 280,
		290, 300, 310, 315, 320, 330, 340, 350,
	],
};

const angleDegrees = (slots, index) =>
	ORBIT_ANGLES[slots]?.[index] ?? (360 * index) / slots;

function kindOf(n) {
	if (n.isKeystone) return "keystone";
	if (n.isMastery) return "mastery";
	if (n.isJewelSocket) return "jewel";
	if (n.isNotable) return "notable";
	return "normal";
}

const res = await fetch(GGG);
if (!res.ok) throw new Error(`${GGG}: ${res.status}`);
const ggg = await res.json();
const { orbitRadii, skillsPerOrbit } = ggg.constants;

const position = (n) => {
	const g = ggg.groups[n.group];
	const radius = orbitRadii[n.orbit];
	const a =
		(angleDegrees(skillsPerOrbit[n.orbit], n.orbitIndex) * Math.PI) / 180;
	return {
		x: Math.round((g.x + radius * Math.sin(a)) * 10) / 10,
		y: Math.round((g.y - radius * Math.cos(a)) * 10) / 10,
	};
};

// The export's "root" has no position; the node it leads to is the unnamed
// hub in the middle of the atlas tree, where every path starts.
const startId = Number(ggg.nodes.root.out[0]);

const nodes = [];
const kept = new Set();
for (const n of Object.values(ggg.nodes)) {
	if (n.group === undefined || n.isProxy || !ggg.groups[n.group]) continue;
	const start = n.skill === startId;
	const id = n.skill;
	nodes.push({
		id,
		name: start ? "Atlas start" : n.name,
		kind: start ? "classStart" : kindOf(n),
		group: n.group,
		...position(n),
		...(start ? { classStart: 0 } : {}),
		...(n.masteryEffects?.length
			? { masteryEffects: n.masteryEffects.map((e) => e.effect) }
			: {}),
		out: (n.out ?? []).map(Number),
	});
	kept.add(id);
}
for (const n of nodes) n.out = n.out.filter((id) => kept.has(id));

const atlas = {
	source: "grindinggear/atlastree-export",
	classes: [{ name: "Atlas", startNode: startId, ascendancies: [] }],
	nodes,
	logIds: {},
};
await writeFile(OUT, `${JSON.stringify(atlas)}\n`);
console.log(`wrote ${nodes.length} atlas nodes`);
