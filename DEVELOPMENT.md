## Development

The code is [Rust](https://www.rust-lang.org/); the toolchain version and components are pinned via
[`rust-toolchain.toml`](rust-toolchain.toml), so [`rustup`](https://rustup.rs/) installs the correct
toolchain automatically.

###### Downloads / Installation:

* [rustup](https://rustup.rs/) - Rust toolchain manager
* [Python 3](https://www.python.org/) - runs the QA checks
* [FFmpeg](https://ffmpeg.org/) - `ffmpeg` / `ffprobe`, for media probing and transcoding

### Getting Started

1) Clone or fork the repo
2) Run `python3 qa.py`

`qa.py` runs all checks (format, lint, build, test, coverage); pass step names to run a subset, for
example `python3 qa.py fmt clippy`.

### Dependency Updates

A full `python3 qa.py` run ends with a report-only dependency updates check. It parses `cargo
update --dry-run` and, under an `outdated dependencies found:` header, lists each dependency with a
newer semver-compatible release as `name vOld -> vNew` (or reports `all dependencies are up to
date`). It never fails the run, never writes `Cargo.lock`, and is skipped gracefully when offline.
Run it on its own with `python3 qa.py deps`, and add `cargo update --dry-run --verbose` to also see
dependencies a major version behind. Nothing is bumped automatically. Security advisories are a
separate, gating concern handled by `cargo deny check advisories`.

### Running with Docker

The container image is built from the [`Dockerfile`](Dockerfile) at the repo root (multi-stage,
multi-arch, FFmpeg with VAAPI). Compose stacks live under [`deployment/`](deployment):

```
# development: build and run locally, with first-run bootstrap and fixture media
docker compose -f deployment/dev/docker-compose.yml up --build

# multi-arch build (both platforms)
docker buildx build --platform linux/amd64,linux/arm64 -t shadowmask:local .
```

`podman` and `podman-compose` work in place of `docker` and `docker compose`. The server listens
on port 8080; `/health` reports readiness. See [`deployment/production`](deployment/production) for
the production template.

### Logging

Log levels are set per target so the service can run at `debug` without the SQL query firehose:

* `SHADOWMASK_LOG_LEVEL` (default `info`) — Shadowmask's own crates.
* `SHADOWMASK_SQLX_LOG_LEVEL` (default `warn`) — the `sqlx` target; raise to `debug` to log every
  query. All other dependencies stay at `warn`.
* `RUST_LOG`, if set, overrides both with a raw `tracing` filter directive.

### End-to-end smoke test

[`deployment/dev/scripts/run_smoke_test.sh`](deployment/dev/scripts/run_smoke_test.sh) exercises the
full product surface (auth, scan, catalog, playback, backup/recover, media-removal soft-delete)
against a running dev stack. It generates its own FFmpeg fixtures and is destructive to the dev
databases and fixture media by design. Bring the dev stack up first, then run the script.

### Backup and recovery

`shadowmask backup --help` and `shadowmask recover --help` document the process. In a container:

```
docker compose exec shadowmask shadowmask backup /data/snapshot.tar
```

### Current State

Early development (pre-v1).
