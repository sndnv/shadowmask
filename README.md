# Shadowmask - Self-Hosted Media Server

<img src="./assets/brand/shadowmask.logo-retro.svg" width="64px" alt="Shadowmask Logo" align="right"/>

Shadowmask is a self-hosted media library and streaming server for movies and series. It scans your
media files, recognizes and organizes them, enriches them with metadata, artwork and subtitles, and
streams to multiple players with per-user accounts and access control.

## Features

**Streaming and playback**

* HLS adaptive streaming, direct play, and remux when the codecs already fit
* Hardware-accelerated transcoding with software fallback, and HDR to SDR tone mapping
* Resume across devices, and a per-user bitrate cap
* Multi-track audio, soft (WebVTT) and burned-in subtitles, plus external subtitle files
* Per-user, per-track subtitle sync offset, and admin-derived bilingual stacked subtitle tracks
* Chapter navigation, trickplay seek thumbnails, and autoplay of the next episode

**Library and metadata**

* Movies, series, seasons, episodes and collections, with automatic recognition and a manual
  resolution queue for what it cannot match
* Metadata from OMDB and TMDB; subtitles from OpenSubtitles
* Poster, backdrop, banner, logo and clearart variants, resized and served per request
* Content ratings as a normalized field, driving parental controls
* Per-library watcher strategies (local events, polling, scheduled, manual), plus duplicate detection

**Discovery**

* Full-text search across titles, people and genres
* Continue watching, next up, and composed home hubs

**Optional**

* Local AI enrichment: transcription and translation, run on your own hardware. See
  [`deployment/ENRICHMENT.md`](deployment/ENRICHMENT.md).
* Remote content fetch through yt-dlp

## Quick start

```
git clone https://github.com/sndnv/shadowmask.git
cd shadowmask/deployment/production
cp secrets/templates/shadowmask.env.template secrets/shadowmask.env
```

Fill in `SHADOWMASK_JWT_SECRET` and `SHADOWMASK_STREAM_SECRET` (`openssl rand -hex 32` for each) and
set an admin password, then point the stack at your libraries and start it:

```
export SHADOWMASK_MOVIES_DIR=/path/to/movies
export SHADOWMASK_TV_DIR=/path/to/tv
docker compose up -d
```

The server listens on port 8080, and `/health` reports readiness. See
[`deployment/production/README.md`](deployment/production/README.md) for TLS, bootstrap, hardware
acceleration and the rest.

## Clients

* **Web** ([`clients/flutter`](./clients/flutter)) - the primary interface, published as
  `ghcr.io/sndnv/shadowmask/web-ui`
* **Basic** ([`clients/basic`](./clients/basic)) - a dependency-free HTML/JS client served by the
  server itself at `/ui/basic/`, useful as a fallback and for debugging

## Components

A Cargo workspace where every crate depends inward on `domain`:

* [`domain`](./crates/domain) - core entities, traits, and errors; zero infrastructure dependencies
* [`persistence`](./crates/persistence) - SQLite (sqlx) implementations of the repository traits
* [`media`](./crates/media) - FFmpeg/ffprobe, transcoding, HLS, and capability negotiation
* [`metadata`](./crates/metadata) - external metadata and subtitle clients (OMDB, TMDB, OpenSubtitles)
* [`inference`](./crates/inference) - optional local AI enrichment (transcription, translation)
* [`fetch`](./crates/fetch) - optional remote content retrieval via yt-dlp
* [`services`](./crates/services) - business logic for catalog, library, streaming, and users
* [`jobs`](./crates/jobs) - background job queue, workers, and scheduler
* [`api`](./crates/api) - axum HTTP router, handlers, DTOs, and auth/RBAC
* [`server`](./crates/server) - the binary; composition root, config, and bootstrap
* [`mocks`](./crates/mocks) - in-memory trait implementations used by tests
* [`contracts`](./crates/contracts) - shared contract tests every repository implementation must pass

## Development

Refer to the [DEVELOPMENT.md](DEVELOPMENT.md) file for more details.

## Contributing

Contributions are always welcome!

Refer to the [CONTRIBUTING.md](CONTRIBUTING.md) file for more details.

## Security

Refer to the [SECURITY.md](SECURITY.md) file for how to report a vulnerability.

## Versioning

We use [SemVer](http://semver.org/) for versioning.

## Third-party content

* [hls.js](https://github.com/video-dev/hls.js) is bundled by both web clients for HLS playback,
  under the Apache License 2.0. The licence and copyright notices are distributed alongside it, at
  [`assets/vendor/hls.js.LICENSE.txt`](./assets/vendor/hls.js.LICENSE.txt).
* CTranslate2 and `ct2rs` are bundled into the enrichment image under the MIT License, with the
  operator's model licensing responsibilities described in
  [`deployment/ENRICHMENT.md`](deployment/ENRICHMENT.md#licensing-and-attribution).
* Test fixtures and dev-deployment media clips are credited in
  [`crates/media/tests/fixtures/CREDITS.md`](./crates/media/tests/fixtures/CREDITS.md) and
  [`deployment/dev/CREDITS.md`](./deployment/dev/CREDITS.md).

## License

This project is licensed under the Apache License, Version 2.0 - see the [LICENSE](LICENSE) file for details.

> Copyright 2026 https://github.com/sndnv
>
> Licensed under the Apache License, Version 2.0 (the "License");
> you may not use this file except in compliance with the License.
> You may obtain a copy of the License at
>
> http://www.apache.org/licenses/LICENSE-2.0
>
> Unless required by applicable law or agreed to in writing, software
> distributed under the License is distributed on an "AS IS" BASIS,
> WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
> See the License for the specific language governing permissions and
> limitations under the License.
