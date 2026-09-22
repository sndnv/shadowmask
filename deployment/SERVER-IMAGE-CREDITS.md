# Credits and third-party attribution: the server image

Covers `ghcr.io/sndnv/shadowmask-server` and its `-enrichment` variant. Copied into those images as
`/usr/share/doc/shadowmask/CREDITS.md`. The clients bundle a different set of libraries and are
credited from the *Third-party content* section of the
[README](https://github.com/sndnv/shadowmask#third-party-content).

## Where every licence lives inside the image

| What | Path in the image |
|------|-------------------|
| Shadowmask itself (Apache-2.0) | `/usr/share/doc/shadowmask/LICENSE` |
| The Rust crates linked into the binary | `/usr/share/doc/shadowmask/THIRD-PARTY-LICENSES.md` |
| Every Debian package, with its version | `/usr/share/doc/<package>/copyright` |
| Licence texts those files reference | `/usr/share/common-licenses/` |
| hls.js, bundled by the basic web client | `/usr/share/shadowmask/basic/vendor/hls.js.LICENSE.txt` |
| C++ source vendored into the binary, `-enrichment` image only | `/usr/share/doc/shadowmask/third-party-licenses/` |

Nothing has to be fetched from the network. The Debian copyright files are present because
`debian:bookworm-slim` re-includes `/usr/share/doc/*/copyright` after excluding `/usr/share/doc/*`;
a base image carries 318 of them.

## Debian packages

Installed unmodified from bookworm: `ffmpeg`, `mesa-va-drivers`, `libva2`, `ca-certificates`,
`curl`, `python3`, `python3-venv`, and their dependencies. The `-enrichment` image adds
`libopenblas0` and `libgomp1`.

**FFmpeg is Debian's GPL-2+ build.** From `/usr/share/doc/ffmpeg/copyright`:

> For building the default Debian packages some of the GPL licensed files are used, so the
> resulting binaries are licensed under GPL v2+.

`ffmpeg -buildconf` confirms `--enable-gpl`. Debian builds no non-free variant.

**Shadowmask links nothing from FFmpeg.** There are no `libav*` bindings in `Cargo.lock`; every use
is a subprocess (`std::process::Command::new`). The image aggregates two separately licensed works
rather than combining them: Shadowmask stays Apache-2.0, `ffmpeg` is redistributed under its own
terms, and replacing it needs no change here since both binaries resolve from `PATH`.

**Source.** Each package records its source package and version in its own `copyright` file. Debian
publishes the corresponding source at <https://sources.debian.org> and <https://snapshot.debian.org>,
also reachable with `apt-get source <package>` given a `deb-src` entry for bookworm. If a source
package is no longer available there, open an issue at
<https://github.com/sndnv/shadowmask/issues> and it will be provided.

## yt-dlp

Installed from PyPI into `/opt/yt-dlp` at build time, symlinked to `/usr/local/bin/yt-dlp`, and run
as a subprocess. Released into the public domain under the
[Unlicense](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE).

It is **not pinned**, so extractor fixes arrive on each image rebuild and the version varies between
builds of the same Shadowmask version. Run `yt-dlp --version` to see which one is installed.

## The enrichment image

Adds CTranslate2 and SentencePiece (both MIT), compiled from vendored source and statically linked
through the `ct2rs` and `sentencepiece-sys` crates. Every licence text found in those two crate
sources — theirs and their own vendored dependencies' — is at
`/usr/share/doc/shadowmask/third-party-licenses/`, under paths naming the library each came from.

Model weights are not part of any Shadowmask image; you supply them and their licences are yours to
comply with. See
[`ENRICHMENT.md`](https://github.com/sndnv/shadowmask/blob/main/deployment/ENRICHMENT.md#licensing-and-attribution).
