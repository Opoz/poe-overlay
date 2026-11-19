// Mirrors of the Rust types sent over IPC (crates/poe-core, src-tauri).

export type PauseReason = "manual" | "login" | "hideout" | "otherCharacter";

export const PAUSE_LABEL: Record<PauseReason, string> = {
	manual: "paused",
	login: "at login",
	hideout: "in hideout",
	otherCharacter: "other character",
};

export type Phase =
	| { phase: "running" }
	| { phase: "paused"; reason: PauseReason }
	| { phase: "finished" };

export interface SplitTime {
	realMs: number;
	loadRemovedMs: number;
}

export interface SplitRow {
	act: number;
	time: SplitTime | null;
	pbRealMs: number | null;
	pbLoadRemovedMs: number | null;
	deltaRealMs: number | null;
	deltaLoadRemovedMs: number | null;
	bestSegment: boolean;
}

export interface TimerView {
	phase: Phase | null;
	character: string | null;
	class: string | null;
	category: string | null;
	currentAct: number | null;
	realMs: number;
	loadRemovedMs: number;
	splits: SplitRow[];
	bestPossibleRealMs: number | null;
	bestPossibleLoadRemovedMs: number | null;
}

export interface Run {
	startedAt: number;
	character: string | null;
	class: string | null;
	category: string | null;
	splits: (SplitTime | null)[];
	phase: Phase;
}

export interface Profile {
	name: string;
	class: string | null;
	level: number;
	lastSeen: number;
	run: Run | null;
	passives: string[];
	masteries: Record<string, number>;
	guideOptions: GuideOptions;
	guideStep: number | null;
	lastArea: string | null;
	build: {
		class: string;
		ascendancy: string | null;
		skillGroups: SkillGroupData[];
		treeStages: { title: string | null }[];
	} | null;
	buildSource: string | null;
	banditSetByHand: boolean;
	groupLevels: Record<string, LevelRange | null>;
	skillSet: string | null;
	treeStage: number | null;
}

export interface SkillGroupData {
	label: string | null;
	skillSet: string | null;
	slot: string | null;
	enabled: boolean;
	gems: { name: string }[];
	levels: LevelRange | null;
}

export interface TreeNode {
	id: number;
	name: string;
	kind:
		| "normal"
		| "notable"
		| "keystone"
		| "mastery"
		| "jewel"
		| "classStart"
		| "ascendancyStart";
	x: number;
	y: number;
	classStart?: number;
	out: number[];
}

export interface Transform {
	scale: number;
	offsetX: number;
	offsetY: number;
}

export type Calibrating =
	| { step: "quick" }
	| { step: "classStart" }
	| { step: "partner"; classStartAt: [number, number] };

export interface TreeView {
	visible: boolean;
	editing: boolean;
	calibrating: Calibrating | null;
	transform: Transform | null;
	character: string | null;
	classStart: TreeNode | null;
	partner: TreeNode | null;
	allocated: number[];
	masteries: { node: number; option: number }[];
	stage: string | null;
	stageIndex: number | null;
	stages: string[];
	stageByHand: boolean;
	planned: number[];
	plannedMasteries: { node: number; option: number }[];
}

export interface Characters {
	active: string | null;
	profiles: Profile[];
}

export interface OverlayPlacement {
	x: number;
	y: number;
}

export interface Settings {
	logPath: string | null;
	category: string;
	pauseInHideout: boolean;
	timerOverlay: OverlayPlacement;
	treeKey: string | null;
	overlayOpacity: number;
}

export interface Status {
	logPath: string | null;
	gameRunning: boolean;
}

export type TimerCommand =
	| "start"
	| "togglePause"
	| "split"
	| "undoSplit"
	| "reset";

export type BanditChoice = "killAll" | "alira" | "kraityn" | "oak";

export interface GuideOptions {
	leagueStart: boolean;
	bandit: BanditChoice;
}

export type Fragment =
	| { kind: "text"; text: string }
	| { kind: "enter"; area: string; name: string }
	| { kind: "waypoint"; area: string | null; name: string | null }
	| { kind: "waypointGet" }
	| { kind: "logout" }
	| { kind: "portalSet" }
	| { kind: "portalUse" }
	| {
			kind: "quest";
			name: string;
			npc: string | null;
			book: "skill" | "regrets" | null;
			take?: GemPick[];
			buy?: GemPick[];
	  }
	| { kind: "questItem"; text: string }
	| { kind: "kill"; name: string }
	| { kind: "arena"; name: string }
	| { kind: "trial" }
	| { kind: "generic"; text: string }
	| { kind: "crafting" }
	| { kind: "ascend"; difficulty: string }
	| { kind: "dir"; degrees: number }
	| { kind: "area"; area: string; name: string };

export interface GuideLine {
	parts: Fragment[];
	notes: Fragment[][];
}

export interface GuideStep {
	id: number;
	act: number;
	area: string;
	areaName: string;
	areaLevel: number;
	lines: GuideLine[];
}

export interface GemPick {
	gemId: string;
	name: string;
	colour: "r" | "g" | "b" | "w";
	npc: string | null;
}

export interface GuidePayload {
	character: string | null;
	act: number;
	options: GuideOptions;
	position: number;
	total: number;
	current: GuideStep | null;
	upcoming: GuideStep[];
	actSteps: GuideStep[];
	actView: boolean;
	vendorRegex: string[];
	unobtainable: GemPick[];
	xp: XpView | null;
}

export interface XpView {
	areaName: string;
	monsterLevel: number;
	upcoming: boolean;
	playerLevel: number | null;
	multiplier: number | null;
	fullFrom: number;
	fullTo: number;
	cells: [number, number][];
}

export interface LevelRange {
	from: number;
	to: number | null;
}

export interface GemView {
	name: string;
	colour: "r" | "g" | "b" | "w";
	support: boolean;
	tooHigh: boolean;
}

export interface GroupView {
	label: string | null;
	skillSet: string | null;
	slot: string | null;
	levels: LevelRange | null;
	gems: GemView[];
}

export interface GemsPayload {
	character: string | null;
	level: number;
	hasBuild: boolean;
	skillSet: string | null;
	skillSets: string[];
	skillSetByHand: boolean;
	groups: GroupView[];
	visible: boolean;
}

export interface RegexCopied {
	text: string | null;
	part: number;
	of: number;
}

export interface BuildSummary {
	class: string;
	ascendancy: string | null;
	skillGroups: number;
	treeStages: number;
}

export type GuideCommand =
	| { kind: "next" }
	| { kind: "previous" }
	| { kind: "findMe" }
	| { kind: "jump"; step: number };

export type LabDifficulty = "normal" | "cruel" | "merciless" | "eternal";

export interface LabRoom {
	name: string;
	x: number;
	y: number;
	exits: number[];
	contents: {
		darkshrine: boolean;
		silverKey: boolean;
		silverDoor: boolean;
		argus: boolean;
		gauntlet: boolean;
		puzzle: boolean;
		raw: string[];
	};
	izaro: boolean;
}

export interface LabPayload {
	layout: {
		difficulty: LabDifficulty | null;
		date: string | null;
		rooms: LabRoom[];
	} | null;
	current: number | null;
	visited: number[];
	route: number[];
	darkshrineDetour: boolean;
	inLab: boolean;
	running: LabDifficulty | null;
	outdated: boolean;
	wrongDifficulty: boolean;
	visible: boolean;
}

export interface LabImportStatus {
	state: "watching" | "opened" | "loaded" | "failed" | "stopped";
	message: string;
}
