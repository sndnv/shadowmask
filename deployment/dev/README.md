# deployment / dev

`docker-compose.yml` builds and runs Shadowmask locally for development and testing.

## Getting started

1) Provide fixture media, or point the library dirs at your own:
   - `SHADOWMASK_MOVIES_DIR` (default `./media/movies`)
   - `SHADOWMASK_TV_DIR` (default `./media/tv`)
2) Build and start: `docker compose up --build` (or `podman-compose up --build`).
3) The server listens on `http://localhost:${SHADOWMASK_PORT:-8080}`; `/health` reports readiness.

First-run bootstrap is enabled (`SHADOWMASK_BOOTSTRAP_MODE=init-and-start`) and creates the
`admin` user with password `SHADOWMASK_ADMIN_PASSWORD` (default `passw0rd`). Library and user
definitions live in `config/bootstrap/`.

The JWT and stream secrets are dev-only throwaway values set inline in the compose file. Do not
reuse them anywhere.

The Flutter web client runs on its own origin, so cross-origin (CORS) access is enabled for dev:
`SHADOWMASK_CORS_ALLOWED_ORIGINS` defaults to `http://localhost:8090`. Run the client on that port
(`flutter run -d chrome --web-port=8090 --dart-define=SHADOWMASK_API_BASE=http://localhost:8080`),
or override the env var to match a different origin. Rebuild the image (`docker compose up --build`)
so the server binary includes the CORS layer. The basic client is served same-origin and needs none.

TLS is off by default here (dev serves plaintext HTTP). To exercise the server's optional TLS
locally, mount a self-signed cert + key and set `SHADOWMASK_TLS_CERT` / `SHADOWMASK_TLS_KEY`; see
the TLS section in the production README.

## Container images

Two images are built from the root `Dockerfile` as separate stages:

- **enrichment** (`server:dev-latest-enrichment`, stage `runtime-enrichment`) - the default image for
  this compose. It is `FROM` the base stage plus the local AI features (transcription and
  translation, bundling CTranslate2), so it shares the base layers but is larger and slower to build
  (compiles CTranslate2 via cmake / make / g++ / OpenBLAS).
- **base** (`server:dev-latest`, stage `runtime`) - no local-inference runtime; smaller and faster to
  build. Used by `docker-compose.base.yml`.

`docker compose up --build` builds the enrichment image (`docker-compose.yml` pins
`target: runtime-enrichment`). For a lighter base-only stack, use
`docker compose -f docker-compose.base.yml up --build`. Build the images directly with (docker or
podman auto-detected; override with `CONTAINER_ENGINE`):

```
scripts/build_images.sh                # both
scripts/build_images.sh base
scripts/build_images.sh enrichment
```

Enrichment is enabled by default in `docker-compose.yml`: transcription and translation are on
`auto`, which schedules them after a scan, and upscaling is on. Upscaling is pure ffmpeg and needs
no model. Transcription and translation need models on
disk (see "Local AI models" below); without them the server boots and each behaves as off, queueing
no job of that kind and answering `409 feature_disabled` on any request for the work. Each takes a
mode, so `SHADOWMASK_ENRICHMENT_TRANSCRIPTION_MODE=off` turns one off and `=on` keeps it triggerable
by an admin without scheduling it.

## Local AI models

Local transcription, translation, and upscaling are documented in
[`../ENRICHMENT.md`](../ENRICHMENT.md) (model formats, obtaining/converting models, licensing). This
dev compose already targets the enrichment image, enables the features, mounts `./models` at
`/models`, and sets `SHADOWMASK_ENRICHMENT_MODEL_CACHE=/models`. To try it, drop a CTranslate2 model
folder into `./models/transcription/` (and `./models/translation/` for translation), then trigger a
transcribe from the admin UI or API. Each directory holds one or more candidate model folders and the
first valid one wins. Override the host models directory with `SHADOWMASK_MODELS_DIR`.

Translation defaults to a MADLAD model: the compose sets
`SHADOWMASK_ENRICHMENT_TRANSLATION_SOURCE_PREFIX=<2{target}>`, so put a MADLAD CTranslate2 model (with
its `tokenizer.json`) under `./models/translation/`. For a different model, change that prefix (see
[`../ENRICHMENT.md`](../ENRICHMENT.md)).

## Hardware transcoding

GPU-accelerated H.264 encoding (VAAPI) applies to this stack too; see
[hardware acceleration](../README.md#hardware-acceleration) for the `SHADOWMASK_HARDWARE_ACCELERATION`
modes and the `/dev/dri` passthrough snippet. It is not wired into this `docker-compose.yml` by
default (macOS dev hosts have no render node), so add the `devices`/`group_add` block on a Linux GPU
host to try it.

## Basic web client (live editing)

The server serves the basic web client at `http://localhost:${SHADOWMASK_PORT:-8080}/ui/basic/`
(`/` and `/ui` redirect there). The image bakes a copy of `clients/basic` at build time, but this
compose file also bind-mounts the working tree over it read-only:

```
../../clients/basic:/usr/share/shadowmask/basic:ro
```

Because the server serves those files straight off disk on each request, edits to the HTML or JS
are live: save the file and reload the browser, no rebuild or restart. Sign in with the bootstrap
admin (`admin` / `SHADOWMASK_ADMIN_PASSWORD`, default `passw0rd`).

## Fixture media

`scripts/generate_media.sh` populates the dev media dirs with a batch of small synthetic movies and
TV episodes (generated with ffmpeg, no real content), so the catalog has something to browse and
play. It writes into the same dirs the compose file mounts (`SHADOWMASK_MOVIES_DIR` /
`SHADOWMASK_TV_DIR`, default `./media/movies` and `./media/tv`). Run it, then trigger a library scan
(admin UI or API). See its `--help` for options. It is additive by default; pass `--reset` to clear
the dirs first. Pass `--real` to download real Creative Commons clips in place of the synthetic
stand-ins so full-length content can be played (see the paragraph below). Pass `--cases` to add the
catalog shapes specific checks need: one title in two qualities (`Paper Skies (2007)`, 2160p and
720p), a show with a Specials season and three regular seasons (`Skyline`), a show with only a
Specials season (`Lantern`), and a film filed one year before its TMDB year (`Casablanca (1942)`;
TMDB has 1943). The smoke test (below) generates its own fixtures separately.

The movie titles are real, TMDB-matchable names (the public-domain Blender open movies). Set
`SHADOWMASK_TMDB_API_KEY` before starting the stack and a scan will enrich them with real metadata
and artwork, so the full artwork pipeline is visible end to end. Without a key the catalog still
works but shows placeholder posters and empty metadata. One series is intentionally unmatchable so
the mismatch / manual-resolution behavior is also visible.

TMDB does not return content ratings, so parental controls stay inert on TMDB alone. Also set
`SHADOWMASK_OMDB_API_KEY` and the scan will look up each title's certification via OMDB and populate
`content_rating`, which is what the per-user rating cap enforces. TMDB stays the primary provider;
OMDB is an optional rating supplement, so an OMDB key without a TMDB key does not enable enrichment.

OMDb data is [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/); the NonCommercial term
follows the key, so a commercial deployment should leave it unset.

`--real` downloads two Creative Commons BY 3.0 clips (Big Buck Bunny and a short Elephants Dream clip
with clear speech, used by the transcription smoke below) in place of their synthetic stand-ins.
Downloads are cached under `media/.cache` and reused across runs and by the smoke test, so nothing is
re-downloaded. Attribution for all downloaded content is in `deployment/dev/CREDITS.md`.

## Logging

Log verbosity is set per target so you can run the service at `debug` without the SQL query
firehose:

- `SHADOWMASK_LOG_LEVEL` (default `info`, `debug` in this compose file) - level for Shadowmask's own
  crates.
- `SHADOWMASK_SQLX_LOG_LEVEL` (default `warn`) - level for the `sqlx` target; raise it to `debug` to
  see every query. Everything else (hyper/axum/tower) stays at `warn`.
- `RUST_LOG`, if set, overrides both with a raw `tracing` filter directive.

## Monitoring (optional)

`docker compose --profile monitoring up -d` (with `-f docker-compose.base.yml` for the base stack)
also starts Prometheus and Grafana; see "Prometheus and Grafana" in
[`../README.md`](../README.md#prometheus-and-grafana).

## Smoke test

`scripts/run_smoke_test.sh` drives an already-running stack through the full product surface (auth,
scan, catalog, playback, backup/recover, media-removal soft-delete, and a basic web UI sanity
check that `/ui/basic/` and `app.js` are served). It generates its own ffmpeg
fixtures and is destructive to the dev databases and fixture media by design, so run it only against
this disposable stack. Bring the stack up first, then run the script; see its `--help` for
environment overrides.

The transcribe -> translate section runs only on the enrichment image. On the base image it is
skipped. On the enrichment image the enrichment features must be enabled with models present or the
test fails; it uses the Elephants Dream speech clip from the clip cache, so run
`scripts/generate_media.sh --real` first to populate it. Set
`SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS=true` to skip the enrichment section.

## Playback check

`scripts/run_playback_check.sh` plays the server's streams in two players the clients use, all in
containers (docker or podman), with a throwaway server of its own (compose project
`shadowmask-playback`, its own volume). It does not touch the dev stack.

| Player   | Image                                           | Stands in for                        |
|----------|-------------------------------------------------|--------------------------------------|
| chromium | Debian bookworm, Chromium, basic's hls.js       | basic and the Flutter web client     |
| mpv      | Ubuntu 24.04, mpv 0.37 with media_kit's options | the Flutter desktop and Android apps |

Per subject and player:

- `play`, `resume` (20 min), `far-seek` (in a stream `/seek` and a new stream; in Direct play a
  seek in the player, as every client does when the stream is not sequential), `near-seek`.
- With a subtitle: `subtitles-resume`, `subtitles-far-seek`, `subtitles-seek-back` (a line three
  times), and `subtitles-mode`: the subtitle must not change the playback mode (M139).
- `second-audio`: the second audio track is the one heard (mpv: the playing track; chromium: the
  tone the page hears through Web Audio, each test file track having its own). It runs last for
  each subject, since the server remembers the tracks chosen for a version and later sessions
  would get it.
- mpv only: `forced-resume`, `forced-far-seek` and, with a subtitle, `forced-subtitles-far-seek`,
  each streamed through `delivery: "always"`.
- Image subtitles (F7, PGS and DVB): `image-subtitle-pgs`, `image-subtitle-dvb` from 58 s. mpv
  passes when its selected track has the image codec, `sub-start` is the 60 s line and mpv draws
  subtitles (`sub-visibility`); chromium passes when the stream plays past the line.

A case fails when playback does not advance, the position is more than 1.5 s off the target, a
subtitle line shows more than once, no audio is decoded, hls.js or the media element reports an
error, or mpv's log has "could not find codec parameters". The run exits 1 when any case fails.

In the test set a case also fails when its negotiations differ from `playback/modes.json`. Each
entry, keyed `<player> | <subject> | <action>`, lists the negotiations of the sessions the case
started, in order, as the server logged them: mode and segment container, reasons, and whether
the server took the client's capability report (`report none`, `accepted` or `rejected`). The
file holds the mode each case should get, not today's when today's is wrong, so such a case stays
red until the fix lands; a change that alters a mode on purpose changes the file with it. The
run's own negotiations go to `modes.json` in the report folder; `--accept-modes` copies them
into `playback/modes.json` instead of checking, replacing only the cases that ran.

```
scripts/run_playback_check.sh                                 # the test set
scripts/run_playback_check.sh --accept-modes                  # record the negotiations
scripts/run_playback_check.sh --movies /path/to/movies --tv /path/to/tv --per-shape 2
scripts/run_playback_check.sh --movies /path/to/movies --census-only
scripts/run_playback_check.sh --player mpv --only 'F[23] .*far-seek'  # a slice
```

- `--player chromium|mpv` runs one player; `--only REGEX` runs only the cases whose
  `<subject> | <action>` matches (subjects are the report's labels, such as `F2 H.264, AC3
  5.1(side)`).
- Each mpv case writes its player log to `logs/mpv/<subject number>-<action>.log` in the report
  folder, and the report names the log of each failed case. The logs hold the stream URLs of the
  throwaway server, whose tokens end with it.

- The test set is fourteen 30 minute files (`generate_media.sh --playback`), generated inside the
  mpv image into `media/playback` once per set version (`PLAYBACK_SET_VERSION`, written to
  `.complete`); F7's image subtitles come from `crates/media/tests/fixtures/supsample.mkv`. F1-F7
  are H.264 or HEVC in MKV with AAC, AC3, E-AC3 or DTS; F8-F14 are the catalog's other shapes:
  MPEG-4 Part 2 with B-frames and MS MPEG-4 v3 in AVI with MP3, MPEG-4 Part 2 in MKV, VP9 with
  Opus in WebM, H.264 with FLAC, H.264 with AAC in MP4, and H.264 with AAC in AVI. The report goes to
  `media/playback/out` (`report.md`, the results as JSON, the server's log, `modes.json`).
- Library mode mounts the catalog read-only, probes every file's headers, groups files by
  container, video codec and depth, HDR, the first audio track, more than one audio track and
  subtitle kind, and checks `--per-shape` files of each shape. Only the sampled movie folders and
  show folders are scanned. The census and report name real titles, so they go outside the repo
  (default under `$TMPDIR`). The catalog must be visible to the container engine: for podman on
  macOS, under `$HOME`.
- The server image is `ghcr.io/sndnv/shadowmask-server:dev-latest`, built with
  `build_images.sh base` when missing; `--rebuild` builds it from this tree, which is how a fix
  is checked. `SHADOWMASK_PLAYBACK_SERVER_IMAGE` picks another image.
- The players copy what each client does, including what it leaves out, so a client change that
  alters how it loads a stream needs the same change here:
  - mpv: the session start of `client_capabilities_native.dart` and the desktop report of
    `player_report.dart` (platform `desktop`: its tables, `decoder-list`, `demuxer-lavf-list`),
    `NativePlayerController.attach` (`start`, `sid`, the buffer settings, the seek to
    the offset once the new file's length is known), media_kit 1.2.6's player options and its
    `open` (`stop`, `playlist-clear`, `pause`, then the load), and the timeline of
    `actors/player.dart` (`origin_ms` plus the player's position).
  - chromium: the session start of basic (platform `chrome`) and the `attach` of
    `player/watch.html`, the timeline and far seek of `player/session.js`. Its capability report
    is not a copy: the page imports basic's own `player/capabilities.js` (compose mounts
    `clients/basic/player` read-only) and the driver sends what it measures. The page sets
    `crossorigin` on its video because it runs on another origin than the server (basic is
    served by the server), so Web Audio and a `<track>` from the server work.
- **Ahead of the apps (M139, until its client phase is built):** both players already declare
  `subtitle_deliveries` and `audio_tracks` and act on the answers M139's server phase adds: mpv
  selects `embedded` tracks by `ff-index` (`sid`, with `sub-visibility` on for an image track),
  loads `external` files with `sub-add` and sets `aid` when `audio_delivery` is `embedded`;
  chromium adds a `<track>` for `external`. Today's server sends none of these, so until then
  the players behave as the apps do.
- Not covered: Firefox, the macOS and Android apps' own libmpv builds (0.36), anything about the
  picture (there is no GPU or display), Roku.
- `--keep` leaves the server running after the run; `docker compose -f playback/compose.yml down -v`
  (or `podman compose`) removes it.
