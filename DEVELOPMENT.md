## Development

The code is [Rust](https://www.rust-lang.org/); the toolchain version and components are pinned via
[`rust-toolchain.toml`](rust-toolchain.toml), so [`rustup`](https://rustup.rs/) installs the correct
toolchain automatically.

###### Downloads / Installation:

* [rustup](https://rustup.rs/) - Rust toolchain manager
* [Python 3](https://www.python.org/) - runs the QA checks
* [FFmpeg](https://ffmpeg.org/) - `ffmpeg` / `ffprobe`, for media probing and transcoding

### Getting Started

1) Clone or fork the repo
2) Run `python3 qa.py`

`qa.py` runs all checks (format, lint, build, test, coverage); pass step names to run a subset, for
example `python3 qa.py fmt clippy`.

### Current State

Early development (pre-v1).
