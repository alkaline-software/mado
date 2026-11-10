#!/bin/zsh
# The README's moving picture, from scripts/readme/readme.html (a canvas
# drawing in emaki.lol's hand): every frame rendered by WebKit
# (scripts/readme/render.swift), then made into a GIF by ffmpeg with one
# palette for the whole loop, at .github/readme.gif. Runs on a Mac with
# ffmpeg; the faces come from Google Fonts, so it needs the network.
#
#   scripts/make-readme-gif.sh
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=.github/readme.gif
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
swiftc -O scripts/readme/render.swift -o "$TMP/render" 2>/dev/null
"$TMP/render" scripts/readme/readme.html "$TMP"
fps=30
ffmpeg -v error -y -framerate $fps -i "$TMP/frame-%04d.png" -vf "palettegen=max_colors=128:stats_mode=full" "$TMP/palette.png"
ffmpeg -v error -y -framerate $fps -i "$TMP/frame-%04d.png" -i "$TMP/palette.png" -lavfi "paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" -loop 0 "$OUT"
ls -la "$OUT"
