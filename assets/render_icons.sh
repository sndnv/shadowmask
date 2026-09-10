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

for size in 16 32 64 128 256 512 1024; do
    rsvg-convert -w "$size" -h "$size" "$brand/shadowmask.icon-desktop.svg" -o "$out/app_icon_$size.png"
done

for size in 64 128 256 512; do
    rsvg-convert -w "$size" -h "$size" "$brand/shadowmask.logo.svg" -o "$out/linux_icon_$size.png"
done

for size in 48 72 96 144 192; do
    rsvg-convert -w "$size" -h "$size" "$brand/shadowmask.logo.svg" -o "$out/android_icon_$size.png"
done

for size in 108 162 216 324 432; do
    rsvg-convert -w "$size" -h "$size" "$brand/shadowmask.icon-android.svg" -o "$out/android_icon_fg_$size.png"
done

for size in 20 29 40 58 60 76 80 87 120 152 167 180 1024; do
    rsvg-convert -w "$size" -h "$size" "$brand/shadowmask.icon-maskable.svg" -o "$out/ios_icon_$size.png"
done

echo "Rendered brand icons into $out. Run refresh_assets.py to distribute."
