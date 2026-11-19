import { invoke } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";
import { onEvent } from "../events";
import type { GuideCommand, GuidePayload } from "../types";

export function useGuide() {
	const [guide, setGuide] = createSignal<GuidePayload | null>(null);
	onEvent<GuidePayload>("guide-view", setGuide);
	invoke<GuidePayload>("get_guide_view").then(setGuide);
	return guide;
}

export const guideCommand = (command: GuideCommand) =>
	invoke("guide_command", { command });
export const toggleActView = () => invoke("toggle_act_view");
