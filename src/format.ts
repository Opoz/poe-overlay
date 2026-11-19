/** `1:02:03`, or `2:03` under an hour. */
export function formatDuration(ms: number): string {
	const total = Math.max(0, Math.floor(ms / 1000));
	const h = Math.floor(total / 3600);
	const m = Math.floor((total % 3600) / 60);
	const s = total % 60;
	const ss = String(s).padStart(2, "0");
	return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** Signed difference against a comparison: `-0:21`, `+1:05`. */
export function formatDelta(ms: number): string {
	const sign = ms < 0 ? "−" : "+";
	return `${sign}${formatDuration(Math.abs(ms))}`;
}

/** Local-wall-clock ms (the Client Log's scale) as a readable date. */
export function formatLocalDate(localMs: number): string {
	// Stored as local time encoded as if it were UTC.
	const d = new Date(localMs);
	const pad = (n: number) => String(n).padStart(2, "0");
	return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;
}

/**
 * Capitalise each word ("estate path" -> "Estate Path"), but not after an
 * apostrophe ("Aspirant's Trial").
 */
export function titleCase(s: string): string {
	return s.replace(
		/(^|[\s-])(\p{L})/gu,
		(_, gap: string, c: string) => gap + c.toUpperCase(),
	);
}

const KEY_NAMES: Record<string, string> = {
	BracketLeft: "[",
	BracketRight: "]",
	Backquote: "`",
	Minus: "-",
	Equal: "=",
	Semicolon: ";",
	Quote: "'",
	Comma: ",",
	Period: ".",
	Slash: "/",
	Backslash: "\\",
};

/** A shortcut as the backend names it (`alt+shift+KeyS`) for people: `Alt+Shift+S`. */
export function formatShortcut(s: string): string {
	return s
		.split("+")
		.map((part) => {
			if (/^Key[A-Z]$/.test(part)) return part.slice(3);
			if (/^Digit\d$/.test(part)) return part.slice(5);
			if (part in KEY_NAMES) return KEY_NAMES[part];
			return part.charAt(0).toUpperCase() + part.slice(1);
		})
		.join("+");
}
