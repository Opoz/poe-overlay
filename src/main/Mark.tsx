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
				d="M8.2 13.7 10.7 11.5l2.2 1.9 2.3-2.8"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
				stroke-linejoin="round"
				opacity="0.75"
			/>
			<circle cx="15.2" cy="10.6" r="1.5" fill="currentColor" />
		</svg>
	);
}
