# deployment

Deployment configuration for Shadowmask. The container image is built from the `Dockerfile` at
the repository root.

- [`dev`](./dev) - builds and runs the server locally for development and testing; the
  end-to-end smoke test runs against this stack.
- [`production`](./production) - a template for running the published image in production.

Optional local AI (transcription, translation, upscaling) applies to both and is documented in
[`ENRICHMENT.md`](./ENRICHMENT.md).

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
