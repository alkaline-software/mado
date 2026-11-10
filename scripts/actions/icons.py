#!/usr/bin/env python3
"""Each action's picture: a small screen, 24 by 16, its frame a line and
the part the action puts a window in filled (each window's part, for a
tiling), in Tabler's hand (round ends,
a 1.5 line), one color the window tints. Written to
crates/mado-app/assets/icons/actions/<key>.svg, for the settings' rows and
the menu bar item's menu. Run it again when an action is added; it
writes the pictures of the actions there are and deletes the rest.

    python3 scripts/actions/icons.py
"""
import os

OUT = os.path.join(os.path.dirname(__file__), "..", "..", "crates", "mado-app", "assets", "icons", "actions")
# The screen's inside, where a part is drawn.
X0, Y0, W, H = 3.0, 3.0, 18.0, 10.0

def frac(x, y, w, h):
    return (x, y, w, h)

T = 1 / 3
PARTS = {
    "left-half": [frac(0, 0, .5, 1)], "right-half": [frac(.5, 0, .5, 1)],
    "top-half": [frac(0, 0, 1, .5)], "bottom-half": [frac(0, .5, 1, .5)],
    "top-left": [frac(0, 0, .5, .5)], "top-right": [frac(.5, 0, .5, .5)], "bottom-left": [frac(0, .5, .5, .5)], "bottom-right": [frac(.5, .5, .5, .5)],
    "maximize": [frac(0, 0, 1, 1)], "almost-maximize": [frac(.1, .12, .8, .76)], "maximize-height": [frac(.33, 0, .34, 1)],
    "center": [frac(.3, .25, .4, .5)],
    # The tilings: every window on the screen, a part each.
    "tile-rows": [frac(0, k * T, 1, T) for k in range(3)],
    "tile-columns": [frac(k * T, 0, T, 1) for k in range(3)],
    "tile-grid": [frac(x, y, .5, .5) for y in (0, .5) for x in (0, .5)],
}
# The ones that are not a part: a sign in the middle of the screen.
SIGNS = {
    "larger": '<path d="M12 5.5v5M9.5 8h5" />',
    "smaller": '<path d="M9.5 8h5" />',
    # The circular arrow the window's Restore Defaults and the hero draw
    # (Tabler's undo, icons/undo.svg), at 0.45 of its size.
    "restore": '<path d="M15.58 7.55a3.60 3.60 0 1 0 -0.225 1.80m0.225 2.25v-2.25h-2.25" />',
    "next-display": '<path d="M10.5 5.5l2.5 2.5l-2.5 2.5" />',
    "previous-display": '<path d="M13.5 5.5l-2.5 2.5l2.5 2.5" />',
}

HEAD = '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="16" viewBox="0 0 24 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">'
FRAME = '<rect x="1.75" y="1.75" width="20.5" height="12.5" rx="2.5" />'

def svg(key):
    if key in SIGNS:
        inner = SIGNS[key]
    else:
        inner = ""
        # A hair inside the screen, so neighbors read apart; a tiling's
        # windows a little more, so each reads as one.
        e = .3 if len(PARTS[key]) == 1 else .5
        for x, y, w, h in PARTS[key]:
            px, py, pw, ph = X0 + W * x + e, Y0 + H * y + e, W * w - 2 * e, H * h - 2 * e
            inner += f'<rect x="{px:.2f}" y="{py:.2f}" width="{pw:.2f}" height="{ph:.2f}" rx=".75" fill="currentColor" stroke="none" />'
    return HEAD + FRAME + inner + "</svg>\n"

os.makedirs(OUT, exist_ok=True)
for old in os.listdir(OUT):
    if old.endswith(".svg") and old[:-4] not in PARTS and old[:-4] not in SIGNS:
        os.remove(os.path.join(OUT, old))
for key in list(PARTS) + list(SIGNS):
    with open(os.path.join(OUT, key + ".svg"), "w") as f:
        f.write(svg(key))
print(len(PARTS) + len(SIGNS), "pictures in", os.path.normpath(OUT))
