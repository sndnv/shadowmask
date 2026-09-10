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
  icons/         rasterized launcher/PWA icon sets, grouped per client
  vendor/        third-party files bundled verbatim into a client
  screenshots/   client captures for the root README
```

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

As the Roku client lands, add its launcher and icon targets here.
