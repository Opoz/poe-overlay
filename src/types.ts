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
	calibrating: Calibrating | null;
	transform: Transform | null;
	character: string | null;
	classStart: TreeNode | null;
	partner: TreeNode | null;
	allocated: number[];
	masteries: { node: number; option: number }[];
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
	lines: GuideLine[];
}

export interface GuidePayload {
	character: string | null;
	options: GuideOptions;
	position: number;
	total: number;
	current: GuideStep | null;
	upcoming: GuideStep[];
	actSteps: GuideStep[];
	actView: boolean;
}

export type GuideCommand = "next" | "previous" | "findMe";
