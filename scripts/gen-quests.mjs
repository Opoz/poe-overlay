#!/usr/bin/env node
// Builds crates/poe-core/data/quests.json and gems.json.
//
// quests.json: each quest reward offer's hand-in NPC, any book, and the gems
// it offers per class (as a free quest reward, or unlocked at a vendor).
// gems.json: every gem's name, colour, required level and whether it's a
// support.
//
// Sources (see THIRD_PARTY.md):
// - exile-leveling's quests.json and gems.json (MIT).
// - RePoE's base items: the books' names (Book of Skill / Book of Regrets).
//
// Usage: node scripts/gen-quests.mjs

import { writeFile } from "node:fs/promises";

const EXLEV =
	"https://raw.githubusercontent.com/HeartofPhos/exile-leveling/main/common/data/json";
const ITEMS =
	"https://raw.githubusercontent.com/repoe-fork/repoe-fork.github.io/master/data/base_items.json";
const OUT = (name) =>
	new URL(`../crates/poe-core/data/${name}`, import.meta.url);

const BOOKS = { "Book of Skill": "skill", "Book of Regrets": "regrets" };
const COLOURS = { strength: "r", dexterity: "g", intelligence: "b", none: "w" };

async function getJson(url) {
	const res = await fetch(url);
	if (!res.ok) throw new Error(`${url}: ${res.status}`);
	return res.json();
}

const [quests, gems, items] = await Promise.all([
	getJson(`${EXLEV}/quests.json`),
	getJson(`${EXLEV}/gems.json`),
	getJson(ITEMS),
]);

const isGem = (id) => id in gems;

const outQuests = {};
for (const q of Object.values(quests)) {
	const offers = {};
	for (const [offerId, offer] of Object.entries(q.reward_offers)) {
		const rewards = Object.entries(offer.quest ?? {});
		const book = rewards.map(([id]) => BOOKS[items[id]?.name]).find(Boolean);
		const questGems = Object.fromEntries(
			rewards.filter(([id]) => isGem(id)).map(([id, r]) => [id, r.classes]),
		);
		const vendorGems = Object.fromEntries(
			Object.entries(offer.vendor ?? {})
				.filter(([id]) => isGem(id))
				.map(([id, r]) => [id, { classes: r.classes, npc: r.npc }]),
		);
		offers[offerId] = {
			npc: offer.quest_npc,
			...(book ? { book } : {}),
			...(Object.keys(questGems).length ? { questGems } : {}),
			...(Object.keys(vendorGems).length ? { vendorGems } : {}),
		};
	}
	outQuests[q.id] = { name: q.name, offers };
}

const outGems = Object.fromEntries(
	Object.values(gems).map((g) => [
		g.id,
		{
			name: g.name,
			colour: COLOURS[g.primary_attribute] ?? "w",
			level: g.required_level,
			...(g.is_support ? { support: true } : {}),
		},
	]),
);

await writeFile(OUT("quests.json"), `${JSON.stringify(outQuests)}\n`);
await writeFile(OUT("gems.json"), `${JSON.stringify(outGems)}\n`);
console.log(
	`wrote ${Object.keys(outQuests).length} quests, ${Object.keys(outGems).length} gems`,
);
