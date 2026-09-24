# deployment / production

`docker-compose.yml` is a template for running Shadowmask in production from the published image.

## Getting started

1) Create the secrets file:
   `cp secrets/templates/shadowmask.env.template secrets/shadowmask.env` and fill in real values.
   At least `SHADOWMASK_JWT_SECRET` and `SHADOWMASK_STREAM_SECRET` are required; generate them with
   `openssl rand -hex 32`.
2) Point the library dirs at your media: export `SHADOWMASK_MOVIES_DIR` and `SHADOWMASK_TV_DIR`
   (both required).
3) Set `SHADOWMASK_API_BASE`, and `SHADOWMASK_WEB_UI_ORIGIN` if you reach the UI as anything other
   than `localhost:8090`. See [Web UI](#web-ui); the stack does not start without the first.
4) Choose an image version with `SHADOWMASK_VERSION` (defaults to `latest`, and covers both the
   server and the web UI).
5) First run only: enable bootstrap by setting `SHADOWMASK_BOOTSTRAP_MODE=init-and-start` and
   editing `bootstrap/{libraries.toml,users.toml}`. Set it back to `off` and restart afterwards.
6) Start: `docker compose up -d` (or `podman-compose up -d`).

Data (databases, caches) persists under `./local/data`. Take a snapshot with
`docker compose exec shadowmask shadowmask backup /data/snapshot.tar` and restore with
`shadowmask recover --from ...`.

## Data directory permissions

The image runs as uid `1001`, gid `0`. The server takes a lock inside `/data` before anything else,
so a data directory that uid cannot write stops it at startup with a message naming the path. Either:

- `sudo chown -R 1001:0 ./local/data` before the first start, or
- run as a different uid with `user: "1000:1000"` on the `shadowmask` service.

Any uid works as long as it can write `/data`, and `/media/external` if content fetch is on. The
media libraries are mounted read-only.

## Web UI

The `shadowmask-web-ui` service serves the Flutter web client from nginx on
`${SHADOWMASK_WEB_UI_PORT:-8090}`. The basic client stays where it is, inside the server image at
`/ui/basic/`, and needs nothing.

Two variables have to agree, and both describe what the **browser** sees rather than what the
containers see:

- `SHADOWMASK_API_BASE` (required) is where the browser reaches the server, for example
  `http://192.168.1.10:8080`. It is baked into the served bundle when the container starts, so it
  cannot be `http://shadowmask:8080`: that name only resolves inside the compose network. The
  container refuses to start when it is unset.
- `SHADOWMASK_WEB_UI_ORIGIN` (defaults to `http://localhost:8090`) is the origin the UI itself is
  served from, and it becomes the server's `SHADOWMASK_CORS_ALLOWED_ORIGINS`. Set it whenever you
  reach the UI as anything other than `localhost:8090`, or the browser blocks every API call.

So for a LAN host at `192.168.1.10` with the default ports:

```bash
export SHADOWMASK_API_BASE=http://192.168.1.10:8080
export SHADOWMASK_WEB_UI_ORIGIN=http://192.168.1.10:8090
```

nginx can terminate TLS itself: set `NGINX_SERVER_PORT` to `443 ssl`, mount a certificate and key,
and set all four `NGINX_SERVER_SSL_*` values. The entrypoint requires every one of them once the
port ends in `ssl` and exits if any is missing. The compose file has the block commented out.

## Logging

Log levels are set per target in the compose `environment:` block:

- `SHADOWMASK_LOG_LEVEL` (default `info`) - Shadowmask's own crates.
- `SHADOWMASK_SQLX_LOG_LEVEL` (default `warn`) - the `sqlx` target; raise to `debug` to log every
  query. Other dependencies stay at `warn`.
- `RUST_LOG`, if set, overrides both with a raw `tracing` filter directive.

## Hardware acceleration

The image ships FFmpeg with VAAPI drivers baked in. Uncomment the `devices` block in
`docker-compose.yml` to pass `/dev/dri` through for Intel or AMD iGPU H.264 encoding, selected with
`SHADOWMASK_HARDWARE_ACCELERATION` (`auto` by default). See
[hardware acceleration](../README.md#hardware-acceleration) for the modes and passthrough details.

## Transcode height

`SHADOWMASK_MAX_TRANSCODE_HEIGHT` caps the picture height the server will re-encode to. It is
unset by default, meaning no cap, and it applies only to transcoding; direct play and remux are
never downscaled. Set it to a number such as `1080` or `720` on a host without a hardware encoder,
where transcoding at the source resolution runs slower than realtime and playback stalls. The
resolved value is printed in the startup block as `max_transcode_height`, and each session logs the
height it resolved to.

## Local AI (enrichment)

Transcription, translation, and upscaling can run locally with no external service. These features
live in a separate image and are off by default. In short: switch the `image:` tag to the
`-enrichment` variant, enable the feature(s), and mount CTranslate2 models. The compose file has a
commented block to uncomment; full setup and licensing are in
[`../ENRICHMENT.md`](../ENRICHMENT.md).

## TLS (optional)

TLS is off by default and the server listens on plaintext HTTP. That is fine on a trusted LAN or
behind your own reverse proxy (we ship no proxy config, but we do not restrict it either: the
server is a valid plaintext upstream). To have the server terminate TLS itself, mount a
certificate and private key (PEM) and set `SHADOWMASK_TLS_CERT` and `SHADOWMASK_TLS_KEY`
(uncomment the block in `docker-compose.yml`, both must be set together).

When TLS is enabled the server serves HTTPS on the same port, so change the healthcheck to
`curl -fsSk https://127.0.0.1:8080/health` (the `-k` allows a self-signed cert).

## CORS (optional)

CORS is off by default in the server. The basic client is served by the server itself (same origin)
and needs nothing. A web client hosted on a **different** origin needs cross-origin access:
`SHADOWMASK_CORS_ALLOWED_ORIGINS` takes a comma-separated list of exact origins (scheme + host +
port), e.g. `https://watch.example.com`. Auth is Bearer-token only, so no cookie/credential
handling is involved.

The compose file already sets it from `SHADOWMASK_WEB_UI_ORIGIN` for the bundled web UI, so adjust
that variable rather than this one unless you are allowing an additional origin on top.
