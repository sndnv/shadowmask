# shadowmask / assets

Shared image and vendored assets used by the client subprojects and by the project
README.

The files here are the single source of truth. Make any change here first, then
distribute it to the clients with `./refresh_assets.py`. Do not edit the copies
inside `clients/*`; they are generated from this directory and `--verify` will fail
if they drift.

Not everything here is distributed. `brand/shadowmask.logo-retro.svg` and everything
under `screenshots/` are referenced only by the root `README.md` and have no entry in
`refresh_assets.py`.

## Layout

```
assets/
  brand/         brand mark; source for every derived client favicon/icon
  placeholders/  artwork fallbacks (poster, landscape, person)
  icons/         rasterized launcher/PWA icon sets, grouped per client
  vendor/        third-party files bundled verbatim into a client
  screenshots/   client captures for the root README
```

- `brand/shadowmask.logo.svg` - the brand mark.
- `brand/shadowmask.logo-retro.svg` - the brand mark without its background plate,
  in the retro theme's colours. Root README only.
- `placeholders/{poster,landscape,person}.svg` - artwork fallbacks.
- `icons/flutter/*.png` - the Flutter web favicon and PWA icons.
- `vendor/hls.min.js` - hls.js, bundled by the web clients for HLS playback
  (Apache-2.0). Replace with a newer upstream `hls.min.js` here, then refresh.
- `vendor/hls.js.LICENSE.txt` - the Apache-2.0 text and copyright notices for the
  bundle above. It is distributed alongside `hls.min.js` so the licence travels
  with the redistributed file into both container images. Update it whenever the
  bundle is replaced.
- `screenshots/*.png` - Flutter web client captures used by the root `README.md`.
  Not distributed to any client.

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
| `clients/basic` | `placeholders/poster.svg` | `placeholder.svg` |
| `clients/basic` | `placeholders/landscape.svg` | `placeholder-landscape.svg` |
| `clients/basic` | `placeholders/person.svg` | `placeholder-person.svg` |
| `clients/basic` | `vendor/hls.min.js` | `vendor/hls.min.js` |
| `clients/flutter` | `icons/flutter/favicon.png` | `web/favicon.png` |
| `clients/flutter` | `icons/flutter/Icon-192.png` | `web/icons/Icon-192.png` |
| `clients/flutter` | `icons/flutter/Icon-512.png` | `web/icons/Icon-512.png` |
| `clients/flutter` | `icons/flutter/Icon-maskable-192.png` | `web/icons/Icon-maskable-192.png` |
| `clients/flutter` | `icons/flutter/Icon-maskable-512.png` | `web/icons/Icon-maskable-512.png` |
| `clients/flutter` | `vendor/hls.min.js` | `web/hls.min.js` |

As the Android, iOS, and Roku clients land, add their launcher and icon targets
here.
