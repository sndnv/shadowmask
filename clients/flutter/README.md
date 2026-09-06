# Shadowmask Client

The primary user interface for the [Shadowmask](../../README.md) media server: one Flutter codebase
targeting the web and the desktop, talking to the server's REST API and playing HLS.

| Target  | Status        | Player                   |
|---------|---------------|--------------------------|
| Web     | Supported     | `hls.js`                 |
| macOS   | Supported     | mpv, through `media_kit` |
| Linux   | Supported     | mpv, through `media_kit` |
| Windows | Not supported | -                        |

Windows is not supported: its required native dependencies are out of date and unmaintained.

## Build and run

Prerequisites, setup and the run commands are documented in
[DEVELOPMENT.md](../../DEVELOPMENT.md#clients). In short:

```
flutter pub get
dart run build_runner build
flutter run -d chrome --web-port=8090 --dart-define=SHADOWMASK_API_BASE=http://localhost:8080
flutter run -d macos
```

`dart run build_runner build` is required; generated sources are not committed.

Add `--release` when you need a build that renders the way the published image does, for example
when capturing screenshots. The debug build also asserts on some framework layout paths that a
release build does not, notably `Tooltip` when the window is resized while a tooltip is on screen.

### Desktop prerequisites

* macOS: CocoaPods, `brew install cocoapods`.
* Linux: libmpv, `sudo apt install libmpv-dev mpv` on Debian and Ubuntu.

## Packaging

[`publish_release.yml`](../../.github/workflows/publish_release.yml) attaches a macOS `.dmg` and a
Linux `.AppImage` to the GitHub release on a `v*` tag.
[`publish_branch.yml`](../../.github/workflows/publish_branch.yml) builds both on demand and uploads
them as workflow artifacts.

Locally:

```
flutter build macos --release
create-dmg --volname "shadowmask" --window-size 800 400 --app-drop-link 600 200 \
  shadowmask.dmg build/macos/Build/Products/Release/shadowmask.app/

flutter build linux --release
appimage-builder --skip-test
```

`create-dmg` comes from Homebrew, `appimage-builder` from pip. The AppImage is described by
[`AppImageBuilder.yml`](AppImageBuilder.yml).

The AppImage does not bundle libmpv or GTK; both come from the host, so the desktop prerequisites
above apply to it as well. It is built on Ubuntu 24.04 and links `libmpv.so.2`, so it needs a host
of that vintage or newer.

The macOS build is neither signed nor notarized. On first open, right-click the app and choose Open,
or run `xattr -dr com.apple.quarantine` against it.

The macOS bundle vendors mpv, FFmpeg and nine supporting libraries. They are attributed in
[CREDITS.md](CREDITS.md), with their licence texts under [`licenses/`](../../licenses).

## Configuration

On the **web**, the API base URL comes from `SHADOWMASK_API_BASE`, read either from `--dart-define`
at build time or from `web/assets/.env` at runtime. Copy
[`deployment/dev/.env.template`](deployment/dev/.env.template) to `web/assets/.env` for local
development. In the published image,
[`deployment/production/entrypoint.sh`](deployment/production/entrypoint.sh) renders that file from
the container's environment at start.

On the **desktop** the address is entered on first run and stored on the device.
`SHADOWMASK_API_BASE` is used when nothing has been stored yet. Changing the server signs the user
out.

## Checks

```
./qa.py
```

Runs package resolution, code generation, format check, analyze, and tests with coverage.

## Deployment

[`deployment/production`](deployment/production) builds the web bundle and packages it behind nginx
as `ghcr.io/sndnv/shadowmask/web-ui`.
