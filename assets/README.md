# shadowmask / assets

Shared image and vendored assets used by the client subprojects and by the project
README.

The files here are the single source of truth. Make any change here first, then
distribute it to the clients with `./refresh_assets.py`. Do not edit the copies
inside `clients/*`; they are generated from this directory and `--verify` will fail
if they drift.

Not everything here is distributed. `brand/shadowmask.logo-retro.svg` and everything
under `screenshots/` are referenced only by the root `README.md` and have no entry in
`refresh_assets.py`. `icons/flutter/linux_icon_*.png` are read straight from here by
`clients/flutter/AppImageBuilder.yml` when the AppImage is packaged.

## Layout

```
assets/
  brand/         brand mark; source for every derived client favicon/icon
  placeholders/  artwork fallbacks (poster, landscape, person)
  glyphs/        single-colour UI marks, tinted per theme by the client
  icons/         rasterized launcher/PWA icon sets, grouped per client
  vendor/        third-party files bundled verbatim into a client
  attribution/   provider logos we are required to display unmodified
  screenshots/   client captures for the root README
```

`attribution/` holds provider logos that must be shown unmodified in colour, aspect and rotation.
Replace them only with a fresh download from the provider.

- `attribution/tmdb.svg` - TMDB's "alt short" blue logo, from
  <https://www.themoviedb.org/about/logos-attribution>, byte-identical to their download.

  The basic client uses the SVG. Flutter and Roku use `icons/flutter/tmdb-logo.png` (480px) and
  `icons/roku/tmdb-logo.png` (224px), rendered by `render_icons.sh` width-only so the aspect ratio
  holds: the file takes its gradient from a `<style>` block, which `flutter_svg` ignores, rendering
  it black.

All four brand icons carry the same artwork: the retro mark, amber tube with RGB dots, on a warm
gradient plate running `#43331f` to `#1c140d`. They differ only in how the plate meets the canvas,
because each platform masks differently.

- `brand/shadowmask.logo.svg` - the brand mark on a rounded plate. The default, and the widest
  reach: both client favicons, the Flutter PWA `any` icons, the Linux hicolor sizes, and the
  Android legacy `ic_launcher` mipmaps used below API 26.
- `brand/shadowmask.icon-desktop.svg` - the same on a transparent canvas, art at 80.5% of the
  canvas and corner radius at 22.5% of the art, matching Apple's icon template. macOS applies no
  mask of its own, so the full-bleed version renders larger and squarer than its neighbours.
- `brand/shadowmask.icon-maskable.svg` - the same on a square full-bleed plate, with the mark held
  inside the 80% safe circle. The PWA maskable icons and the iOS `AppIcon` set. Opaque, so it meets
  the no-alpha rule for the 1024 iOS marketing icon; iOS applies its own squircle.
- `brand/shadowmask.icon-android.svg` - the mark alone, no plate, for the Android adaptive icon
  foreground. Android crops to the inner 66.7% of the canvas; the mark sits at 58% of that visible
  area, centered by the same rule as the others, `translate = 32 - 12 * scale`. The plate is the
  background layer, `ic_launcher_background.xml`. No `<monochrome>` layer.
The two plateless variants below are page logos, not icons. They sit on a themed page background
rather than a launcher or tab, so they carry no plate and take the theme's `accent`.

- `brand/shadowmask.logo-light.svg` - the mark on no plate, in the light theme's accent `#0e7d88`,
  dots at 0.55. The basic client's in-page nav logo. Single-accent dots, since the design system
  makes the RGB dots a retro-theme variant only. **This is a hand-kept port of what
  `brandMarkSvg` in `clients/flutter/lib/components/brand_mark.dart` generates at runtime**, which
  the basic client cannot call because it is Dart. Same `viewBox`, tube, dot grid and opacity, so
  the two clients render the same mark; change one and change the other.
- `brand/shadowmask.logo-retro.svg` - the same without a plate in the retro theme's colours.
  Currently unreferenced: the root README used it before switching to the desktop icon.
- `placeholders/{poster,landscape,person}.svg` - artwork fallbacks.
Everything under `glyphs/` is **pure white on transparent** and tinted at runtime through Roku's
`Poster.blendColor`, which multiplies: any colour baked into a source would survive the tint and come
out wrong in the other two themes. That is the rule for any glyph added here.

- `glyphs/watched-{disc,ring}.svg` - the watched marker on a card, for clients that cannot draw an
  icon font. The disc carries the check as a knocked-out hole rather than a stroke, so the tint
  colours the disc and the check shows the artwork behind it. The ring is the separating halo the
  design system asks for, tinted `surface` so the marker reads over any poster.
- `glyphs/art-{movie,landscape,person}.svg` - artwork placeholders, tinted `border` and centred on
  `art-bg`. These match what the Flutter client draws (a centred icon, `card_art.dart`) rather than
  the full-bleed panels under `placeholders/`, which belong to the basic client. Authored here rather
  than taken from Material Icons, so there is no third-party licence to carry for three outlines.
- `glyphs/chevron-{left,right,up,down}.svg` - directional marks. The left/right pair does double
  duty: the rail scroll arrows (ported from `_RailArrow` in `card_rail.dart`) and the nav-rail
  indicator, which shows `<` when the menu is closed and `>` when it is open. The up/down pair is
  used for step buttons.
- `glyphs/button-{fill,line}.9.svg` - the rounded-rectangle control shape, as **nine-patch** sources.
  SceneGraph's `Rectangle` has no corner radius, so every button is two tinted `Poster` nodes: the
  fill and the 3px outline, each stretched by Roku's nine-patch rules. Authored at exactly 24×24 so
  `rsvg-convert` maps 1:1 to pixels, with a 22×22 interior at radius 10 (the design system's `sm`
  radius of 6 at the Roku canvas's 1.75× scale) and a 1px border carrying **pure opaque black**
  stretch markers, 2px wide, dead centre on the top and left edges. The marker rects are integer
  aligned so they cannot antialias: a grey marker pixel silently disables stretching. These are the
  only `.9.png` files we ship; a *half*-rounded segment needs no extra asset, because `RoundedBox`
  oversizes the same image inside a clipping rect.
- `glyphs/chip-cap-{left,right}-line.svg` - the outline halves of a chip, a 3px stroked arc with no
  fill, sitting over the solid caps below. The straight top and bottom of the stroke are `Rectangle`
  nodes in the client, so only the curved ends need artwork.
- `glyphs/icon-shuffle.svg` - the Random action on a series or season, two crossed arrows. Authored
  here rather than taken from Material Icons, for the same licence reason as the `art-*` glyphs.
- `glyphs/icon-{sort,filter,library}.svg` - the marks on the Roku browse toolbar chips: three
  descending bars for the sort, a funnel for the genre filter and a folder for the library filter.
  The order chip takes `chevron-up` or `chevron-down` depending on which way it runs, and the random
  chip takes `icon-shuffle`, so those two need no artwork of their own. Authored here for the same
  licence reason as the rest.
- `glyphs/icon-{check,close,play,search,bookmark,bookmark-on,heart,heart-on}.svg` - the control
  icons behind the Roku action bar. The `-on` variants are the filled forms, matching the
  `icon` / `filledIcon` pair in `toggle_button.dart`: outline when the toggle is off, filled when it
  is on. **Roku ships no icon set**: its `common:/images/` has only dialog and field 9-patches, the
  four focus bitmaps, the Options-key icon, a generic placeholder and two player images, so these
  are ours. An icon font would have been one file instead of eight and was rejected for the same
  reason as the `art-*` glyphs below: no third-party licence to carry.
- `glyphs/disc.svg` - a plain filled circle, tinted and used as the plate behind a chevron.
- `glyphs/chip-cap-{left,right}.svg` - the rounded ends of a chip. SceneGraph's `Rectangle` has no
  corner radius and stretching a pill PNG distorts its caps, so a chip is drawn as cap, middle
  `Rectangle`, cap.
- `glyphs/hex-texture.svg` - the design system's §2.9 shadow-mask hex lattice, 14px cell spacing, as
  an SVG `<pattern>` filling a 1920×1080 canvas. Roku has no canvas API and `Poster` cannot tile, so
  it renders to one full-screen PNG; the pattern is sparse enough that the result is ~22KB. Tinted
  `text` at 6% opacity by the client.
- `icons/flutter/{favicon,Icon-*}.png` - the Flutter web favicon and PWA icons.
- `icons/flutter/app_icon_*.png` - the Flutter macOS app icon set. Filenames match the
  ones the appiconset's `Contents.json` references.
- `icons/flutter/ios_icon_*.png` - the Flutter iOS app icon set, named by pixel size. Several of
  the appiconset's point-and-scale entries share a pixel size and take the same render.
- `icons/flutter/android_icon_*.png` - the Android legacy `ic_launcher` mipmaps, 48 through 192.
- `icons/flutter/android_icon_fg_*.png` - the Android adaptive icon foreground, 108 through 432.
- `icons/flutter/linux_icon_*.png` - the hicolor sizes for the Linux AppImage, rendered full-bleed
  from `brand/shadowmask.logo.svg`. Not distributed by `refresh_assets.py`;
  `clients/flutter/AppImageBuilder.yml` reads them from here at package time.
- `vendor/hls.min.js` - hls.js, bundled by the web clients for HLS playback
  (Apache-2.0). Replace with a newer upstream `hls.min.js` here, then refresh.
- `vendor/hls.js.LICENSE.txt` - the Apache-2.0 text and copyright notices for the
  bundle above. It is distributed alongside `hls.min.js` so the licence travels
  with the redistributed file into both container images. Update it whenever the
  bundle is replaced.
- `screenshots/*.jpg` - Flutter web client captures used by the root `README.md`.
  Not distributed to any client. JPEG rather than PNG on purpose: the same set as
  lossless PNG was 17 MB, which is a lot to carry in history for images GitHub
  renders at under 900px.

## Rendering

The PNGs under `icons/` are generated from the SVGs under `brand/` by
`./render_icons.sh`, which needs `rsvg-convert` from librsvg: `librsvg2-bin` on Debian
and Ubuntu, `librsvg2-tools` on Fedora, `librsvg` on Arch or via Homebrew. Run it when a
brand SVG changes, then refresh.

```
./render_icons.sh              # brand SVGs -> icons/flutter/*.png
```

## Usage

```
./refresh_assets.py            # distribute all assets to all clients
./refresh_assets.py -p clients/basic
./refresh_assets.py -p clients/flutter
./refresh_assets.py --verify   # check that distributed copies match the source (CI drift)
./refresh_assets.py -v         # debug logging
```

## Targets

| Project | Source | Target |
|---|---|---|
| `clients/basic` | `brand/shadowmask.logo.svg` | `favicon.svg` |
| `clients/basic` | `brand/shadowmask.logo-light.svg` | `logo.svg` |
| `clients/basic` | `placeholders/poster.svg` | `placeholder.svg` |
| `clients/basic` | `placeholders/landscape.svg` | `placeholder-landscape.svg` |
| `clients/basic` | `placeholders/person.svg` | `placeholder-person.svg` |
| `clients/basic` | `vendor/hls.min.js` | `vendor/hls.min.js` |
| `clients/flutter` | `icons/flutter/favicon.png` | `web/favicon.png` |
| `clients/flutter` | `icons/flutter/Icon-192.png` | `web/icons/Icon-192.png` |
| `clients/flutter` | `icons/flutter/Icon-512.png` | `web/icons/Icon-512.png` |
| `clients/flutter` | `icons/flutter/Icon-maskable-192.png` | `web/icons/Icon-maskable-192.png` |
| `clients/flutter` | `icons/flutter/Icon-maskable-512.png` | `web/icons/Icon-maskable-512.png` |
| `clients/flutter` | `icons/flutter/app_icon_{16,32,64,128,256,512,1024}.png` | `macos/Runner/Assets.xcassets/AppIcon.appiconset/` |
| `clients/flutter` | `icons/flutter/ios_icon_*.png` | `ios/Runner/Assets.xcassets/AppIcon.appiconset/` |
| `clients/flutter` | `icons/flutter/android_icon_{48,72,96,144,192}.png` | `android/app/src/main/res/mipmap-*/ic_launcher.png` |
| `clients/flutter` | `icons/flutter/android_icon_fg_{108,162,216,324,432}.png` | `android/app/src/main/res/mipmap-*/ic_launcher_foreground.png` |
| `clients/flutter` | `vendor/hls.min.js` | `web/hls.min.js` |
| `clients/roku` | `fonts/Roboto-{Regular,Medium}.ttf` | `fonts/` |
| `clients/roku` | `fonts/Roboto-OFL.txt` | `fonts/Roboto-OFL.txt` |
| `clients/roku` | `icons/roku/channel-poster-{hd,fhd}.png` | `images/` |
| `clients/roku` | `icons/roku/splash-{sd,hd,fhd}.png` | `images/` |
| `clients/roku` | `icons/roku/brand-mark.png` | `images/brand-mark.png` |
| `clients/roku` | `icons/roku/art-{movie,landscape,person}.png` | `images/` |
| `clients/roku` | `icons/roku/watched-{disc,ring}.png` | `images/` |
| `clients/roku` | `icons/roku/hex-texture.png` | `images/hex-texture.png` |
| `clients/roku` | `icons/roku/chevron-{left,right,up,down}.png` | `images/` |
| `clients/roku` | `icons/roku/icon-{check,close,play,search}.png` | `images/` |
| `clients/roku` | `icons/roku/icon-{bookmark,bookmark-on,heart,heart-on}.png` | `images/` |
| `clients/roku` | `icons/roku/icon-shuffle.png` | `images/icon-shuffle.png` |
| `clients/roku` | `icons/roku/chip-cap-{left,right}.png` | `images/` |
| `clients/roku` | `icons/roku/chip-cap-{left,right}-line.png` | `images/` |
| `clients/roku` | `icons/roku/button-{fill,line}.9.png` | `images/` |
| `clients/roku` | `icons/roku/disc.png` | `images/disc.png` |

Roku takes PNG only - there is no SVG support on the platform, so every Roku target is a render
rather than a copy of the source SVG.

The `.9.png` suffix is load-bearing: Roku only applies nine-patch stretching to files named that way,
so the suffix has to survive rendering and distribution. `render_icons.sh` writes it and
`refresh_assets.py` copies it verbatim.
