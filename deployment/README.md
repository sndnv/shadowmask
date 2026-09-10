# deployment

Deployment configuration for Shadowmask. The container image is built from the `Dockerfile` at
the repository root.

- [`dev`](./dev) - builds and runs the server locally for development and testing; the
  end-to-end smoke test runs against this stack.
- [`production`](./production) - a template for running the published image in production.

Optional local AI (transcription, translation, upscaling) applies to both and is documented in
[`ENRICHMENT.md`](./ENRICHMENT.md).

## Scanning

**Nothing is scanned automatically unless you ask for it.** A scan otherwise runs only when an
admin triggers one or a webhook does.

Turn on the nightly re-scan with `SHADOWMASK_DAILY_SCAN_AT`, a clock time as `HH:MM`. It runs in
the container's timezone, which is UTC unless you set `TZ`. Leave it unset to disable.

The nightly run is opt in per library: it only queues a scan for libraries whose `watcher` is
`scheduled` and that hold local files. Set that in `bootstrap/libraries.toml` before first start,
or on the library in the admin UI afterwards. A library already scanning is left alone.

A re-scan is cheap on a settled library: titles you already have are not re-fetched and versions
whose file has not changed are skipped entirely. New files, changed files, and titles that appear
for the first time are picked up as usual. To deliberately re-pull metadata and artwork for
everything in a library, use "Refresh metadata" on the library's admin page rather than a scan. It
queues one job per title, replaces descriptions, ratings and artwork, and leaves the files alone.

## Hardware acceleration

Video transcoding (live HLS streaming and the offline upscale job) uses the software H.264 encoder
(`libx264`) by default. When a VAAPI render node is available it offloads H.264 encoding to the GPU.
This applies to every image and is independent of the optional AI enrichment features. It is
controlled by `SHADOWMASK_HARDWARE_ACCELERATION`:

- `auto` (default) - use VAAPI when the render node exists, otherwise software.
- `off` - always software.
- `vaapi` - force VAAPI; if the render node is missing it logs a warning and falls back to software.

The render node defaults to `/dev/dri/renderD128` (override with `SHADOWMASK_VAAPI_DEVICE`). If a
hardware encode fails at runtime it is retried once in software, so playback is not interrupted.

VAAPI is Linux-only and needs the GPU passed into the container. It is not enabled in the compose
files by default (a required device would break hosts without a GPU, including macOS). On a host
with an Intel or AMD iGPU, pass the render node through and grant access:

```
    devices:
      - "/dev/dri:/dev/dri"
    group_add:
      - "video"
      - "render"
```

The `render` group id can differ between host distributions; use the numeric gid if the group name
does not resolve inside the container. Intel QuickSync uses the same VAAPI path (its driver is baked
into the image). Hardware H.264 encoding is VAAPI-only; NVIDIA NVENC is not used by the encoder.

## Playback profiles

Every client names a device type when it starts a session, and the server resolves that to a
built-in profile saying what the device can play. Clients that can measure their own decode support
report it as well, and the report overrides the built-in field by field, so the profile is only the
starting point on those devices.

For a device that cannot measure and that the built-in gets wrong, point
`SHADOWMASK_PROFILE_OVERRIDES_DIR` at a directory of JSON profile files. The file name without its
extension is the device type, so `roku.json` replaces the built-in `roku` profile and
`lounge-tv.json` adds a new type a client can name for itself.

```json
{
  "containers": ["mp4", "hls"],
  "video": [{ "codec": "h264", "max_level": "4.2", "max_bit_depth": 8 }],
  "audio": [{ "codec": "aac", "max_channels": 2 }],
  "hdr": [],
  "max_width": 1920,
  "max_height": 1080,
  "max_bitrate": 8000000
}
```

`hdr`, `max_frame_rate` and each codec's `max_level` are optional; everything else is required.
Claiming something a device cannot actually play fails playback outright, which is worse than the
transcode that leaving it out costs, so prefer narrow. The server refuses to start if the directory
is missing or a file does not parse, rather than quietly falling back to the built-in you meant to
replace.

## Content fetch

Shadowmask can pull a single video from an external site over HTTP, store it in a dedicated
"external" library, and then run the usual discovery and enrichment against it (trickplay,
transcription, translation). This is admin-only and off by default. It wraps the `yt-dlp` binary
(baked into the image), so it works for YouTube and every site yt-dlp supports.

Enable it:

```
SHADOWMASK_FETCH_PROVIDERS_ENABLED=true
SHADOWMASK_FETCH_PROVIDERS_CONCURRENCY=1
```

Downloads run on their own capped worker queue (`SHADOWMASK_FETCH_PROVIDERS_CONCURRENCY`, default 1) so
they never all run at once and never block ordinary jobs. Each fetch is stored under the root of the
external library you pick, so create one external library per destination directory (for example a
"YouTube" library rooted at one path and another library rooted at a different path). The library
root must be a writable path inside the container. Then use the admin "Fetch content" page (URL,
type, title, optional IMDb id, and season and episode for TV). Override the binary location with
`SHADOWMASK_FETCH_PROVIDERS_YT_DLP_BINARY` if you mount your own.

By default yt-dlp downloads the highest quality available (up to 4K). To cap the download resolution
and save disk, set a maximum height:

```
SHADOWMASK_FETCH_PROVIDERS_MAX_HEIGHT=1080
```

Unset means best quality. Playback still scales down to whatever the client requests, so a 1080 cap
is a good fit if you never need the 4K source stored.

### Sites yt-dlp does not support natively

For a site with no built-in yt-dlp extractor, supply a yt-dlp extractor plugin and mount its
directory into the container:

```
SHADOWMASK_FETCH_PROVIDERS_YT_DLP_PLUGIN_DIR=/config/yt-dlp-plugins
```

The directory is passed to yt-dlp as `--plugin-dirs`. Plugins are site-specific and are your
responsibility; none ship with Shadowmask. Pulling from a third-party site is subject to that
site's terms and the content's rights.

### Sites that require an account

Some sites only serve certain content to logged-in accounts (member-only videos, region-locked
overseas catalogs, paid tiers). To fetch those, export your session cookies from a browser where
you are signed in, save them as a Netscape-format `cookies.txt`, mount the file into the container,
and point at it:

```
SHADOWMASK_FETCH_PROVIDERS_COOKIES_FILE=/config/cookies.txt
```

The file is passed to yt-dlp as `--cookies`. Shadowmask deliberately does not use
`--cookies-from-browser`, because the server is headless and has no browser profile to read.

When you start a fetch, the server checks the cookies for that site first. If the cookies that apply to
the target site have all expired, the request is rejected right away with a clear error telling you to
re-export the file, instead of queueing a download that would fail. Cookies that do not apply to the
target site (and public fetches) are never blocked by this check.

That file holds live session secrets. Mount it read-only, never commit it, and treat it like a
password. Rotate it if a session expires. Using your account to download is subject to that site's
terms; this is intended for personal use of content you can already access.
