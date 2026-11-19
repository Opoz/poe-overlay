/**
 * The app's mark: a route climbing through a diamond, ending at a waymark.
 * Drawn for this project (also the source of the app icon).
 */
export default function Mark() {
	return (
		<svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
			<path
				d="M12 2.2 21.8 12 12 21.8 2.2 12Z"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linejoin="round"
			/>
			<path
				d="M6.6 14.6 10 11.6l2.6 2.2 4-4.2"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
				stroke-linejoin="round"
				opacity="0.75"
			/>
			<circle cx="16.8" cy="9.4" r="1.9" fill="currentColor" />
		</svg>
	);
}
