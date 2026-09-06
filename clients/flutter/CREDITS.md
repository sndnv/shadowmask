# Credits and third-party attribution

The macOS application bundles the libraries listed below. They are built and published by
[libmpv-darwin-build](https://github.com/media-kit/libmpv-darwin-build) `v0.6.0`, flavour
`macos-universal-video-default`, and reach this project through the `media_kit_libs_macos_video`
package.

Each library ships as a separate `.framework` dylib under
`shadowmask.app/Contents/Frameworks` and is linked dynamically. Any of them may be replaced with a
modified version by substituting the corresponding framework in that directory. The bundle is not
code signed, so no signature has to be reproduced afterwards.

The licence of each build was read from the shipped binaries: `Mpv` embeds `-Dgpl=false`, and all
six FFmpeg frameworks embed `license: LGPL version 3 or later`.

The web client bundles none of these. The Linux application bundles none of these either: it links
the distribution's own libmpv and GTK, neither of which is redistributed here. What the Linux
AppImage does bundle is listed under [Linux application](#linux-application) below.

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
