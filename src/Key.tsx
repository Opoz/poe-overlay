import { Show } from "solid-js";
import { keyFor } from "./hotkeys";

/** The current shortcut for a hotkey action as a keycap; nothing if unbound. */
export default function Key(props: { id: string }) {
	return <Show when={keyFor(props.id)}>{(k) => <kbd>{k()}</kbd>}</Show>;
}
