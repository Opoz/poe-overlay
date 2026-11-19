#!/usr/bin/env node
// Builds crates/poe-core/data/quests.json: for each quest reward offer, the
// NPC to hand it in to and whether it gives a book.
//
// Sources (see THIRD_PARTY.md):
// - exile-leveling's quests.json: offers and NPCs (MIT).
// - RePoE's base items: the books' names (Book of Skill / Book of Regrets).
//
// Usage: node scripts/gen-quests.mjs

import { writeFile } from "node:fs/promises";

const QUESTS =
	"https://raw.githubusercontent.com/HeartofPhos/exile-leveling/main/common/data/json/quests.json";
const ITEMS =
	"https://raw.githubusercontent.com/repoe-fork/repoe-fork.github.io/master/data/base_items.json";
const OUT = new URL("../crates/poe-core/data/quests.json", import.meta.url);

const BOOKS = { "Book of Skill": "skill", "Book of Regrets": "regrets" };

async function getJson(url) {
	const res = await fetch(url);
	if (!res.ok) throw new Error(`${url}: ${res.status}`);
	return res.json();
}

const [quests, items] = await Promise.all([getJson(QUESTS), getJson(ITEMS)]);

const out = {};
for (const q of Object.values(quests)) {
	const offers = {};
	for (const [offerId, offer] of Object.entries(q.reward_offers)) {
		const book = Object.keys(offer.quest ?? {})
			.map((id) => BOOKS[items[id]?.name])
			.find(Boolean);
		offers[offerId] = { npc: offer.quest_npc, ...(book ? { book } : {}) };
	}
	out[q.id] = { name: q.name, offers };
}

await writeFile(OUT, `${JSON.stringify(out)}\n`);
console.log(`wrote ${Object.keys(out).length} quests`);
