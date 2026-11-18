import { listen } from "@tauri-apps/api/event";
import { onCleanup } from "solid-js";

/** Subscribe to a backend event for the lifetime of the current component. */
export function onEvent<T>(name: string, handler: (payload: T) => void): void {
	const unlisten = listen<T>(name, (e) => handler(e.payload));
	onCleanup(() => {
		unlisten.then((stop) => stop());
	});
}
