#!/usr/bin/env sh
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
brand="$here/brand"
placeholders="$here/placeholders"
glyphs="$here/glyphs"
attribution="$here/attribution"
out="$here/icons/flutter"
roku="$here/icons/roku"

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

mkdir -p "$roku"

rsvg-convert -w 290  -h 218  "$brand/shadowmask.channel-poster.svg" -o "$roku/channel-poster-hd.png"
rsvg-convert -w 540  -h 405  "$brand/shadowmask.channel-poster.svg" -o "$roku/channel-poster-fhd.png"
rsvg-convert -w 720  -h 480  "$brand/shadowmask.splash-sd.svg"      -o "$roku/splash-sd.png"
rsvg-convert -w 1280 -h 720  "$brand/shadowmask.splash-wide.svg"    -o "$roku/splash-hd.png"
rsvg-convert -w 1920 -h 1080 "$brand/shadowmask.splash-wide.svg"    -o "$roku/splash-fhd.png"
rsvg-convert -w 128  -h 128  "$brand/shadowmask.mark-white.svg"     -o "$roku/brand-mark.png"

rsvg-convert -w 224 "$attribution/tmdb.svg" -o "$roku/tmdb-logo.png"
rsvg-convert -w 480 "$attribution/tmdb.svg" -o "$out/tmdb-logo.png"


rsvg-convert -w 48 -h 48 "$glyphs/watched-disc.svg" -o "$roku/watched-disc.png"
rsvg-convert -w 56 -h 56 "$glyphs/watched-ring.svg" -o "$roku/watched-ring.png"
rsvg-convert -w 1920 -h 1080 "$glyphs/hex-texture.svg" -o "$roku/hex-texture.png"

rsvg-convert -w 96 -h 96 "$glyphs/art-movie.svg"     -o "$roku/art-movie.png"
rsvg-convert -w 96 -h 96 "$glyphs/art-landscape.svg" -o "$roku/art-landscape.png"
rsvg-convert -w 96 -h 96 "$glyphs/art-person.svg"    -o "$roku/art-person.png"

for chevron in left right up down; do
    rsvg-convert -w 44 -h 44 "$glyphs/chevron-$chevron.svg" -o "$roku/chevron-$chevron.png"
done

for icon in check close play pause previous next subtitles audio quality settings replay search bookmark bookmark-on heart heart-on shuffle trash sort filter library; do
    rsvg-convert -w 40 -h 40 "$glyphs/icon-$icon.svg" -o "$roku/icon-$icon.png"
done

rsvg-convert -w 96 -h 96 "$glyphs/icon-play.svg"   -o "$roku/icon-play-large.png"
rsvg-convert -w 96 -h 96 "$glyphs/icon-replay.svg" -o "$roku/icon-replay-large.png"

rsvg-convert -w 24 -h 48 "$glyphs/chip-cap-left.svg"       -o "$roku/chip-cap-left.png"
rsvg-convert -w 24 -h 48 "$glyphs/chip-cap-right.svg"      -o "$roku/chip-cap-right.png"
rsvg-convert -w 24 -h 48 "$glyphs/chip-cap-left-line.svg"  -o "$roku/chip-cap-left-line.png"
rsvg-convert -w 24 -h 48 "$glyphs/chip-cap-right-line.svg" -o "$roku/chip-cap-right-line.png"
rsvg-convert -w 72 -h 72 "$glyphs/disc.svg"                -o "$roku/disc.png"

rsvg-convert -w 24 -h 24 "$glyphs/button-fill.9.svg" -o "$roku/button-fill.9.png"
rsvg-convert -w 24 -h 24 "$glyphs/button-line.9.svg" -o "$roku/button-line.9.png"

echo "Rendered brand icons into $out and $roku. Run refresh_assets.py to distribute."
