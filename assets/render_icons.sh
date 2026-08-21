#!/usr/bin/env sh
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
brand="$here/brand"
out="$here/icons/flutter"

rsvg-convert -w 32  -h 32  "$brand/shadowmask.logo.svg"          -o "$out/favicon.png"
rsvg-convert -w 192 -h 192 "$brand/shadowmask.logo.svg"          -o "$out/Icon-192.png"
rsvg-convert -w 512 -h 512 "$brand/shadowmask.logo.svg"          -o "$out/Icon-512.png"
rsvg-convert -w 192 -h 192 "$brand/shadowmask.icon-maskable.svg" -o "$out/Icon-maskable-192.png"
rsvg-convert -w 512 -h 512 "$brand/shadowmask.icon-maskable.svg" -o "$out/Icon-maskable-512.png"

echo "Rendered brand icons into $out. Run refresh_assets.py to distribute."
