import { invoke } from "@tauri-apps/api/core";
import { type Accessor, createSignal, onCleanup } from "solid-js";
import { onEvent } from "./events";
import type { TimerView } from "./types";

export interface LiveTimer {
	view: Accessor<TimerView | null>;
	/** Elapsed times, counted forward locally between backend updates. */
	realMs: Accessor<number>;
	loadRemovedMs: Accessor<number>;
}

export function useTimerView(): LiveTimer {
	const [view, setView] = createSignal<TimerView | null>(null);
	const [receivedAt, setReceivedAt] = createSignal(performance.now());
	const [now, setNow] = createSignal(performance.now());

	const accept = (v: TimerView) => {
		setView(v);
		setReceivedAt(performance.now());
		setNow(performance.now());
	};

	onEvent<TimerView>("timer-view", accept);
	invoke<TimerView>("get_timer_view").then(accept);

	const tick = setInterval(() => setNow(performance.now()), 100);
	onCleanup(() => clearInterval(tick));

	const extra = () =>
		view()?.phase?.phase === "running" ? now() - receivedAt() : 0;
	return {
		view,
		realMs: () => (view()?.realMs ?? 0) + extra(),
		loadRemovedMs: () => (view()?.loadRemovedMs ?? 0) + extra(),
	};
}
