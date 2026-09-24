# Shadowmask Client

The primary user interface for the [Shadowmask](../../README.md) media server: one Flutter codebase
targeting web, desktop and mobile, talking to the server's REST API and playing HLS.

| Target  | Status        | Player                   |
|---------|---------------|--------------------------|
| Web     | Supported     | `hls.js`                 |
| macOS   | Supported     | mpv, through `media_kit` |
| Linux   | Supported     | mpv, through `media_kit` |
| Android | Supported     | mpv, through `media_kit` |
| iOS     | Supported     | mpv, through `media_kit` |
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

### Platform prerequisites

* macOS: CocoaPods, `brew install cocoapods`.
* Linux: libmpv, `sudo apt install libmpv-dev mpv` on Debian and Ubuntu.
* iOS: Xcode and CocoaPods.
* Android: the Android SDK. `flutter build apk --release` needs the signing keystore held in CI, so
  build `--debug` locally.

## Packaging

[`publish_release.yml`](../../.github/workflows/publish_release.yml) attaches a macOS `.dmg`, a
Linux `.AppImage`, an Android `.apk` and an unsigned iOS `.ipa` to the GitHub release on a `v*` tag.
[`publish_branch.yml`](../../.github/workflows/publish_branch.yml) builds the same set on demand and
uploads them as workflow artifacts.

Locally:

```
flutter build macos --release
create-dmg --volname "shadowmask" --window-size 800 400 --app-drop-link 600 200 \
  shadowmask.dmg build/macos/Build/Products/Release/shadowmask.app/

flutter build linux --release
appimage-builder --skip-test
```

`create-dmg` comes from Homebrew, `appimage-builder` from pip. The AppImage is described by
[`AppImageBuilder.yml`](AppImageBuilder.yml). `appimage-builder` does not run on macOS, so the
AppImage can only be built on Linux or in CI, and nothing about it is exercised until it is launched
on a host. Run it once by hand after changing the recipe.

The AppImage does not bundle libmpv or GTK; both come from the host, so the desktop prerequisites
above apply to it as well. It is built on Ubuntu 24.04 and links `libmpv.so.2`, so it needs a host
of that vintage or newer. Without libmpv it will not start, and says so in a dialog naming the
package to install rather than failing silently from a file manager.

Two parts of the recipe are load-bearing:

* `after_bundle` copies the glibc loader to `AppDir/lib64/`. `appimage-builder` makes `PT_INTERP`
  relative and `AppRun` resolves it from `runtime/compat` or `runtime/default`; only the latter is
  populated for you, so without the copy the app starts only where the host glibc is newer than the
  bundled one.
* `runtime.env` points `APPDIR_EXEC_PATH` at `shadowmask-launch`, the libmpv check, and
  `runtime.preserve` keeps its shebang absolute. `app_info.exec` must stay the ELF binary.

The macOS build is neither signed nor notarized. On first open, right-click the app and choose Open,
or run `xattr -dr com.apple.quarantine` against it. The iOS `.ipa` is built `--no-codesign`, so it
installs only by sideloading with your own signing identity. The Android `.apk` is signed in CI.

The macOS and iOS bundles vendor mpv, FFmpeg and nine supporting libraries, and the Android APK
vendors a different build of most of the same ones. They are attributed in
[CREDITS.md](CREDITS.md), with their licence texts under [`licenses/`](../../licenses).

## Configuration

On the **web**, the API base URL comes from `SHADOWMASK_API_BASE`, read either from `--dart-define`
at build time or from `web/assets/.env` at runtime. A `--dart-define` takes precedence over the
file. Copy [`deployment/dev/.env.template`](deployment/dev/.env.template) to `web/assets/.env` for
local development. In the published image,
[`deployment/production/entrypoint.sh`](deployment/production/entrypoint.sh) renders that file from
the container's environment at start, and the build passes no `--dart-define`.

On the **desktop** and on **mobile** the address is entered on first run and stored on the device.
`SHADOWMASK_API_BASE` is used when nothing has been stored yet. Changing the server signs the user
out. The address can be changed from the account screen, from the sign-in and link-code screens, and
from the screen shown when the server cannot be reached.

## Checks

```
./qa.py
```

Runs package resolution, code generation, format check, analyze, and tests with coverage.

## Deployment

[`deployment/production`](deployment/production) builds the web bundle and packages it behind nginx
as `ghcr.io/sndnv/shadowmask-web-ui`.
