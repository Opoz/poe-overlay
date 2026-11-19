import { invoke } from "@tauri-apps/api/core";
import { createSignal, Show } from "solid-js";

type Box = { x0: number; y0: number; x1: number; y1: number };

/**
 * Covers the game while a screen copy's source is picked: drag a box around
 * what to copy; right-click cancels.
 */
export default function ClonePicker() {
	const [box, setBox] = createSignal<Box | null>(null);
	const rect = () => {
		const b = box();
		if (!b) return null;
		return {
			left: Math.min(b.x0, b.x1),
			top: Math.min(b.y0, b.y1),
			width: Math.abs(b.x1 - b.x0),
			height: Math.abs(b.y1 - b.y0),
		};
	};

	const down = (e: MouseEvent) => {
		if (e.button === 2) {
			setBox(null);
			invoke("clone_pick_cancel");
			return;
		}
		if (e.button === 0)
			setBox({ x0: e.clientX, y0: e.clientY, x1: e.clientX, y1: e.clientY });
	};
	const move = (e: MouseEvent) => {
		const b = box();
		if (b) setBox({ ...b, x1: e.clientX, y1: e.clientY });
	};
	const up = () => {
		const r = rect();
		setBox(null);
		if (!r || r.width < 4 || r.height < 4) return;
		const [w, h] = [window.innerWidth, window.innerHeight];
		invoke("clone_pick", {
			x: r.left / w,
			y: r.top / h,
			width: r.width / w,
			height: r.height / h,
		});
	};

	return (
		// biome-ignore lint/a11y/noStaticElementInteractions: a drag surface over the game
		<div
			class="clone-picker"
			onMouseDown={down}
			onMouseMove={move}
			onMouseUp={up}
			onContextMenu={(e) => e.preventDefault()}
		>
			<div class="tree-prompt">
				<strong>New screen copy:</strong> drag a box around what to copy.
				Right-click cancels.
			</div>
			<Show when={rect()}>
				{(r) => (
					<div
						class="clone-box"
						style={{
							left: `${r().left}px`,
							top: `${r().top}px`,
							width: `${r().width}px`,
							height: `${r().height}px`,
						}}
					/>
				)}
			</Show>
		</div>
	);
}
