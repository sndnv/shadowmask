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
the dirs first. Pass `--real` to download the real Big Buck Bunny (Creative Commons BY 3.0, about 62
MB) in place of the synthetic stand-in so a full-length clip can be played. The smoke test (below)
generates its own fixtures separately.

The movie titles are real, TMDB-matchable names (the public-domain Blender open movies). Set
`SHADOWMASK_TMDB_API_KEY` before starting the stack and a scan will enrich them with real metadata
and artwork, so the full artwork pipeline is visible end to end. Without a key the catalog still
works but shows placeholder posters and empty metadata. One TV show is intentionally unmatchable so
the mismatch / manual-resolution behavior is also visible.

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
fixtures and is destructive to the dev databases and fixture media by design — run it only against
this disposable stack. Bring the stack up first, then run the script; see its `--help` for
environment overrides.
