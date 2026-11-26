# Credits and third-party attribution

The macOS and iOS applications bundle the libraries listed below. They are built and published by
[libmpv-darwin-build](https://github.com/media-kit/libmpv-darwin-build) `v0.7.3`, flavour
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

The Windows application bundles a third build, with many more libraries linked into one DLL, listed
under [Windows application](#windows-application) below.

This file and the licence texts it links are bundled into every build, so they travel inside the
APK, IPA, disk image, AppImage and Windows build. `assets/licenses/` is distributed by `assets/refresh_assets.py`
from the repository's `licenses/`, and the Roboto text from its `assets/fonts/`; it must not be
edited by hand.

The `../../licenses/` links below, and the Roboto row's `../../assets/fonts/`, resolve in the
repository. Inside an artifact the texts are re-rooted next to this file, so every text named is
present but the link is repo-shaped:

| Artifact          | This file                       | Licence texts                        |
|-------------------|---------------------------------|--------------------------------------|
| Disk image (.dmg) | `CREDITS.md` at the volume root | `Licenses/`                          |
| Windows zip       | `CREDITS.md` at the zip root    | `Licenses\`                          |
| AppImage          | `usr/share/doc/shadowmask/`     | `usr/share/doc/shadowmask/licenses/` |
| APK / IPA         | `flutter_assets/CREDITS.md`     | `flutter_assets/assets/licenses/`    |

The AppImage carries `LGPL-2.1.txt` and `Roboto-OFL.txt`: it bundles the GNU C Library and the
Roboto font. See [Linux application](#linux-application) and [Font](#font).

The same texts are readable under Account → Profile → About → Third-party licenses, alongside the
Dart packages, per-platform so a build credits only what it bundles.

It also credits the content providers, with TMDB's logo
(`assets/attribution/tmdb-logo.png`, distributed by `assets/refresh_assets.py`) and their required
notice: *This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise
approved by TMDB.* The logo must not be recoloured, reproportioned or rotated; the notice is
verbatim and pinned by a test.

## Font

Every build carries Roboto (Regular, Medium and Bold, the hinted static faces of release v3.016) in
`assets/fonts/`, distributed from the repository's `assets/fonts/` by `assets/refresh_assets.py`.
Only the Linux application draws its text with it; the other platforms use their system font.

| Font   | License     | License text                                        | Source                                          |
|--------|-------------|-----------------------------------------------------|-------------------------------------------------|
| Roboto | SIL OFL-1.1 | [Roboto-OFL.txt](../../assets/fonts/Roboto-OFL.txt) | https://github.com/googlefonts/roboto-3-classic |

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

## Windows application

The Windows application bundles `libmpv-2.dll` from
[libmpv-win32-video-cmake](https://github.com/media-kit/libmpv-win32-video-cmake) release
`20261123`, archive `mpv-dev-x86_64-20261123-git-b0841e8c4d.7z` (SHA-256
`437191bc801cd6f9f87c24e79a7f4afd117d07f609d196184a031634bcc1dcd3`), through the
`media_kit_libs_windows_video` package. It carries mpv v0.42.0 and FFmpeg n9.1. This is the x64
build, the only one released; an arm64 build, made only in the development VM, takes the release's
aarch64 archive instead.

All of the libraries below are linked statically into that one DLL, which ships beside
`shadowmask.exe` and is loaded dynamically by it. It may be replaced with a modified version by
substituting the file. The application is not code signed.

The licence of the build was read from the shipped DLL and its build files: `libmpv-2.dll` embeds
mpv's `-Dgpl=false`, and the builder configures FFmpeg with `--disable-gpl --disable-nonfree
--enable-version3` at the release's source commit. None of FFmpeg's GPL-only libraries are in the
DLL.

Every licence text, with each project's source and the commit or version it was built from, is in
[libmpv-windows.NOTICES.txt](../../licenses/libmpv-windows.NOTICES.txt). It is generated by
`licenses/refresh_windows_notices.py` from the release's build record. subrandr, SRT and the Public
Suffix List (built into libpsl) are under MPL-2.0; their source is at the links below.

| Library                              | License                                                                             | Source                                                                                 |
|--------------------------------------|-------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------|
| mpv                                  | LGPL-2.1-or-later                                                                   | https://github.com/mpv-player/mpv                                                      |
| FFmpeg                               | LGPL-3.0-or-later                                                                   | https://github.com/FFmpeg/FFmpeg                                                       |
| dav1d                                | BSD-2-Clause                                                                        | https://code.videolan.org/videolan/dav1d                                               |
| libvpx                               | BSD-3-Clause                                                                        | https://github.com/webmproject/libvpx                                                  |
| uavs3d                               | BSD-3-Clause                                                                        | https://github.com/uavs3/uavs3d                                                        |
| libjxl                               | BSD-3-Clause                                                                        | https://github.com/libjxl/libjxl                                                       |
| Highway                              | Apache-2.0 OR BSD-3-Clause, taken here under Apache-2.0                             | https://github.com/google/highway                                                      |
| Brotli                               | MIT                                                                                 | https://github.com/google/brotli                                                       |
| libopenmpt                           | BSD-3-Clause                                                                        | https://lib.openmpt.org/files/libopenmpt/src/libopenmpt-0.8.10+release.autotools.tar.gz |
| libogg                               | BSD-3-Clause                                                                        | https://github.com/xiph/ogg                                                            |
| libvorbis                            | BSD-3-Clause                                                                        | https://github.com/xiph/vorbis                                                         |
| libmodplug                           | Public domain                                                                       | https://github.com/Konstanty/libmodplug                                                |
| libsoxr                              | LGPL-2.1-or-later                                                                   | https://gitlab.com/shinchiro/soxr                                                      |
| libplacebo                           | LGPL-2.1-or-later                                                                   | https://github.com/haasn/libplacebo                                                    |
| shaderc                              | Apache-2.0                                                                          | https://github.com/google/shaderc                                                      |
| glslang                              | Several, see the notices                                                            | https://github.com/KhronosGroup/glslang                                                |
| SPIRV-Tools                          | Apache-2.0                                                                          | https://github.com/KhronosGroup/SPIRV-Tools                                            |
| SPIRV-Headers                        | Several, see the notices                                                            | https://github.com/KhronosGroup/SPIRV-Headers                                          |
| SPIRV-Cross                          | Apache-2.0                                                                          | https://github.com/KhronosGroup/SPIRV-Cross                                            |
| Vulkan-Headers                       | Apache-2.0 OR MIT                                                                   | https://github.com/KhronosGroup/Vulkan-Headers                                         |
| Little CMS                           | MIT                                                                                 | https://github.com/mm2/Little-CMS                                                      |
| zimg                                 | WTFPL                                                                               | https://github.com/sekrit-twc/zimg                                                     |
| graphengine                          | WTFPL                                                                               | https://github.com/sekrit-twc/graphengine                                              |
| libass                               | ISC                                                                                 | https://github.com/libass/libass                                                       |
| GNU FriBidi                          | LGPL-2.1-or-later                                                                   | https://github.com/fribidi/fribidi                                                     |
| HarfBuzz                             | Old MIT                                                                             | https://github.com/harfbuzz/harfbuzz                                                   |
| FreeType                             | FTL OR GPL-2.0-or-later, taken here under FTL                                       | https://github.com/freetype/freetype                                                   |
| Fontconfig                           | Several, see the notices                                                            | https://gitlab.freedesktop.org/fontconfig/fontconfig                                   |
| libunibreak                          | Zlib                                                                                | https://github.com/adah1972/libunibreak                                                |
| uchardet                             | MPL-1.1 OR GPL-2.0-or-later OR LGPL-2.1-or-later, taken here under LGPL-2.1-or-later | https://gitlab.freedesktop.org/uchardet/uchardet                                       |
| GNU libiconv                         | LGPL-2.1-or-later                                                                   | https://ftp.gnu.org/pub/gnu/libiconv/libiconv-1.19.1.tar.gz                            |
| libva                                | MIT                                                                                 | https://github.com/intel/libva                                                         |
| Intel VPL                            | MIT                                                                                 | https://github.com/intel/libvpl                                                        |
| AMF                                  | MIT                                                                                 | https://github.com/GPUOpen-LibrariesAndSDKs/AMF                                        |
| nv-codec-headers                     | MIT                                                                                 | https://github.com/FFmpeg/nv-codec-headers                                             |
| xxHash                               | BSD-2-Clause                                                                        | https://github.com/Cyan4973/xxHash                                                     |
| VapourSynth headers                  | LGPL-2.1-or-later                                                                   | https://github.com/vapoursynth/vapoursynth                                             |
| OpenSSL                              | Apache-2.0                                                                          | https://github.com/openssl/openssl                                                     |
| curl                                 | curl                                                                                | https://github.com/curl/curl                                                           |
| nghttp2                              | MIT                                                                                 | https://github.com/nghttp2/nghttp2                                                     |
| nghttp3                              | MIT                                                                                 | https://github.com/ngtcp2/nghttp3                                                      |
| ngtcp2                               | MIT                                                                                 | https://github.com/ngtcp2/ngtcp2                                                       |
| c-ares                               | MIT                                                                                 | https://github.com/c-ares/c-ares                                                       |
| libpsl                               | MIT                                                                                 | https://github.com/rockdaboot/libpsl                                                   |
| Public Suffix List                   | MPL-2.0                                                                             | https://github.com/publicsuffix/list                                                   |
| libssh                               | LGPL-2.1-or-later                                                                   | https://gitlab.com/libssh/libssh-mirror                                                |
| SRT                                  | MPL-2.0                                                                             | https://github.com/Haivision/srt                                                       |
| libxml2                              | MIT                                                                                 | https://github.com/GNOME/libxml2                                                       |
| libarchive                           | Several, see the notices                                                            | https://github.com/libarchive/libarchive                                               |
| libjpeg-turbo                        | IJG AND BSD-3-Clause AND Zlib                                                       | https://github.com/libjpeg-turbo/libjpeg-turbo                                         |
| libpng                               | libpng-2.0                                                                          | https://github.com/glennrp/libpng                                                      |
| zlib-ng                              | Zlib                                                                                | https://github.com/zlib-ng/zlib-ng                                                     |
| Zstandard                            | BSD-3-Clause OR GPL-2.0-only, taken here under BSD-3-Clause                         | https://github.com/facebook/zstd                                                       |
| XZ Utils (liblzma)                   | 0BSD                                                                                | https://github.com/tukaani-project/xz                                                  |
| bzip2                                | bzip2-1.0.6                                                                         | https://gitlab.com/bzip2/bzip2                                                         |
| MinGW-w64 runtime and winpthreads    | Several, see the notices                                                            | https://github.com/mingw-w64/mingw-w64                                                 |
| LLVM libc++, libc++abi and libunwind | Apache-2.0 WITH LLVM-exception                                                      | https://github.com/llvm/llvm-project                                                   |
| C++/WinRT                            | MIT                                                                                 | https://github.com/microsoft/cppwinrt                                                  |
| subrandr                             | MPL-2.0; the Rust crates it links are in the notices                                | https://github.com/afishhh/subrandr                                                    |
| Rust standard library                | MIT OR Apache-2.0; the Rust crates it links are in the notices                      | https://github.com/rust-lang/rust                                                      |

The Windows zip also carries Microsoft's Visual C++ runtime (`msvcp140.dll`, `vcruntime140.dll` and
`vcruntime140_1.dll`) beside `shadowmask.exe`, redistributed under the Visual Studio licence terms
for distributable code.

## Linux application

The AppImage bundles the GNU C Library and the Roboto font (see [Font](#font)), and links libmpv
and GTK from the host without redistributing either.

| Library               | License           | License text                                | Source                        |
|-----------------------|-------------------|---------------------------------------------|-------------------------------|
| GNU C Library (glibc) | LGPL-2.1-or-later | [LGPL-2.1.txt](../../licenses/LGPL-2.1.txt) | https://sourceware.org/glibc/ |

## Web client

The web client bundles [hls.js](https://github.com/video-dev/hls.js) under Apache-2.0. Its licence
text and copyright notices travel with the bundle as
[`web/hls.js.LICENSE.txt`](web/hls.js.LICENSE.txt).
