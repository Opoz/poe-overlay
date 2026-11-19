"""Render our own schematic zone layouts to SVG.

Each layout is described by hand in `act1.py` (and later acts): a few
simplified lines and markers on a 600x600 canvas, drawn from what the zone
looks like. This script turns those descriptions into SVGs in our own style.

Usage: python scripts/layouts/render.py <out dir>
"""

import importlib.util
import math
import pathlib
import sys

HERE = pathlib.Path(__file__).parent

BG = "#15191b"
EDGE = "#7d8a90"
FILL = "rgba(255,255,255,0.045)"
HINT = "#7fe0a8"
BONE = "#ede6d6"


def smooth(points, closed):
    """A path through the points with rounded corners (Catmull-Rom)."""
    if len(points) < 3:
        return "M" + " L".join(f"{x:.0f} {y:.0f}" for x, y in points)
    pts = points + points[:2] if closed else [points[0], *points, points[-1]]
    if closed:
        pts = [points[-1], *points, points[0], points[1]]
    d = f"M{pts[1][0]:.0f} {pts[1][1]:.0f}"
    for i in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[i - 1], pts[i], pts[i + 1], pts[i + 2]
        c1 = (p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6)
        c2 = (p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6)
        d += f" C{c1[0]:.0f} {c1[1]:.0f} {c2[0]:.0f} {c2[1]:.0f} {p2[0]:.0f} {p2[1]:.0f}"
    return d + (" Z" if closed else "")


def marker(kind, x, y, text=""):
    if kind == "exit":
        # A gold ring with an arrow: the way onward.
        return (
            f'<circle cx="{x}" cy="{y}" r="17" fill="{BG}" stroke="#f5cf5a" stroke-width="5"/>'
            f'<path d="M{x-7} {y+6} L{x+6} {y-7} M{x-3} {y-7} L{x+6} {y-7} L{x+6} {y+2}" '
            f'stroke="#f5cf5a" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>'
        )
    if kind == "side":
        # A side exit (another area, not the way onward).
        return (
            f'<circle cx="{x}" cy="{y}" r="14" fill="{BG}" stroke="#a8a294" stroke-width="4"/>'
            f'<circle cx="{x}" cy="{y}" r="4" fill="#a8a294"/>'
        )
    if kind == "start":
        return f'<circle cx="{x}" cy="{y}" r="11" fill="{HINT}" stroke="{BG}" stroke-width="4"/>'
    if kind == "wp":
        # Waypoint: a blue diamond.
        return (
            f'<path d="M{x} {y-18} L{x+18} {y} L{x} {y+18} L{x-18} {y} Z" '
            f'fill="{BG}" stroke="#8ad8ff" stroke-width="5" stroke-linejoin="round"/>'
            f'<circle cx="{x}" cy="{y}" r="4" fill="#8ad8ff"/>'
        )
    if kind == "quest":
        return (
            f'<circle cx="{x}" cy="{y}" r="16" fill="#f5cf5a"/>'
            f'<path d="M{x} {y-9} L{x} {y+2}" stroke="{BG}" stroke-width="5" stroke-linecap="round"/>'
            f'<circle cx="{x}" cy="{y+8}" r="3" fill="{BG}"/>'
        )
    if kind == "boss":
        return (
            f'<circle cx="{x}" cy="{y}" r="16" fill="#ff8070"/>'
            f'<path d="M{x-7} {y-7} L{x+7} {y+7} M{x+7} {y-7} L{x-7} {y+7}" '
            f'stroke="{BG}" stroke-width="5" stroke-linecap="round"/>'
        )
    if kind == "text":
        return (
            f'<text x="{x}" y="{y}" fill="{BONE}" font-family="Segoe UI, sans-serif" '
            f'font-size="40" font-weight="700" text-anchor="middle">{text}</text>'
        )
    raise ValueError(kind)


def svg(layout):
    parts = [
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 600" width="300" height="300">',
        f'<rect width="600" height="600" rx="28" fill="{BG}"/>',
    ]
    for line in layout.get("areas", []):
        parts.append(
            f'<path d="{smooth(line, True)}" fill="{FILL}" stroke="{EDGE}" '
            'stroke-width="6" stroke-linejoin="round"/>'
        )
    for line in layout.get("edges", []):
        parts.append(
            f'<path d="{smooth(line, False)}" fill="none" stroke="{BONE}" '
            'stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>'
        )
    for line in layout.get("hints", []):
        parts.append(
            f'<path d="{smooth(line, False)}" fill="none" stroke="{HINT}" '
            'stroke-width="7" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="2 14"/>'
        )
    for m in layout.get("marks", []):
        parts.append(marker(*m))
    parts.append("</svg>")
    return "".join(parts)


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.LAYOUTS


if __name__ == "__main__":
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "layouts")
    out.mkdir(parents=True, exist_ok=True)
    count = 0
    for act in ["act1"]:
        for name, layout in load(act).items():
            (out / f"{name}.svg").write_text(svg(layout), encoding="utf8")
            count += 1
    print(f"wrote {count} layouts to {out}")
