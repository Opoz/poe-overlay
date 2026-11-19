import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { createSignal, Show } from "solid-js";

/**
 * Checks GitHub releases once at start. A new version is offered, never
 * installed without the player clicking.
 */
export default function UpdateNotice() {
	const [update, setUpdate] = createSignal<Update | null>(null);
	const [state, setState] = createSignal<"idle" | "installing" | "failed">(
		"idle",
	);
	if (!import.meta.env.DEV) {
		check()
			.then(setUpdate)
			.catch(() => {
				// Offline or no release yet: say nothing.
			});
	}

	const install = async () => {
		const u = update();
		if (!u) return;
		setState("installing");
		try {
			await u.downloadAndInstall();
			await relaunch();
		} catch {
			setState("failed");
		}
	};

	return (
		<Show when={update()}>
			{(u) => (
				<div class="update">
					<strong>Version {u().version} is out</strong>
					<button
						type="button"
						class="primary small"
						disabled={state() === "installing"}
						onClick={install}
					>
						{state() === "installing" ? "Installing…" : "Install and restart"}
					</button>
					<Show when={state() === "failed"}>
						<span class="warn-text">Couldn't install. Try again later.</span>
					</Show>
				</div>
			)}
		</Show>
	);
}
