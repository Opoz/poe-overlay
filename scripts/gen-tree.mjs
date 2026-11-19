#!/usr/bin/env node
// Builds crates/poe-core/data/tree.json: a trimmed passive tree with absolute
// node positions plus the Client Log passive id -> node id map.
//
// Sources (see THIRD_PARTY.md):
// - GGG's official skill tree export: layout, names, connections.
// - RePoE: maps the string ids the Client Log prints ("intelligence101")
//   to tree node ids (19635).
//
// Usage: node scripts/gen-tree.mjs

import { writeFile } from "node:fs/promises";

const GGG =
	"https://raw.githubusercontent.com/grindinggear/skilltree-export/master/data.json";
const REPOE =
	"https://raw.githubusercontent.com/repoe-fork/repoe-fork.github.io/master/data/passive_skill_trees/Default.json";
const OUT = new URL("../crates/poe-core/data/tree.json", import.meta.url);

// Orbits with 16 and 40 slots are not evenly spaced (since 3.17).
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

function angleDegrees(slots, index) {
	return ORBIT_ANGLES[slots]?.[index] ?? (360 * index) / slots;
}

function kindOf(n) {
	if (n.classStartIndex !== undefined) return "classStart";
	if (n.isAscendancyStart) return "ascendancyStart";
	if (n.isKeystone) return "keystone";
	if (n.isMastery) return "mastery";
	if (n.isJewelSocket) return "jewel";
	if (n.isNotable) return "notable";
	return "normal";
}

async function getJson(url) {
	const res = await fetch(url);
	if (!res.ok) throw new Error(`${url}: ${res.status}`);
	return res.json();
}

const [ggg, repoe] = await Promise.all([getJson(GGG), getJson(REPOE)]);
const { orbitRadii, skillsPerOrbit } = ggg.constants;

const nodes = [];
const kept = new Set();
for (const n of Object.values(ggg.nodes)) {
	if (n.group === undefined || n.isProxy) continue;
	const g = ggg.groups[n.group];
	if (!g) continue;
	const radius = orbitRadii[n.orbit];
	const a =
		(angleDegrees(skillsPerOrbit[n.orbit], n.orbitIndex) * Math.PI) / 180;
	nodes.push({
		id: n.skill,
		name: n.name,
		kind: kindOf(n),
		x: Math.round((g.x + radius * Math.sin(a)) * 10) / 10,
		y: Math.round((g.y - radius * Math.cos(a)) * 10) / 10,
		...(n.ascendancyName ? { ascendancy: n.ascendancyName } : {}),
		...(n.classStartIndex !== undefined
			? { classStart: n.classStartIndex }
			: {}),
		// In the order the game lists them, so a pick's position is its option number.
		...(n.masteryEffects?.length
			? { masteryEffects: n.masteryEffects.map((e) => e.effect) }
			: {}),
		out: (n.out ?? []).map(Number),
	});
	kept.add(n.skill);
}
for (const n of nodes) n.out = n.out.filter((id) => kept.has(id));

const logIds = {};
for (const p of Object.values(repoe.passives)) {
	if (p.id && kept.has(p.hash)) logIds[p.id] = p.hash;
}

const classes = ggg.classes.map((c, i) => ({
	name: c.name,
	startNode: nodes.find((n) => n.classStart === i)?.id ?? null,
	ascendancies: c.ascendancies.map((a) => a.name),
}));

const tree = {
	source: "grindinggear/skilltree-export + repoe-fork",
	generated: new Date().toISOString().slice(0, 10),
	classes,
	nodes,
	logIds,
};
await writeFile(OUT, `${JSON.stringify(tree)}\n`);
console.log(
	`wrote ${nodes.length} nodes, ${Object.keys(logIds).length} log ids, ${classes.length} classes`,
);
