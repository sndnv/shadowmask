# syntax=docker/dockerfile:1

FROM rust:1.96-slim-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked -p server \
    && cp target/release/server /usr/local/bin/shadowmask

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        ffmpeg \
        mesa-va-drivers \
        libva2 \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        python3 \
        python3-venv \
    && python3 -m venv /opt/yt-dlp \
    && /opt/yt-dlp/bin/pip install --no-cache-dir --upgrade pip yt-dlp \
    && ln -s /opt/yt-dlp/bin/yt-dlp /usr/local/bin/yt-dlp \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home --home-dir /home/shadowmask --uid 1001 --gid 0 shadowmask \
    && mkdir -p /data /config \
    && chown -R 1001:0 /data /config
COPY --from=builder /usr/local/bin/shadowmask /usr/local/bin/shadowmask
COPY clients/basic /usr/share/shadowmask/basic
LABEL org.opencontainers.image.title="Shadowmask" \
      org.opencontainers.image.description="Self-hosted media library and streaming server" \
      org.opencontainers.image.source="https://github.com/sndnv/shadowmask"
ENV SHADOWMASK_DB_ROOT=/data \
    SHADOWMASK_TRANSCODE_CACHE=/data/transcode \
    SHADOWMASK_ARTWORK_CACHE=/data/artwork \
    SHADOWMASK_TRICKPLAY_CACHE=/data/trickplay \
    SHADOWMASK_SUBTITLE_CACHE=/data/subtitles \
    SHADOWMASK_JOB_LOG_DIR=/data/job-logs \
    SHADOWMASK_BOOTSTRAP_DIR=/config/bootstrap \
    SHADOWMASK_BASIC_CLIENT_DIR=/usr/share/shadowmask/basic
WORKDIR /config
USER 1001:0
EXPOSE 8080
STOPSIGNAL SIGINT
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1
ENTRYPOINT ["shadowmask"]
CMD ["service"]

FROM rust:1.96-slim-bookworm AS builder-enrichment
WORKDIR /build
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        cmake \
        g++ \
        make \
        libopenblas-dev \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked -p server --features enrichment \
    && cp target/release/server /usr/local/bin/shadowmask

FROM runtime AS runtime-enrichment
USER root
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        libopenblas0 \
        libgomp1 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder-enrichment /usr/local/bin/shadowmask /usr/local/bin/shadowmask
COPY licenses/ /usr/share/doc/shadowmask/third-party-licenses/
USER 1001:0
