# deployment / production

`docker-compose.yml` is a template for running Shadowmask in production from the published image.

## Getting started

1) Create the secrets file:
   `cp secrets/templates/shadowmask.env.template secrets/shadowmask.env` and fill in real values.
   At least `SHADOWMASK_JWT_SECRET` and `SHADOWMASK_STREAM_SECRET` are required; generate them with
   `openssl rand -hex 32`.
2) Point the library dirs at your media: export `SHADOWMASK_MOVIES_DIR` and `SHADOWMASK_TV_DIR`
   (both required).
3) Choose an image version with `SHADOWMASK_VERSION` (defaults to `latest`).
4) First run only: enable bootstrap by setting `SHADOWMASK_BOOTSTRAP_MODE=init-and-start` and
   editing `bootstrap/{libraries.toml,users.toml}`. Set it back to `off` and restart afterwards.
5) Start: `docker compose up -d` (or `podman-compose up -d`).

Data (databases, caches) persists under `./local/data`. Take a snapshot with
`docker compose exec shadowmask shadowmask backup /data/snapshot.tar` and restore with
`shadowmask recover --from ...`.

## Logging

Log levels are set per target in the compose `environment:` block:

- `SHADOWMASK_LOG_LEVEL` (default `info`) — Shadowmask's own crates.
- `SHADOWMASK_SQLX_LOG_LEVEL` (default `warn`) — the `sqlx` target; raise to `debug` to log every
  query. Other dependencies stay at `warn`.
- `RUST_LOG`, if set, overrides both with a raw `tracing` filter directive.

## Hardware acceleration

The image ships FFmpeg with VAAPI drivers baked in. Uncomment the `devices` block to pass
`/dev/dri` through for Intel or AMD iGPU transcoding. Intel QuickSync additionally needs
`intel-media-va-driver`; NVIDIA NVENC needs the nvidia-container-toolkit on the host (see the
commented `deploy` block).
