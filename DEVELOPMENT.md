## Development

The code is [Rust](https://www.rust-lang.org/); the toolchain version and components are pinned via
[`rust-toolchain.toml`](rust-toolchain.toml), so [`rustup`](https://rustup.rs/) installs the correct
toolchain automatically.

###### Downloads / Installation:

* [rustup](https://rustup.rs/) - Rust toolchain manager
* [Python 3](https://www.python.org/) - runs the QA checks
* [FFmpeg](https://ffmpeg.org/) - `ffmpeg` / `ffprobe`, for media probing and transcoding
* [Flutter](https://docs.flutter.dev/get-started/install) - for the web, desktop and mobile clients;
  CI pins the version in [`.github/workflows/build.yml`](.github/workflows/build.yml)

### Getting Started

1) Clone or fork the repo
2) Run `python3 qa.py`

`qa.py` runs all checks (format, lint, build, test, coverage); pass step names to run a subset, for
example `python3 qa.py fmt clippy`. It covers the Rust workspace only; the web client has its own
gate, below.

### Clients

Two clients live under [`clients/`](clients). [`clients/basic`](clients/basic) is a dependency-free
HTML/JS client served by the server itself at `/ui/basic/`; it needs no build step.

[`clients/flutter`](clients/flutter) is the primary client, one codebase covering the web, the
desktop and mobile:

```
cd clients/flutter
flutter pub get
dart run build_runner build
```

`build_runner` is not optional. Generated sources (`*.g.dart`, `*.freezed.dart`, `*.mocks.dart`) are
not committed, and without them every model file is missing its `part` and the analyzer reports
dozens of unrelated-looking errors.

To run it against a local server:

```
flutter run -d chrome --web-port=8090 --dart-define=SHADOWMASK_API_BASE=http://localhost:8080
```

The server must be up, and `SHADOWMASK_CORS_ALLOWED_ORIGINS` must include the port you pass to
`--web-port` (it defaults to `http://localhost:8090`). See
[`deployment/dev/README.md`](deployment/dev/README.md) for the dev stack.

The same codebase runs on the desktop with `flutter run -d macos` or `flutter run -d linux`. The
desktop asks for the server address on first run and stores it on the device, so there is no
`--dart-define` and no CORS to configure.

Desktop prerequisites: CocoaPods on macOS (`brew install cocoapods`), and libmpv on Linux
(`sudo apt install libmpv-dev mpv` on Debian and Ubuntu). Linux is not verified on hardware.
Windows is not supported.

The same codebase runs on mobile with `flutter run -d android` or `flutter run -d ios`. Mobile
prerequisites are the Android SDK, and Xcode plus CocoaPods for iOS. Mobile asks for the server
address on first run and stores it on the device, so there is no `--dart-define` and no CORS to
configure. A device can also be provisioned with a link code.

The client's own gate is `./qa.py` inside `clients/flutter`, which mirrors the root `qa.py`: package
resolution, code generation, format, analyze, test with coverage. It does not build the desktop or
mobile targets, and it does not cover the native channels, so build and verify those by hand when
the native side changes.

Desktop releases are a macOS `.dmg` and a Linux `.AppImage`, built by the publish workflows and
described in [`clients/flutter/README.md`](clients/flutter/README.md#packaging).

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

* `SHADOWMASK_LOG_LEVEL` (default `info`) - Shadowmask's own crates.
* `SHADOWMASK_SQLX_LOG_LEVEL` (default `warn`) - the `sqlx` target; raise to `debug` to log every
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
