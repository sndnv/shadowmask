# deployment

Deployment configuration for Shadowmask. The container image is built from the `Dockerfile` at
the repository root.

- [`dev`](./dev) - builds and runs the server locally for development and testing; the
  end-to-end smoke test runs against this stack.
- [`production`](./production) - a template for running the published image in production.

Optional local AI (transcription, translation, upscaling) applies to both and is documented in
[`ENRICHMENT.md`](./ENRICHMENT.md).

## Environment variables

Settings can also come from a `shadowmask.toml` next to the binary. Environment beats file, file
beats default. `SHADOWMASK_TARGET_LANGUAGES` and `SHADOWMASK_CORS_ALLOWED_ORIGINS` are
comma-separated lists; everything else is a single value. Webhook clients are TOML-only.

### Core

| Variable                      | Description                           | Default                       |
|-------------------------------|---------------------------------------|-------------------------------|
| `SHADOWMASK_DB_ROOT`          | Databases and the instance lock       | `data` (`/data` in the image) |
| `SHADOWMASK_BIND`             | Listen address                        | `0.0.0.0:8080`                |
| `SHADOWMASK_JWT_SECRET`       | Access and refresh token signing key  | `change-me-jwt-secret`        |
| `SHADOWMASK_STREAM_SECRET`    | Stream and download token signing key | `change-me-stream-secret`     |
| `SHADOWMASK_ACCESS_TTL_SECS`  | Access token lifetime                 | `3600`                        |
| `SHADOWMASK_REFRESH_TTL_SECS` | Refresh token lifetime                | `86400`                       |

### Paths

| Variable                               | Description                                      | Default                                                  |
|----------------------------------------|--------------------------------------------------|----------------------------------------------------------|
| `SHADOWMASK_TRANSCODE_CACHE`           | Transcoded segments                              | `data/transcode` (`/data/transcode`)                     |
| `SHADOWMASK_ARTWORK_CACHE`             | Artwork                                          | `data/artwork` (`/data/artwork`)                         |
| `SHADOWMASK_TRICKPLAY_CACHE`           | Trickplay sprites                                | `data/trickplay` (`/data/trickplay`)                     |
| `SHADOWMASK_SUBTITLE_CACHE`            | Subtitles                                        | `data/subtitles` (`/data/subtitles`)                     |
| `SHADOWMASK_SUBTITLE_EXTRACTION_CACHE` | Subtitle tracks pulled out of media files        | `data/subtitle-extraction` (`/data/subtitle-extraction`) |
| `SHADOWMASK_JOB_LOG_DIR`               | Per-job logs                                     | `data/job-logs` (`/data/job-logs`)                       |
| `SHADOWMASK_BOOTSTRAP_DIR`             | `libraries.toml`, `users.toml`                   | `bootstrap` (`/config/bootstrap`)                        |
| `SHADOWMASK_BASIC_CLIENT_DIR`          | Basic client served at `/ui/basic/`              | `clients/basic` (`/usr/share/shadowmask/basic`)          |
| `SHADOWMASK_PROFILE_OVERRIDES_DIR`     | JSON playback profiles, one file per device type | unset                                                    |

### Metadata providers

| Variable                                   | Description                                             | Default |
|--------------------------------------------|---------------------------------------------------------|---------|
| `SHADOWMASK_TMDB_API_KEY`                  | Enables TMDB                                            | unset   |
| `SHADOWMASK_OMDB_API_KEY`                  | Enables OMDb                                            | unset   |
| `SHADOWMASK_OPENSUBTITLES_API_KEY`         | Enables OpenSubtitles                                   | unset   |
| `SHADOWMASK_TMDB_MIN_INTERVAL_MS`          | Minimum gap between TMDB requests                       | `100`   |
| `SHADOWMASK_OMDB_MIN_INTERVAL_MS`          | Minimum gap between OMDb requests                       | `500`   |
| `SHADOWMASK_OPENSUBTITLES_MIN_INTERVAL_MS` | Minimum gap between OpenSubtitles requests              | `1000`  |
| `SHADOWMASK_TARGET_LANGUAGES`              | Subtitle fetch and translation targets, e.g. `en,bg,de` | `en`    |

### Workers and scheduling

| Variable                              | Description                                                                | Default |
|---------------------------------------|----------------------------------------------------------------------------|---------|
| `SHADOWMASK_SCAN_PROBE_CONCURRENCY`   | Files probed in parallel per scan                                          | `8`     |
| `SHADOWMASK_TRICKPLAY_THREADS`        | Cores one trickplay ffmpeg may use                                         | `2`     |
| `SHADOWMASK_TRICKPLAY_KEYFRAMES_ONLY` | Decode only keyframes for thumbnails                                       | `true`  |
| `SHADOWMASK_WORKER_PERIOD_SECS`       | Queue drain interval                                                       | `5`     |
| `SHADOWMASK_SCHEDULER_PERIOD_SECS`    | Scheduling interval                                                        | `30`    |
| `SHADOWMASK_REAPER_PERIOD_SECS`       | Dead-session reap interval                                                 | `30`    |
| `SHADOWMASK_SHUTDOWN_TIMEOUT_SECS`    | Grace period before in-flight work is killed                               | `30`    |
| `SHADOWMASK_REINDEX_EVERY_SECS`       | Search index rebuild interval                                              | `3600`  |
| `SHADOWMASK_DAILY_SCAN_AT`            | Nightly re-scan at `HH:MM`, container timezone, `scheduled` libraries only | unset   |

### Job pools

Each kind runs on one pool, up to that pool's concurrency. Unassigned kinds run on `default`.

| Pool         | Concurrency | Kinds                                     |
|--------------|-------------|-------------------------------------------|
| `default`    | `4`         | everything not named below                |
| `trickplay`  | `1`         | `trickplay`                               |
| `enrichment` | `1`         | `transcription`, `translation`, `upscale` |
| `fetch`      | `1`         | `fetch`                                   |

| Variable                                  | Description                             | Default |
|-------------------------------------------|-----------------------------------------|---------|
| `SHADOWMASK_JOB_POOLS_<POOL>_CONCURRENCY` | Jobs that pool runs at once             | `1`     |
| `SHADOWMASK_JOB_POOLS_<POOL>_KINDS`       | Comma-separated job kinds for that pool | none    |

Any `<POOL>` name creates a pool; naming no kinds removes one. An unknown kind, a kind claimed by
two pools, or a concurrency of zero stops the server at startup.

Kinds: `library_scan`, `metadata`, `artwork`, `subtitles`, `trickplay`, `fingerprint`, `dedup`,
`cache_eviction`, `search_reindex`, `ingest`, `relink`, `transcription`, `translation`, `upscale`,
`combine`, `fetch`, `scheduled_scan`, `retention`, `orphan_sweep`.

```yaml
SHADOWMASK_JOB_POOLS_DEFAULT_CONCURRENCY: "8"
SHADOWMASK_JOB_POOLS_TRICKPLAY_CONCURRENCY: "2"
SHADOWMASK_JOB_POOLS_ARTWORK_KINDS: "artwork"
SHADOWMASK_JOB_POOLS_ARTWORK_CONCURRENCY: "6"
```

### Caches, limits and retention

| Variable                               | Description                                            | Default       |
|----------------------------------------|--------------------------------------------------------|---------------|
| `SHADOWMASK_TRANSCODE_CACHE_CAP_BYTES` | Transcode cache size cap                               | `10737418240` |
| `SHADOWMASK_REMUX_READ_RATE`           | Remux read-ahead multiplier                            | `10.0`        |
| `SHADOWMASK_MAX_TRANSCODE_HEIGHT`      | Re-encode height cap; direct play and remux unaffected | unset         |
| `SHADOWMASK_CACHE_EVICTION_EVERY_SECS` | Cache sweep interval                                   | `3600`        |
| `SHADOWMASK_JOB_RETENTION_DAYS`        | Finished job and log retention                         | `30`          |
| `SHADOWMASK_RETENTION_EVERY_SECS`      | Retention interval                                     | `86400`       |
| `SHADOWMASK_ORPHAN_SWEEP_EVERY_SECS`   | Orphan sweep interval                                  | `86400`       |
| `SHADOWMASK_ORPHAN_SWEEP_GRACE_SECS`   | Age before an orphan is removable                      | `86400`       |

### Logging

| Variable                    | Description                                | Default |
|-----------------------------|--------------------------------------------|---------|
| `SHADOWMASK_LOG_LEVEL`      | Shadowmask's own crates                    | `info`  |
| `SHADOWMASK_SQLX_LOG_LEVEL` | The `sqlx` target                          | `warn`  |
| `RUST_LOG`                  | Raw `tracing` filter; overrides both above | unset   |

### Bootstrap

| Variable                    | Description                                      | Default |
|-----------------------------|--------------------------------------------------|---------|
| `SHADOWMASK_BOOTSTRAP_MODE` | `off`, `init` (apply and exit), `init-and-start` | `off`   |

Strings in the bootstrap TOML files expand `${NAME}` against the environment, so
`password = "${SHADOWMASK_ADMIN_PASSWORD}"` keeps secrets out of the files.

### Network, TLS and CORS

| Variable                          | Description                                                 | Default |
|-----------------------------------|-------------------------------------------------------------|---------|
| `SHADOWMASK_TLS_CERT`             | PEM certificate; HTTPS on the same port. Set with the key   | unset   |
| `SHADOWMASK_TLS_KEY`              | PEM private key                                             | unset   |
| `SHADOWMASK_CORS_ALLOWED_ORIGINS` | Exact origins allowed to call the API. Unset means CORS off | unset   |

### Hardware acceleration

| Variable                           | Description            | Default               |
|------------------------------------|------------------------|-----------------------|
| `SHADOWMASK_HARDWARE_ACCELERATION` | `auto`, `off`, `vaapi` | `auto`                |
| `SHADOWMASK_VAAPI_DEVICE`          | Render node            | `/dev/dri/renderD128` |

### Local AI (enrichment)

Needs the `-enrichment` image. The `*_PROVIDER` settings are informational: enabling a feature runs
the bundled provider, and external providers are not implemented. Leave them at their default.

| Variable                                          | Description                              | Default                  |
|---------------------------------------------------|------------------------------------------|--------------------------|
| `SHADOWMASK_ENRICHMENT_MODEL_CACHE`               | CTranslate2 model root                   | `data/enrichment-models` |
| `SHADOWMASK_ENRICHMENT_THREADS`                   | Cores one enrichment job may use         | half the machine         |
| `SHADOWMASK_ENRICHMENT_TRANSCRIPTION_ENABLED`     | Transcription on                         | `false`                  |
| `SHADOWMASK_ENRICHMENT_TRANSCRIPTION_PROVIDER`    | Transcription provider                   | `none`                   |
| `SHADOWMASK_ENRICHMENT_TRANSCRIPTION_MODEL_PATH`  | Model dir, relative to the model cache   | unset                    |
| `SHADOWMASK_ENRICHMENT_TRANSLATION_ENABLED`       | Translation on                           | `false`                  |
| `SHADOWMASK_ENRICHMENT_TRANSLATION_PROVIDER`      | Translation provider                     | `none`                   |
| `SHADOWMASK_ENRICHMENT_TRANSLATION_MODEL_PATH`    | Model dir, relative to the model cache   | unset                    |
| `SHADOWMASK_ENRICHMENT_TRANSLATION_SOURCE_PREFIX` | Input language token, e.g. `<2{target}>` | unset                    |
| `SHADOWMASK_ENRICHMENT_TRANSLATION_TARGET_PREFIX` | Output language token, e.g. `{target}`   | unset                    |
| `SHADOWMASK_ENRICHMENT_UPSCALING_ENABLED`         | Upscaling on                             | `false`                  |
| `SHADOWMASK_ENRICHMENT_UPSCALING_PROVIDER`        | Upscaling provider                       | `none`                   |
| `SHADOWMASK_ENRICHMENT_UPSCALING_MODEL_PATH`      | Model dir, relative to the model cache   | unset                    |
| `SHADOWMASK_ENRICHMENT_UPSCALING_TARGET_HEIGHT`   | Upscale target height                    | `1080`                   |

### Content fetch

| Variable                                       | Description                              | Default  |
|------------------------------------------------|------------------------------------------|----------|
| `SHADOWMASK_FETCH_PROVIDERS_ENABLED`           | Content fetch on                         | `false`  |
| `SHADOWMASK_FETCH_PROVIDERS_YT_DLP_BINARY`     | `yt-dlp` path                            | `yt-dlp` |
| `SHADOWMASK_FETCH_PROVIDERS_YT_DLP_PLUGIN_DIR` | Extractor plugin directory               | unset    |
| `SHADOWMASK_FETCH_PROVIDERS_MAX_HEIGHT`        | Download resolution cap                  | unset    |
| `SHADOWMASK_FETCH_PROVIDERS_COOKIES_FILE`      | Netscape `cookies.txt` for gated content | unset    |

### Web UI image

Read at container start; a change needs a restart.

| Variable                           | Description                                                                                                       | Default     |
|------------------------------------|-------------------------------------------------------------------------------------------------------------------|-------------|
| `SHADOWMASK_API_BASE`              | Where the browser reaches the server, e.g. `http://192.168.1.10:8080`. Required, and never a compose service name | none        |
| `NGINX_SERVER_NAME`                | vhost `server_name`                                                                                               | `localhost` |
| `NGINX_SERVER_PORT`                | Listen port; ending in ` ssl` requires the four values below                                                      | `80`        |
| `NGINX_SERVER_SSL_CERTIFICATE`     | Certificate path in the container                                                                                 | unset       |
| `NGINX_SERVER_SSL_CERTIFICATE_KEY` | Key path in the container                                                                                         | unset       |
| `NGINX_SERVER_SSL_PROTOCOLS`       | e.g. `TLSv1.2 TLSv1.3`                                                                                            | unset       |
| `NGINX_SERVER_SSL_CIPHERS`         | e.g. `HIGH:!aNULL:!MD5`                                                                                           | unset       |

### Compose

Read by `deployment/production/docker-compose.yml` itself, not by the server.

| Variable                   | Description                                                            | Default                  |
|----------------------------|------------------------------------------------------------------------|--------------------------|
| `SHADOWMASK_MOVIES_DIR`    | Host path mounted at `/media/movies`. Required                         | none                     |
| `SHADOWMASK_TV_DIR`        | Host path mounted at `/media/tv`. Required                             | none                     |
| `SHADOWMASK_WEB_UI_ORIGIN` | Origin the UI is served from; becomes `SHADOWMASK_CORS_ALLOWED_ORIGINS` | `http://localhost:8090`  |
| `SHADOWMASK_VERSION`       | Image tag for both services                                            | `latest`                 |
| `SHADOWMASK_PORT`          | Published server port                                                  | `8080`                   |
| `SHADOWMASK_WEB_UI_PORT`   | Published web UI port                                                  | `8090`                   |
| `SHADOWMASK_MEM_LIMIT`     | Server container memory and swap limit                                 | `8g`                     |
| `SHADOWMASK_EXTERNAL_DIR`  | Host path for fetched content, when that mount is enabled              | `./local/external`       |

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
does not resolve inside the container.

Hardware H.264 encoding is VAAPI only; NVENC is not used, so an NVIDIA card encodes in software
whatever you pass through. On an NVIDIA-only host, leave `/dev/dri` out or set `off`: `auto` enables
VAAPI whenever the render node exists, without checking what is behind it, so every segment fails
once before falling back.

The userspace VAAPI drivers ship in the image; only the render node comes from the host. The amd64
image covers Intel through `intel-media-va-driver` (Broadwell and newer) and `i965-va-driver`
(older), and AMD through `mesa-va-drivers`. The arm64 image has no Intel drivers, since that
silicon does not exist on arm, and covers AMD only. An arm board's own encoder is not reachable:
Raspberry Pi and Rockchip expose V4L2 M2M and RKMPP rather than VAAPI, so arm64 hosts without a
discrete AMD GPU encode in software.

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
  "containers": [
    "mp4",
    "hls"
  ],
  "video": [
    {
      "codec": "h264",
      "max_level": "4.2",
      "max_bit_depth": 8
    }
  ],
  "audio": [
    {
      "codec": "aac",
      "max_channels": 2
    }
  ],
  "hdr": [],
  "max_width": 1920,
  "max_height": 1080,
  "max_bitrate": 8000000
}
```

`hdr`, `max_frame_rate` and each codec's `max_level` are optional; everything else is required.
Prefer narrow: claiming a codec the device cannot play fails playback outright, while omitting one
only costs a transcode. The server refuses to start if the directory is missing or a file does not
parse.

## Content fetch

Shadowmask can pull a single video from an external site over HTTP, store it in a dedicated
"external" library, and then run the usual discovery and enrichment against it (trickplay,
transcription, translation). This is admin-only and off by default. It wraps the `yt-dlp` binary
(baked into the image), so it works for YouTube and every site yt-dlp supports.

Enable it:

```
SHADOWMASK_FETCH_PROVIDERS_ENABLED=true
SHADOWMASK_JOB_POOLS_FETCH_CONCURRENCY=1
```

Downloads run on their own job pool (`SHADOWMASK_JOB_POOLS_FETCH_CONCURRENCY`, default 1) so
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

The server checks the cookies for the target site before queueing a fetch, and rejects the request
with an error naming the expiry rather than starting a download that would fail. Cookies for other
sites, and public fetches, are unaffected.

The file holds live session secrets: mount it read-only and never commit it. Using your account to
download is subject to that site's terms.
