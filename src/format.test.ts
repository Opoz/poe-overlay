import { describe, expect, it } from "vitest";
import {
	formatDelta,
	formatDuration,
	formatLocalDate,
	formatShortcut,
	titleCase,
} from "./format";

describe("formatDuration", () => {
	it("formats under an hour as m:ss", () => {
		expect(formatDuration(0)).toBe("0:00");
		expect(formatDuration(65_400)).toBe("1:05");
	});
	it("formats hours as h:mm:ss", () => {
		expect(formatDuration(3_723_000)).toBe("1:02:03");
	});
	it("clamps negatives to zero", () => {
		expect(formatDuration(-5000)).toBe("0:00");
	});
});

describe("formatDelta", () => {
	it("signs ahead and behind", () => {
		expect(formatDelta(-21_000)).toBe("−0:21");
		expect(formatDelta(65_000)).toBe("+1:05");
	});
});

describe("formatLocalDate", () => {
	it("reads local-as-UTC ms", () => {
		expect(formatLocalDate(Date.UTC(2026, 10, 18, 1, 36))).toBe(
			"2026-11-18 01:36",
		);
	});
});

describe("titleCase", () => {
	it("capitalises words but not after an apostrophe", () => {
		expect(titleCase("aspirant's trial")).toBe("Aspirant's Trial");
		expect(titleCase("estate path")).toBe("Estate Path");
	});
});

describe("formatShortcut", () => {
	it("names keys the way players read them", () => {
		expect(formatShortcut("alt+KeyR")).toBe("Alt+R");
		expect(formatShortcut("alt+shift+BracketLeft")).toBe("Alt+Shift+[");
		expect(formatShortcut("control+Digit1")).toBe("Control+1");
		expect(formatShortcut("F5")).toBe("F5");
	});
});
