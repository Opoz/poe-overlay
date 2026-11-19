// Small line icons for Overlay controls, drawn for this app (24px grid,
// stroked with currentColor). Emoji render differently per machine.

import type { JSX } from "solid-js";

function Icon(props: { children: JSX.Element; label?: string }) {
	return (
		<svg
			class="icon"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2.2"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden={props.label ? undefined : "true"}
			role={props.label ? "img" : undefined}
			aria-label={props.label}
		>
			{props.children}
		</svg>
	);
}

export const PrevIcon = () => (
	<Icon>
		<path d="M15 5 8 12l7 7" />
	</Icon>
);

export const NextIcon = () => (
	<Icon>
		<path d="m9 5 7 7-7 7" />
	</Icon>
);

/** Find me: a location marker. */
export const FindIcon = () => (
	<Icon>
		<circle cx="12" cy="12" r="6.5" />
		<path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3" />
		<circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none" />
	</Icon>
);

/** Vendor search: a magnifier. */
export const SearchIcon = () => (
	<Icon>
		<circle cx="10.5" cy="10.5" r="6" />
		<path d="m15 15 5.5 5.5" />
	</Icon>
);

/** The full act: a list. */
export const ListIcon = () => (
	<Icon>
		<path d="M9 6h11M9 12h11M9 18h11" />
		<path d="M4.5 6h.01M4.5 12h.01M4.5 18h.01" stroke-width="3" />
	</Icon>
);

export const CloseIcon = () => (
	<Icon>
		<path d="m6 6 12 12M18 6 6 18" />
	</Icon>
);

export const RefreshIcon = () => (
	<Icon>
		<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3" />
		<path d="M19.5 4v4.5H15" />
	</Icon>
);
