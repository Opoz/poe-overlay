// Mirrors of the Rust types sent over IPC (crates/poe-core, src-tauri).

export type PauseReason = "manual" | "login" | "hideout";

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
