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

Enrichment is enabled by default in `docker-compose.yml` (transcription, translation, and upscaling
are on). Upscaling is pure ffmpeg and needs no model. Transcription and translation need models on
disk (see "Local AI models" below); without them the server still boots and those jobs fail with a
backend error until the models are present. Turn any feature off with, for example,
`SHADOWMASK_ENRICHMENT_TRANSCRIPTION_ENABLED=false`.

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
stand-ins so full-length content can be played (see the paragraph below). The smoke test (below)
generates its own fixtures separately.

The movie titles are real, TMDB-matchable names (the public-domain Blender open movies). Set
`SHADOWMASK_TMDB_API_KEY` before starting the stack and a scan will enrich them with real metadata
and artwork, so the full artwork pipeline is visible end to end. Without a key the catalog still
works but shows placeholder posters and empty metadata. One TV show is intentionally unmatchable so
the mismatch / manual-resolution behavior is also visible.

TMDB does not return content ratings, so parental controls stay inert on TMDB alone. Also set
`SHADOWMASK_OMDB_API_KEY` and the scan will look up each title's certification via OMDB and populate
`content_rating`, which is what the per-user rating cap enforces. TMDB stays the primary provider;
OMDB is an optional rating supplement, so an OMDB key without a TMDB key does not enable enrichment.

`--real` downloads two Creative Commons BY 3.0 clips (Big Buck Bunny and a short Elephants Dream clip
with clear speech, used by the transcription smoke below) in place of their synthetic stand-ins.
Downloads are cached under `media/.cache` and reused across runs and by the smoke test, so nothing is
re-downloaded. Attribution for all downloaded content is in `deployment/dev/CREDITS.md`.

## Logging

Log verbosity is set per target so you can run the service at `debug` without the SQL query
firehose:

- `SHADOWMASK_LOG_LEVEL` (default `info`, `debug` in this compose file) — level for Shadowmask's own
  crates.
- `SHADOWMASK_SQLX_LOG_LEVEL` (default `warn`) — level for the `sqlx` target; raise it to `debug` to
  see every query. Everything else (hyper/axum/tower) stays at `warn`.
- `RUST_LOG`, if set, overrides both with a raw `tracing` filter directive.

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
