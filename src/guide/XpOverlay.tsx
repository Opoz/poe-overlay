import { onMount, Show } from "solid-js";
import { autoSizeWindow } from "../autoSize";
import { useGuide } from "./useGuide";
import XpRuler from "./XpRuler";

/** The XP level ruler on its own (Alt+X), for the guide's current zone. */
export default function XpOverlay() {
	const guide = useGuide();
	return (
		<div
			class="overlay xp-overlay"
			ref={(el) => onMount(() => autoSizeWindow(el))}
		>
			<Show
				when={guide()?.xp}
				fallback={<span class="muted">XP: enter a campaign zone</span>}
			>
				{(xp) => <XpRuler xp={xp()} />}
			</Show>
		</div>
	);
}
