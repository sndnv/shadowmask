#!/bin/sh
set -eu

HELP="Usage: $0 [base|enrichment|all]

Builds the Shadowmask dev images locally, using the same Dockerfile stages CI publishes:
  base           -> ghcr.io/sndnv/shadowmask/server:dev-latest              (target: runtime)
  enrichment     -> ghcr.io/sndnv/shadowmask/server:dev-latest-enrichment   (target: runtime-enrichment)
  all (default)  -> both

The enrichment image compiles CTranslate2 (cmake, g++, OpenBLAS) inside the build, so it is much
larger and slower to build than the base image; build it only when you want the local AI features
(transcription and translation). It layers on top of the base stage (FROM runtime), so building both
shares the base layers.

Container engine: uses \$CONTAINER_ENGINE if set, otherwise docker, otherwise podman.
"

choice="all"
case "${1:-all}" in
    -h | --help)
        printf '%s' "$HELP"
        exit 0
        ;;
    base | enrichment | all) choice="${1:-all}" ;;
    *)
        printf '%s' "$HELP" >&2
        exit 64
        ;;
esac

engine="${CONTAINER_ENGINE:-}"
if [ -z "$engine" ]; then
    if command -v docker >/dev/null 2>&1; then
        engine="docker"
    elif command -v podman >/dev/null 2>&1; then
        engine="podman"
    else
        printf 'No container engine found (need docker or podman); set CONTAINER_ENGINE.\n' >&2
        exit 69
    fi
fi

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
registry="ghcr.io/sndnv/shadowmask"

build_base() {
    printf 'Building base image (target: runtime)...\n'
    "$engine" build --target runtime -t "$registry/server:dev-latest" "$repo_root"
}

build_enrichment() {
    printf 'Building enrichment image (target: runtime-enrichment)...\n'
    "$engine" build --target runtime-enrichment \
        -t "$registry/server:dev-latest-enrichment" "$repo_root"
}

case "$choice" in
    base) build_base ;;
    enrichment) build_enrichment ;;
    all)
        build_base
        build_enrichment
        ;;
esac

printf 'Done.\n'
