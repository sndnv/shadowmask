# Shadowmask - Self-Hosted Media Server

<img src="./assets/shadowmask.logo.svg" width="64px" alt="Shadowmask Logo" align="right"/>

Shadowmask is a self-hosted media library and streaming server for movies and TV shows.

## Components

A Cargo workspace where every crate depends inward on `domain`:

* [`domain`](./crates/domain) - core entities, traits, and errors; zero infrastructure dependencies
* [`persistence`](./crates/persistence) - SQLite (sqlx) implementations of the repository traits
* [`media`](./crates/media) - FFmpeg/ffprobe, transcoding, HLS, and capability negotiation
* [`metadata`](./crates/metadata) - external metadata and subtitle clients (OMDB, TMDB, OpenSubtitles)
* [`services`](./crates/services) - business logic for catalog, library, streaming, and users
* [`jobs`](./crates/jobs) - background job queue, workers, and scheduler
* [`api`](./crates/api) - axum HTTP router, handlers, DTOs, and auth/RBAC
* [`server`](./crates/server) - the binary; composition root, config, and bootstrap

## Development

Refer to the [DEVELOPMENT.md](DEVELOPMENT.md) file for more details.

## Contributing

Contributions are always welcome!

Refer to the [CONTRIBUTING.md](CONTRIBUTING.md) file for more details.

## Versioning

We use [SemVer](http://semver.org/) for versioning.

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
