# Credits and third-party attribution

The macOS and iOS applications bundle the libraries listed below. They are built and published by
[libmpv-darwin-build](https://github.com/media-kit/libmpv-darwin-build) `v0.6.0`, flavour
`macos-universal-video-default` on macOS and `ios-universal-video-default` on iOS, and reach this
project through the `media_kit_libs_macos_video` and `media_kit_libs_ios_video` packages. Both
flavours carry the same set of frameworks.

Each library ships as a separate `.framework` dylib and is linked dynamically, under
`shadowmask.app/Contents/Frameworks` on macOS and `Shadowmask.app/Frameworks` on iOS. Any of them
may be replaced with a modified version by substituting the corresponding framework in that
directory. The macOS bundle is not code signed, so no signature has to be reproduced afterwards.
An iOS build must be signed to install, so a substitution there has to be followed by re-signing
the application.

The licence of each build was read from the shipped binaries, on both platforms: `Mpv` embeds
`-Dgpl=false`, and all six FFmpeg frameworks embed `license: LGPL version 3 or later`.

The web client bundles none of these. The Linux application bundles none of these either: it links
the distribution's own libmpv and GTK, neither of which is redistributed here. What the Linux
AppImage does bundle is listed under [Linux application](#linux-application) below.

The Android application bundles a different build of most of the same libraries, listed under
[Android application](#android-application) below.

This file and the licence texts it links are bundled into every build, so they travel inside the
APK, IPA, disk image and AppImage. `assets/licenses/` is distributed from the repository's
`licenses/` by `assets/refresh_assets.py` and must not be edited by hand.

The `../../licenses/` links below resolve in the repository. Inside an artifact the texts are
re-rooted next to this file, so every text named is present but the link is repo-shaped:

| Artifact          | This file                       | Licence texts                        |
|-------------------|---------------------------------|--------------------------------------|
| Disk image (.dmg) | `CREDITS.md` at the volume root | `Licenses/`                          |
| AppImage          | `usr/share/doc/shadowmask/`     | `usr/share/doc/shadowmask/licenses/` |
| APK / IPA         | `flutter_assets/CREDITS.md`     | `flutter_assets/assets/licenses/`    |

The AppImage carries only `LGPL-2.1.txt`: it bundles the GNU C Library and nothing else. See
[Linux application](#linux-application).

The same texts are readable under Account → Profile → About → Third-party licenses, alongside the
Dart packages, per-platform so a build credits only what it bundles.

It also credits the content providers, with TMDB's logo
(`assets/attribution/tmdb-logo.png`, distributed by `assets/refresh_assets.py`) and their required
notice: *This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise
approved by TMDB.* The logo must not be recoloured, reproportioned or rotated; the notice is
verbatim and pinned by a test.

## Bundled libraries

| Library     | Frameworks                                                           | License                                                                            | License text                                                                           | Source                                           |
|-------------|----------------------------------------------------------------------|------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------|--------------------------------------------------|
| mpv         | `Mpv`                                                                | LGPL-2.1-or-later                                                                  | [LGPL-2.1.txt](../../licenses/LGPL-2.1.txt)                                            | https://github.com/mpv-player/mpv                |
| FFmpeg      | `Avcodec`, `Avfilter`, `Avformat`, `Avutil`, `Swresample`, `Swscale` | LGPL-3.0-or-later                                                                  | [LGPL-3.0.txt](../../licenses/LGPL-3.0.txt), [GPL-3.0.txt](../../licenses/GPL-3.0.txt) | https://ffmpeg.org/download.html                 |
| dav1d       | `Dav1d`                                                              | BSD-2-Clause                                                                       | [dav1d.LICENSE.txt](../../licenses/dav1d.LICENSE.txt)                                  | https://code.videolan.org/videolan/dav1d         |
| libass      | `Ass`                                                                | ISC                                                                                | [libass.LICENSE.txt](../../licenses/libass.LICENSE.txt)                                | https://github.com/libass/libass                 |
| FreeType    | `Freetype`                                                           | FreeType Project License                                                           | [FreeType.LICENSE.txt](../../licenses/FreeType.LICENSE.txt)                            | https://gitlab.freedesktop.org/freetype/freetype |
| GNU FriBidi | `Fribidi`                                                            | LGPL-2.1-or-later                                                                  | [LGPL-2.1.txt](../../licenses/LGPL-2.1.txt)                                            | https://github.com/fribidi/fribidi               |
| HarfBuzz    | `Harfbuzz`                                                           | Old MIT                                                                            | [HarfBuzz.LICENSE.txt](../../licenses/HarfBuzz.LICENSE.txt)                            | https://github.com/harfbuzz/harfbuzz             |
| Mbed TLS    | `Mbedtls`, `Mbedcrypto`, `Mbedx509`                                  | Apache-2.0 or GPL-2.0-or-later, taken here under Apache-2.0                        | [MbedTLS.LICENSE.txt](../../licenses/MbedTLS.LICENSE.txt)                              | https://github.com/Mbed-TLS/mbedtls              |
| libpng      | `Png16`                                                              | PNG Reference Library License v2                                                   | [libpng.LICENSE.txt](../../licenses/libpng.LICENSE.txt)                                | https://github.com/pnggroup/libpng               |
| uchardet    | `Uchardet`                                                           | MPL-1.1, GPL-2.0-or-later or LGPL-2.1-or-later, taken here under LGPL-2.1-or-later | [uchardet.LICENSE.txt](../../licenses/uchardet.LICENSE.txt)                            | https://gitlab.freedesktop.org/uchardet/uchardet |
| libxml2     | `Xml2`                                                               | MIT                                                                                | [libxml2.LICENSE.txt](../../licenses/libxml2.LICENSE.txt)                              | https://gitlab.gnome.org/GNOME/libxml2           |

`GPL-3.0.txt` is included because the GNU Lesser General Public License version 3 incorporates the
terms of the GNU General Public License version 3 by reference.

On the disk image these texts sit in a `Licenses` folder beside this file rather than at the paths
linked above, which are relative to the repository.

## Android application

The Android application bundles the libraries listed below. They are built and published by
[libmpv-android-video-build](https://github.com/media-kit/libmpv-android-video-build) `v1.1.7`,
flavour `default`, and reach this project through the `media_kit_libs_android_video` package as one
jar per ABI, downloaded at build time against pinned MD5 checksums.

All of them are linked statically into a single `libmpv.so`, which ships as
`lib/<abi>/libmpv.so` inside the APK and is loaded dynamically by the application. That library may
be replaced with a modified version by substituting it in the APK, which then has to be re-signed
to install. Rebuilding it from modified sources is what the upstream build scripts above do.

The licence of each build was read from the shipped binaries, on every ABI in the APK: mpv embeds
`-Dgpl=false`, and FFmpeg embeds `--disable-gpl --disable-nonfree --enable-version3` together with
`license: LGPL version 3 or later` for all six of its libraries.

This build differs from the macOS and iOS one above: it adds zlib and it carries neither libpng nor
uchardet.

| Library     | License                                                     | License text                                                                           | Source                                           |
|-------------|-------------------------------------------------------------|----------------------------------------------------------------------------------------|--------------------------------------------------|
| mpv         | LGPL-2.1-or-later                                           | [LGPL-2.1.txt](../../licenses/LGPL-2.1.txt)                                            | https://github.com/mpv-player/mpv                |
| FFmpeg      | LGPL-3.0-or-later                                           | [LGPL-3.0.txt](../../licenses/LGPL-3.0.txt), [GPL-3.0.txt](../../licenses/GPL-3.0.txt) | https://ffmpeg.org/download.html                 |
| dav1d       | BSD-2-Clause                                                | [dav1d.LICENSE.txt](../../licenses/dav1d.LICENSE.txt)                                  | https://code.videolan.org/videolan/dav1d         |
| libass      | ISC                                                         | [libass.LICENSE.txt](../../licenses/libass.LICENSE.txt)                                | https://github.com/libass/libass                 |
| FreeType    | FreeType Project License                                    | [FreeType.LICENSE.txt](../../licenses/FreeType.LICENSE.txt)                            | https://gitlab.freedesktop.org/freetype/freetype |
| GNU FriBidi | LGPL-2.1-or-later                                           | [LGPL-2.1.txt](../../licenses/LGPL-2.1.txt)                                            | https://github.com/fribidi/fribidi               |
| HarfBuzz    | Old MIT                                                     | [HarfBuzz.LICENSE.txt](../../licenses/HarfBuzz.LICENSE.txt)                            | https://github.com/harfbuzz/harfbuzz             |
| Mbed TLS    | Apache-2.0 or GPL-2.0-or-later, taken here under Apache-2.0 | [MbedTLS.LICENSE.txt](../../licenses/MbedTLS.LICENSE.txt)                              | https://github.com/Mbed-TLS/mbedtls              |
| libxml2     | MIT                                                         | [libxml2.LICENSE.txt](../../licenses/libxml2.LICENSE.txt)                              | https://gitlab.gnome.org/GNOME/libxml2           |
| zlib        | zlib License                                                | [zlib.LICENSE.txt](../../licenses/zlib.LICENSE.txt)                                    | https://zlib.net                                 |

## Linux application

The AppImage bundles the GNU C Library, and links libmpv and GTK from the host without
redistributing either.

| Library               | License           | License text                                | Source                        |
|-----------------------|-------------------|---------------------------------------------|-------------------------------|
| GNU C Library (glibc) | LGPL-2.1-or-later | [LGPL-2.1.txt](../../licenses/LGPL-2.1.txt) | https://sourceware.org/glibc/ |

## Web client

The web client bundles [hls.js](https://github.com/video-dev/hls.js) under Apache-2.0. Its licence
text and copyright notices travel with the bundle as
[`web/hls.js.LICENSE.txt`](web/hls.js.LICENSE.txt).
