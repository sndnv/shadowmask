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
scan, catalog, playback, backup/recover, media-removal soft-delete). It generates its own ffmpeg
fixtures and is destructive to the dev databases and fixture media by design — run it only against
this disposable stack. Bring the stack up first, then run the script; see its `--help` for
environment overrides.
