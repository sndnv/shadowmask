# syntax=docker/dockerfile:1

FROM rust:1.99-slim-bookworm AS builder
ARG TARGETARCH
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,id=cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=server-target-${TARGETARCH},target=/build/target,sharing=locked \
    cargo build --release --locked -p server \
    && cp target/release/server /usr/local/bin/shadowmask

FROM debian:bookworm-slim AS runtime
ARG TARGETARCH
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        ffmpeg \
        mesa-va-drivers \
        libva2 \
        ca-certificates \
        curl \
    && if [ "$TARGETARCH" = "amd64" ]; then \
        apt-get install --no-install-recommends -y \
            intel-media-va-driver \
            i965-va-driver; \
    fi \
    && rm -rf /var/lib/apt/lists/* \
    && ffmpeg -hide_banner -filters | grep -qw zscale \
    && ffmpeg -hide_banner -filters | grep -qw tonemap
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        python3 \
        python3-venv \
    && python3 -m venv /opt/yt-dlp \
    && /opt/yt-dlp/bin/pip install --no-cache-dir --upgrade pip \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home --home-dir /home/shadowmask --uid 1001 --gid 0 shadowmask \
    && mkdir -p /data /config \
    && chown -R 1001:0 /data /config
ARG NOTO_CJK_COMMIT=f8d157532fbfaeda587e826d4cd5b21a49186f7c
ARG NOTO_FONTS_COMMIT=28b15b4b43b7bed62b5cf6e6b0b5ff5846270535
ARG NOTO_LGC_COMMIT=4bc63d7ebca1faed49c6c685f380ba0abc2c1941
RUN mkdir -p /usr/share/shadowmask/fonts \
    && cd /usr/share/shadowmask/fonts \
    && base="https://raw.githubusercontent.com/notofonts/noto-cjk/${NOTO_CJK_COMMIT}/Sans" \
    && curl -fsSL -o NotoSansCJKsc-Regular.otf "$base/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf" \
    && curl -fsSL -o NotoSansCJKsc-Medium.otf "$base/OTF/SimplifiedChinese/NotoSansCJKsc-Medium.otf" \
    && curl -fsSL -o LICENSE "$base/LICENSE" \
    && curl -fsSL -o NotoSans-Italic.ttf "https://raw.githubusercontent.com/notofonts/notofonts.github.io/${NOTO_FONTS_COMMIT}/fonts/NotoSans/hinted/ttf/NotoSans-Italic.ttf" \
    && curl -fsSL -o LICENSE-NotoSans "https://raw.githubusercontent.com/notofonts/latin-greek-cyrillic/${NOTO_LGC_COMMIT}/OFL.txt" \
    && printf '%s\n' \
        "2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b  NotoSansCJKsc-Regular.otf" \
        "ca094f6b0001fb048ca39ddd797a0cdb0179e1e55c6561e111c49c3e6a61d7b7  NotoSansCJKsc-Medium.otf" \
        "6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2  LICENSE" \
        "467e3f89eeca4108bb8710a2b9e0cf2281ac56d5b0609211a83776d0505eecb5  NotoSans-Italic.ttf" \
        "cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a  LICENSE-NotoSans" \
        | sha256sum -c -
COPY deployment/fonts/fonts.json /usr/share/shadowmask/fonts/fonts.json
COPY --from=builder /usr/local/bin/shadowmask /usr/local/bin/shadowmask
COPY clients/basic /usr/share/shadowmask/basic
COPY LICENSE /usr/share/doc/shadowmask/LICENSE
COPY THIRD-PARTY-LICENSES.md /usr/share/doc/shadowmask/THIRD-PARTY-LICENSES.md
COPY deployment/SERVER-IMAGE-CREDITS.md /usr/share/doc/shadowmask/CREDITS.md
RUN /opt/yt-dlp/bin/pip install --no-cache-dir --upgrade yt-dlp \
    && ln -sf /opt/yt-dlp/bin/yt-dlp /usr/local/bin/yt-dlp \
    && yt-dlp --version
LABEL org.opencontainers.image.title="Shadowmask" \
      org.opencontainers.image.description="Self-hosted media library and streaming server" \
      org.opencontainers.image.source="https://github.com/sndnv/shadowmask"
ENV SHADOWMASK_DB_ROOT=/data \
    SHADOWMASK_TRANSCODE_CACHE=/data/transcode \
    SHADOWMASK_ARTWORK_CACHE=/data/artwork \
    SHADOWMASK_TRICKPLAY_CACHE=/data/trickplay \
    SHADOWMASK_SUBTITLE_CACHE=/data/subtitles \
    SHADOWMASK_PLAYBACK_PREP_DIR=/data/playback-prep \
    SHADOWMASK_JOB_LOG_DIR=/data/job-logs \
    SHADOWMASK_SERVER_LOG_DIR=/data/server-logs \
    SHADOWMASK_BOOTSTRAP_DIR=/config/bootstrap \
    SHADOWMASK_BASIC_CLIENT_DIR=/usr/share/shadowmask/basic \
    SHADOWMASK_FONTS_DIR=/usr/share/shadowmask/fonts
WORKDIR /config
USER 1001:0
EXPOSE 8080
STOPSIGNAL SIGINT
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1
ENTRYPOINT ["shadowmask"]
CMD ["service"]

FROM rust:1.99-slim-bookworm AS builder-enrichment
ARG TARGETARCH
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
RUN --mount=type=cache,id=cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=enrichment-target-${TARGETARCH},target=/build/target,sharing=locked \
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
COPY licenses/enrichment/ /usr/share/doc/shadowmask/third-party-licenses/
USER 1001:0
