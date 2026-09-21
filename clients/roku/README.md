# Shadowmask Roku Client

A SceneGraph channel for Roku devices, talking to the [Shadowmask](../../README.md) server's REST
API and playing HLS. Viewer-only: there is no admin surface, and management lives in the other
clients.

Authentication is by link code. The channel has no password entry.

## Requirements

`python3` and a container runtime, `podman` preferred and `docker` supported as a fallback. Nothing
else is installed on the host. The Node toolchain (`brighterscript`, `@rokucommunity/bslint`,
`brs-node`, `rooibos-roku`) lives inside the image built from
[`tooling/Containerfile`](tooling/Containerfile), and every command below runs in a container.

```
brew install podman && podman machine init && podman machine start   # macOS
sudo apt-get install -y podman                                       # Ubuntu
```

## Checks

```
python3 qa.py                 # everything
python3 qa.py lint test       # a subset
```

The steps are `image`, `lint`, `test`, `xml`, `scope` and `manifest`. Naming a subset runs just
those, with the image build always included first. `scope` asserts that every component XML
`<script>`-imports every file in `source/`; a component that `extends` another project component is
skipped, because it inherits the parent's scope.

`brs-cli` exits 0 even when tests fail, so `qa.py` judges the run by the reported result and treats
a missing result as a failure.

The same command runs in CI as the `roku-qa` job in
[`build.yml`](../../.github/workflows/build.yml).

## Packaging

```
python3 package.py                                # build/shadowmask-roku.zip
python3 package.py --output /tmp/channel.zip      # somewhere else
python3 package.py --skip-image                   # reuse the tooling image
python3 package.py --skip-build                   # zip whatever is already staged
```

`package.py` owns the build: it builds the tooling image, compiles the channel into `build/app` with
`bsc`, and zips that into a sideloadable archive. It is the single source for the container plumbing
(`IMAGE`, `runtime()`, `build_image()`), which `qa.py` and `sideload.py` both import rather than
repeat.

It runs in CI as the `roku-build` job in [`build.yml`](../../.github/workflows/build.yml) and as the
`roku` job in [`publish_branch.yml`](../../.github/workflows/publish_branch.yml) and
[`publish_release.yml`](../../.github/workflows/publish_release.yml), where the archive is uploaded
as `shadowmask-client-roku-<tag>.zip`.

## Sideloading

```
python3 sideload.py --target simulator            # a local brs-desktop
python3 sideload.py --target 192.168.1.42         # a real device
ROKU_DEV_PASSWORD=secret python3 sideload.py --target 192.168.1.42
telnet 192.168.1.42 8085                          # debug console
```

`sideload.py` packages the channel through `package.py`, then installs it over the device's
`plugin_install` endpoint. `--skip-build` packages whatever is already staged, and `--skip-image`
reuses the tooling image. The simulator and a real device take the same path, because both expose
the same installer.

A real device must be in developer mode, and its developer password passed as `--password` or
`ROKU_DEV_PASSWORD`.

The simulator is [brs-desktop](https://github.com/lvcabral/brs-desktop), which needs **remote
access enabled** in its settings. Its installer listens on port 80 with `rokudev`/`rokudev`, which
`--target simulator` assumes; ECP is on 8060 and the debug console on 8085 and 8080. On Linux the
installer cannot bind port 80, so start brs-desktop with `--web=<port>` and pass the same port:

```
python3 sideload.py --target simulator --port 8888
```

brs-desktop's second debug console and `deployment/dev/docker-compose.yml` both want port 8080, and
only the simulator's installer port is configurable. Move the server:

```
SHADOWMASK_PORT=8081 docker compose -f deployment/dev/docker-compose.yml up
```

The debug console replays its buffer on connect, so a capture taken after a fix still contains
errors from before it. Attribute each one to a launch first:

```
awk '/Entering .Shadowmask./ {n++} /runtime error/ {print "segment " n": " $0}' console.log
```

## Configuration

The server address is entered on the device at first run and stored in the registry, along with the
session token, the device name, the selected theme, the playback preferences and the capability
overrides. `http://` may be omitted, so `192.168.1.10:8081` is accepted. A real device needs the
machine's LAN address rather than localhost. The address can be changed from the account screen,
which signs the user out.

## Layout

| Path                  | What it is                                                             |
|-----------------------|------------------------------------------------------------------------|
| `manifest`            | channel metadata; `requires_mkv=1` is needed for MKV direct play       |
| `source/*.brs`        | shipped code, plain BrightScript                                       |
| `components/`         | SceneGraph components: `api/`, `screens/`, `widgets/` and `MainScene`  |
| `tests/*.spec.bs`     | rooibos suites, BrighterScript, never shipped                          |
| `tests/test-main.brs` | the test build's entry point, swapped in for `source/main.brs`         |
| `bsconfig.json`       | the app build                                                          |
| `bsconfig-test.json`  | the test build: adds rooibos and stages specs under `source/`          |
| `fonts/`              | bundled Roboto and its OFL licence, generated by `refresh_assets.py`   |
| `fonts/Roboto-OFL.txt`| also read at runtime, to show the licence under Account → Profile → About |
| `images/`             | PNG assets, distributed by `refresh_assets.py`, never hand-edited      |
| `images/tmdb-logo.png`| TMDB's logo, shown under Account → Profile → About; never tinted       |
| `tooling/`            | the container and its pinned npm lockfile, the only place npm exists   |

`source/` holds the testable half: `api`/`catalog`/`playback` build requests, `cards` normalizes
every server shape into one card, `images` resolves artwork URLs, `state` partitions and merges
per-user state, `detail` builds detail and dialog models, `capability` assembles what the device
reports it can decode, `about` supplies the version and attribution shown under Account → Profile →
About, and `format`/`util` are helpers. Almost all of it is pure functions over
associative arrays, which is what the headless runner can cover. Components wire nodes to those
functions and hold no rules of their own.

Four functions in `source/` read the device instead: `MeasureDecoding` in `capability`,
`DefaultDeviceName` in `session`, `TextWidth` in `tokens`, and the registry in `prefs`. Each guards
its call and returns a fallback, because the headless runner answers none of them.

Image and font assets come from the central pipeline in [`assets/`](../../assets). Run
`assets/render_icons.sh` then `assets/refresh_assets.py` to regenerate them; never edit
`images/` or `fonts/` by hand.

## Conventions

**A fixed 1920x1080 canvas.** The manifest declares `ui_resolutions=fhd`, so the coordinate space is
1920x1080 on every device and Roku downscales for HD hardware. Layout is never scaled by
`GetUIResolution()`, which is read for diagnostics only.

**Bundled fonts.** Roku's `common:/Fonts` carries `Roboto-Regular.ttf` but no bold or medium, and a
heading pointed at a system medium renders as nothing. The channel ships
`pkg:/fonts/Roboto-Regular.ttf` and `Roboto-Medium.ttf` under the SIL OFL. They are two separate
families, `Roboto` and `Roboto Medium`.

**Text is measured with the legacy font API.** `ifFontMetrics` belongs to the 2D `roFont` handed out
by `roFontRegistry`; the `Font` node a `Label` takes exposes only `uri` and `size`, and calling
`GetOneLineWidth` on it crashes. `TextWidth` in `source/tokens.brs` registers the bundled faces once
and caches an `roFont` per size and weight. `EstimatedTextWidth` is the fallback and over-measures
short strings by roughly 20%. `boundingRect()` is not used: on brs-desktop it returns the node's
width rather than the text's ink, and it is only valid post-render.

**A wrapped label's line pitch is not the font's line height.** `GetOneLineHeight()` is 28 at size
24, and wrapped lines sit on a 36px pitch. Labels set `lineSpacing` explicitly, and `TextLinePitch`
and `TextBlockHeight` are the single source for reserved height and per-line placement.

**Rounded corners are nine-patch.** `Rectangle` has no radius, so `RoundedBox` uses
`button-{fill,line}.9.png`, a 1px border with pure opaque black stretch markers. An antialiased
marker silently disables stretching.

**Names inside `components/` are not private.** A component's script scope is one flat namespace
shared with every file in `source/`, so a widget helper called `Gap` collides with a `gap` local in
`screen.brs` and fails lint in files that did not change. Prefix them.

**Dialogs are the client's own.** `ChoiceDialog` replaces the platform dialog nodes, which cannot
carry per-row icons, colour, alignment or padding. `KeyboardDialog` stays a platform node.

**Cards carry two kinds of reference**, because `/state/batch` takes leaves only: a movie or episode
has a leaf ref, a series or season has a rollup target for `/state/rollup`. A page costs one batch
call plus at most one rollup call. Grids page at 100, under the server's batch cap of 200.

## Testing

Suites are rooibos classes extending `rooibos.BaseTestSuite`, annotated with `@suite`, `@describe`
and `@it`. They are written in BrighterScript because the annotations require it; shipped source
stays plain `.brs`.

Specs must reach `source/` in the staging directory, because Roku only auto-loads `.brs` from there.
`bsconfig-test.json` maps them; a spec staged anywhere else leaves its suite class undefined and
rooibos crashes on startup. `keepAppOpen` and `sendHomeOnFinish` must both stay `false`, or the run
never terminates.
