# Shadowmask Web Client

The primary user interface for the [Shadowmask](../../README.md) media server: a Flutter application
targeting the web, talking to the server's REST API and playing HLS through `hls.js`.

## Build and run

Prerequisites, setup and the run command are documented in
[DEVELOPMENT.md](../../DEVELOPMENT.md#clients). In short:

```
flutter pub get
dart run build_runner build
flutter run -d chrome --web-port=8090 --dart-define=SHADOWMASK_API_BASE=http://localhost:8080
```

`dart run build_runner build` is required; generated sources are not committed.

Add `--release` when you need a build that renders the way the published image does, for example
when capturing screenshots. The debug build also asserts on some framework layout paths that a
release build does not, notably `Tooltip` when the window is resized while a tooltip is on screen.

## Configuration

The API base URL comes from `SHADOWMASK_API_BASE`, read either from `--dart-define` at build time or
from `web/assets/.env` at runtime. Copy [`deployment/dev/.env.template`](deployment/dev/.env.template)
to `web/assets/.env` for local development. In the published image,
[`deployment/production/entrypoint.sh`](deployment/production/entrypoint.sh) renders that file from
the container's environment at start.

## Checks

```
./qa.py
```

Runs package resolution, code generation, format check, analyze, and tests with coverage.

## Deployment

[`deployment/production`](deployment/production) builds the web bundle and packages it behind nginx as
`ghcr.io/sndnv/shadowmask/web-ui`.
