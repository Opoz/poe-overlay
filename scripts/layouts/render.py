"""Render our own schematic zone layouts to SVG.

Each layout is described by hand in `act1.py` (and later acts) as the
walkable paths through the zone on a 600x600 canvas:

- "route": the way from where you arrive to where you leave (dashed line),
- "branches": other paths worth knowing (side areas, dead ends, loops),
- "marks": what's where (start, exit, side, wp, quest, boss, text).

The zone itself is drawn as a corridor around the paths, so the outline can
never disagree with the route.

Arrival guides ("kind": "arrival") are different: they show what you see on
arriving, to tell a zone's variants apart. They're the landmark edges
("edges", the shoreline or wall, white) and the edge to follow ("follow",
green), with no corridor. `check()` fails if a marker isn't on a path
or anything leaves the canvas: rendering refuses broken layouts.

Usage: python scripts/layouts/render.py <out dir>
"""

import importlib.util
import math
import pathlib
import sys

HERE = pathlib.Path(__file__).parent

BG = "#15191b"
EDGE = "#6f7c82"
FLOOR = "#252c2f"
ROUTE = "#7fe0a8"
BONE = "#ede6d6"
WIDTH = 46  # corridor width, canvas units


def smooth(points):
    """An open path through the points with rounded corners (Catmull-Rom)."""
    if len(points) < 3:
        return "M" + " L".join(f"{x:.0f} {y:.0f}" for x, y in points)
    pts = [points[0], *points, points[-1]]
    d = f"M{pts[1][0]:.0f} {pts[1][1]:.0f}"
    for i in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[i - 1], pts[i], pts[i + 1], pts[i + 2]
        c1 = (p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6)
        c2 = (p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6)
        d += f" C{c1[0]:.0f} {c1[1]:.0f} {c2[0]:.0f} {c2[1]:.0f} {p2[0]:.0f} {p2[1]:.0f}"
    return d


def seg_dist(p, a, b):
    (px, py), (ax, ay), (bx, by) = p, a, b
    dx, dy = bx - ax, by - ay
    if dx == dy == 0:
        return math.hypot(px - ax, py - ay)
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def path_dist(p, paths):
    return min(
        (seg_dist(p, a, b) for path in paths for a, b in zip(path, path[1:])),
        default=math.inf,
    )


def paths_of(layout):
    route = layout.get("route")
    return ([route] if route else []) + layout.get("branches", [])


def check(name, layout):
    """Problems with a layout: markers off the paths, points off the canvas."""
    if layout.get("kind") == "arrival":
        return check_arrival(name, layout)
    problems = []
    width = layout.get("width", WIDTH)
    paths = paths_of(layout)
    for path in paths:
        if len(path) < 2:
            problems.append("a path needs at least two points")
        for x, y in path:
            if not (15 <= x <= 585 and 15 <= y <= 585):
                problems.append(f"path point {(x, y)} is off the canvas")
    for m in layout.get("marks", []):
        kind, x, y = m[0], m[1], m[2]
        if kind == "text":
            continue
        if not paths:
            problems.append(f"{kind} at {(x, y)} but there are no paths")
        elif path_dist((x, y), paths) > width / 2 + 6:
            problems.append(f"{kind} at {(x, y)} isn't on a path")
    return [f"{name}: {p}" for p in problems]


def check_arrival(name, layout):
    lines = layout.get("edges", []) + layout.get("follow", [])
    problems = []
    if not lines:
        problems.append("an arrival guide needs edges or a line to follow")
    for line in lines:
        for x, y in line:
            if not (10 <= x <= 590 and 10 <= y <= 590):
                problems.append(f"line point {(x, y)} is off the canvas")
    for m in layout.get("marks", []):
        if m[0] != "text" and path_dist((m[1], m[2]), lines) > 45:
            problems.append(f"{m[0]} at {(m[1], m[2])} is far from every line")
    return [f"{name}: {p}" for p in problems]


def svg_arrival(layout):
    parts = [
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 600" width="300" height="300">',
        f'<rect width="600" height="600" rx="28" fill="{BG}"/>',
    ]
    for line in layout.get("edges", []):
        parts.append(
            f'<path d="{smooth(line)}" fill="none" stroke="{BONE}" stroke-width="8" '
            'stroke-linecap="round" stroke-linejoin="round"/>'
        )
    for line in layout.get("follow", []):
        parts.append(
            f'<path d="{smooth(line)}" fill="none" stroke="{ROUTE}" stroke-width="8" '
            'stroke-linecap="round" stroke-linejoin="round"/>'
        )
    for m in layout.get("marks", []):
        parts.append(marker(*m))
    parts.append("</svg>")
    return "".join(parts)


def marker(kind, x, y, text=""):
    if kind == "exit":
        return (
            f'<circle cx="{x}" cy="{y}" r="19" fill="{BG}" stroke="#f5cf5a" stroke-width="5"/>'
            f'<path d="M{x-8} {y+7} L{x+7} {y-8} M{x-3} {y-8} L{x+7} {y-8} L{x+7} {y+2}" '
            f'stroke="#f5cf5a" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>'
        )
    if kind == "side":
        return (
            f'<circle cx="{x}" cy="{y}" r="16" fill="{BG}" stroke="#b8b2a4" stroke-width="4"/>'
            f'<circle cx="{x}" cy="{y}" r="5" fill="#b8b2a4"/>'
        )
    if kind == "start":
        return f'<circle cx="{x}" cy="{y}" r="13" fill="{ROUTE}" stroke="{BG}" stroke-width="4"/>'
    if kind == "wp":
        return (
            f'<path d="M{x} {y-19} L{x+19} {y} L{x} {y+19} L{x-19} {y} Z" '
            f'fill="{BG}" stroke="#8ad8ff" stroke-width="5" stroke-linejoin="round"/>'
            f'<circle cx="{x}" cy="{y}" r="4" fill="#8ad8ff"/>'
        )
    if kind == "quest":
        return (
            f'<circle cx="{x}" cy="{y}" r="17" fill="#f5cf5a" stroke="{BG}" stroke-width="3"/>'
            f'<path d="M{x} {y-9} L{x} {y+2}" stroke="{BG}" stroke-width="5" stroke-linecap="round"/>'
            f'<circle cx="{x}" cy="{y+8}" r="3" fill="{BG}"/>'
        )
    if kind == "boss":
        return (
            f'<circle cx="{x}" cy="{y}" r="17" fill="#ff8070" stroke="{BG}" stroke-width="3"/>'
            f'<path d="M{x-7} {y-7} L{x+7} {y+7} M{x+7} {y-7} L{x-7} {y+7}" '
            f'stroke="{BG}" stroke-width="5" stroke-linecap="round"/>'
        )
    if kind == "text":
        return (
            f'<text x="{x}" y="{y}" fill="{BONE}" font-family="Segoe UI, sans-serif" '
            f'font-size="38" font-weight="700" text-anchor="middle" '
            f'stroke="{BG}" stroke-width="8" paint-order="stroke">{text}</text>'
        )
    raise ValueError(kind)


def svg(layout):
    if layout.get("kind") == "arrival":
        return svg_arrival(layout)
    width = layout.get("width", WIDTH)
    paths = paths_of(layout)
    parts = [
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 600" width="300" height="300">',
        f'<rect width="600" height="600" rx="28" fill="{BG}"/>',
    ]
    # The zone: an outlined corridor around every path.
    for path in paths:
        parts.append(
            f'<path d="{smooth(path)}" fill="none" stroke="{EDGE}" stroke-width="{width + 10}" '
            'stroke-linecap="round" stroke-linejoin="round"/>'
        )
    for path in paths:
        parts.append(
            f'<path d="{smooth(path)}" fill="none" stroke="{FLOOR}" stroke-width="{width}" '
            'stroke-linecap="round" stroke-linejoin="round"/>'
        )
    route = layout.get("route")
    if route:
        parts.append(
            f'<path d="{smooth(route)}" fill="none" stroke="{ROUTE}" stroke-width="7" '
            'stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="2 15"/>'
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
    layouts = {k: v for act in ["act1"] for k, v in load(act).items()}
    problems = [p for k, v in layouts.items() for p in check(k, v)]
    if problems:
        print("\n".join(problems))
        sys.exit(f"{len(problems)} problems: nothing rendered")
    out.mkdir(parents=True, exist_ok=True)
    for old in out.glob("*.svg"):
        old.unlink()
    for name, layout in layouts.items():
        (out / f"{name}.svg").write_text(svg(layout), encoding="utf8")
    print(f"wrote {len(layouts)} layouts to {out}")
