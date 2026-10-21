#!/usr/bin/env bash

HELP="Playback check for Shadowmask.

Starts a throwaway server (its own compose project and volume, nothing shared with the dev
stack), scans test media, and plays every subject in two players the clients use:
  chromium  Debian's Chromium with basic's hls.js, as basic and Flutter web play a stream
  mpv       Ubuntu 24.04's mpv 0.37 with media_kit's options, as the desktop and Android apps do
Each case is checked for playback, position after a resume or seek, subtitle lines shown once,
audio, and player errors. See 'Playback check' in deployment/dev/README.md.

Default mode generates the playback test set (generate_media.sh --playback) once into
deployment/dev/media/playback and writes the report to deployment/dev/media/playback/out.

Library mode (--movies and/or --tv) mounts a real catalog read-only, probes every file's
headers, groups files by shape and checks --per-shape files of each. Only the sampled movie
folders and show folders are scanned. The report names real titles, so it goes to a folder
outside the repo (default under \$TMPDIR). The catalog must be visible to the container
engine (for podman on macOS, a path under \$HOME).

Each mpv case writes its player log to logs/mpv/<subject number>-<action>.log in the report
folder; the report names the log of each failed case.

In the test set, each case's negotiations (mode, container, reasons and whether the server took
the client's report, read from the server log) must match deployment/dev/playback/modes.json;
the run's own are written to modes.json in the report folder.

The run exits 1 when any case fails."
USAGE="Usage: $0 [--rebuild] [--keep] [--accept-modes] [--player NAME] [--only REGEX] [--movies DIR] [--tv DIR] [--per-shape N] [--census-only] [--out DIR] [-h|--help]

Options:
  --rebuild       build the server image from this tree (build_images.sh base) even if present
  --accept-modes  record this run's negotiations in deployment/dev/playback/modes.json instead of
                  checking them (only the cases that ran are replaced)
  --keep          leave the server and its volume running after the run
  --player NAME   run one player only: chromium or mpv
  --only REGEX    run only the cases whose \"<subject> | <action>\" matches, e.g. 'F[23] .*far-seek'
  --movies DIR    library mode: a movies catalog to sample
  --tv DIR        library mode: a TV catalog to sample
  --per-shape N   library mode: files checked per shape (default 2)
  --census-only   library mode: stop after the census
  --out DIR       report folder
  -h, --help      show this help and exit

Environment overrides:
  CONTAINER_ENGINE                   docker or podman (default: docker, else podman)
  SHADOWMASK_PLAYBACK_SERVER_IMAGE   server image (default ghcr.io/sndnv/shadowmask-server:dev-latest)
  PLAYBACK_PREPARE_TIMEOUT_S         how long the scan and playback prep may take (default 1800)"

REBUILD=0
KEEP=0
ACCEPT=0
MOVIES=""
TV=""
PER_SHAPE=2
CENSUS_ONLY=0
OUT=""
PLAYER=""
ONLY=""
while (($# > 0)); do
    case "$1" in
        -h|--help) printf '%s\n\n%s\n' "$HELP" "$USAGE"; exit 0 ;;
        --rebuild) REBUILD=1 ;;
        --keep) KEEP=1 ;;
        --accept-modes) ACCEPT=1 ;;
        --player) PLAYER="${2:?--player needs chromium or mpv}"; shift ;;
        --only) ONLY="${2:?--only needs a pattern}"; shift ;;
        --movies) MOVIES="${2:?--movies needs a directory}"; shift ;;
        --tv) TV="${2:?--tv needs a directory}"; shift ;;
        --per-shape) PER_SHAPE="${2:?--per-shape needs a number}"; shift ;;
        --census-only) CENSUS_ONLY=1 ;;
        --out) OUT="${2:?--out needs a directory}"; shift ;;
        *) printf 'Unknown option: [%s]\n\n%s\n' "$1" "$USAGE" >&2; exit 64 ;;
    esac
    shift
done

case "$PLAYER" in
    ""|chromium|mpv) ;;
    *) printf 'Unknown player: [%s]; use chromium or mpv\n\n%s\n' "$PLAYER" "$USAGE" >&2; exit 64 ;;
esac

if ((BASH_VERSINFO[0] < 4)); then
    printf 'Error: bash 4+ required, found [%s.%s].\n' "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}" >&2
    exit 1
fi

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
DEV_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
CHECK_DIR="$DEV_DIR/playback"
SET_DIR="$DEV_DIR/media/playback"
SERVER_IMAGE="${SHADOWMASK_PLAYBACK_SERVER_IMAGE:-ghcr.io/sndnv/shadowmask-server:dev-latest}"
MPV_IMAGE="localhost/shadowmask-playback-mpv:latest"
BROWSER_IMAGE="localhost/shadowmask-playback-browser:latest"
DENO=(deno run --allow-net --allow-read --allow-write --allow-env --allow-run)
LIBRARY=0
[[ -n "$MOVIES" || -n "$TV" ]] && LIBRARY=1
START_TS=$(date +%s)
SECTION_N=0

if [[ -t 2 && -z "${NO_COLOR:-}" ]]; then
    C0=$'\033[0m'; DIM=$'\033[2m'; BOLD=$'\033[1m'
    GREEN=$'\033[32m'; RED=$'\033[31m'; CYAN=$'\033[36m'
else
    C0=''; DIM=''; BOLD=''; GREEN=''; RED=''; CYAN=''
fi

now() { date +"%Y-%m-%dT%H:%M:%SZ"; }
ts() { printf '%s[%s]%s' "$DIM" "$(now)" "$C0"; }
log() { printf '%s %s\n' "$(ts)" "$*" >&2; }
ok() { printf '%s       %sok:%s %s\n' "$(ts)" "$GREEN" "$C0" "$*" >&2; }
die() { printf '%s     %sFAIL: %s%s\n' "$(ts)" "$RED$BOLD" "$*" "$C0" >&2; exit 1; }
section() {
    SECTION_N=$((SECTION_N + 1))
    printf '\n%s %s>: [%02d]: %s%s\n' "$(ts)" "$BOLD$CYAN" "$SECTION_N" "$*" "$C0" >&2
}

ENGINE="${CONTAINER_ENGINE:-}"
if [[ -z "$ENGINE" ]]; then
    if command -v docker >/dev/null 2>&1; then
        ENGINE="docker"
    elif command -v podman >/dev/null 2>&1; then
        ENGINE="podman"
    else
        die "no container engine found (need docker or podman); set CONTAINER_ENGINE"
    fi
fi
COMPOSE=("$ENGINE" compose -f "$CHECK_DIR/compose.yml")

absolute() { (cd "$1" && pwd); }
has_image() { "$ENGINE" image inspect "$1" >/dev/null 2>&1; }

if ((LIBRARY)); then
    OUT="${OUT:-${TMPDIR:-/tmp}/shadowmask-playback-$(date +%Y%m%d-%H%M%S)}"
else
    OUT="${OUT:-$SET_DIR/out}"
fi
mkdir -p "$OUT"
OUT=$(absolute "$OUT")

teardown() {
    local ec=$?
    "${COMPOSE[@]}" logs --no-color server >"$OUT/server.log" 2>/dev/null || true
    [[ -s "$OUT/server.log" ]] || rm -f "$OUT/server.log"
    if ((KEEP)); then
        log "left the server running; stop it with: ${COMPOSE[*]} down -v"
    else
        "${COMPOSE[@]}" down -v >/dev/null 2>&1 || true
    fi
    log "finished in [$(($(date +%s) - START_TS))]s (exit [$ec]); report in [$OUT]"
}

section "images"
if ((REBUILD)) || ! has_image "$SERVER_IMAGE"; then
    log "building the server image [$SERVER_IMAGE]"
    CONTAINER_ENGINE="$ENGINE" "$SCRIPT_DIR/build_images.sh" base
fi
has_image "$MPV_IMAGE" || "$ENGINE" build -f "$CHECK_DIR/mpv.Containerfile" -t "$MPV_IMAGE" "$CHECK_DIR"
has_image "$BROWSER_IMAGE" || "$ENGINE" build -f "$CHECK_DIR/browser.Containerfile" -t "$BROWSER_IMAGE" "$CHECK_DIR"
ok "server [$SERVER_IMAGE], players [$MPV_IMAGE] [$BROWSER_IMAGE]"

section "media"
if ((LIBRARY)); then
    mkdir -p "$OUT/empty"
    MOVIES_DIR=$(absolute "${MOVIES:-$OUT/empty}")
    TV_DIR=$(absolute "${TV:-$OUT/empty}")
    BOOTSTRAP="$OUT/bootstrap"
else
    SET_VERSION=$(sed -n 's/^PLAYBACK_SET_VERSION=//p' "$SCRIPT_DIR/generate_media.sh")
    if [[ "$(cat "$SET_DIR/.complete" 2>/dev/null)" != "$SET_VERSION" ]]; then
        log "generating the playback test set [$SET_VERSION] (a few minutes, once)"
        "$ENGINE" run --rm -e NO_COLOR=1 -e PLAYBACK_DIR=/work/dev/media/playback \
            -e PLAYBACK_PGS=/work/fixtures/supsample.mkv -v "$DEV_DIR:/work/dev" \
            -v "$DEV_DIR/../../crates/media/tests/fixtures:/work/fixtures:ro" \
            "$MPV_IMAGE" bash /work/dev/scripts/generate_media.sh --playback
    fi
    MOVIES_DIR="$SET_DIR/movies"
    TV_DIR="$SET_DIR/tv"
    BOOTSTRAP="$DEV_DIR/config/bootstrap"
fi
export PLAYBACK_SERVER_IMAGE="$SERVER_IMAGE" PLAYBACK_BOOTSTRAP="$BOOTSTRAP" \
    PLAYBACK_MOVIES="$MOVIES_DIR" PLAYBACK_TV="$TV_DIR" PLAYBACK_OUT="$OUT"
rm -f "$OUT"/results-*.json "$OUT/versions.json" "$OUT/report.md"
rm -rf "$OUT/logs"
mkdir -p "$BOOTSTRAP"
ok "movies [$MOVIES_DIR], tv [$TV_DIR]"

trap teardown EXIT

if ((LIBRARY)); then
    section "census"
    "${COMPOSE[@]}" --profile tools run --rm -e PLAYBACK_PER_SHAPE="$PER_SHAPE" mpv \
        "${DENO[@]}" /work/playback/census.ts
    ok "census in [$OUT/census.md]"
    if ((CENSUS_ONLY)); then
        exit 0
    fi
fi

section "server"
"${COMPOSE[@]}" up -d server
"${COMPOSE[@]}" --profile tools run --rm -e PLAYBACK_LIBRARY="$LIBRARY" \
    -e PLAYBACK_PREPARE_TIMEOUT_S="${PLAYBACK_PREPARE_TIMEOUT_S:-1800}" mpv "${DENO[@]}" /work/playback/prepare.ts
ok "scanned and prepared"

if [[ -z "$PLAYER" || "$PLAYER" == "chromium" ]]; then
    section "chromium"
    "${COMPOSE[@]}" --profile tools run --rm -e PLAYBACK_ONLY="$ONLY" browser "${DENO[@]}" /work/playback/browser.ts \
        || log "the chromium run stopped early; its results may be missing"
fi

if [[ -z "$PLAYER" || "$PLAYER" == "mpv" ]]; then
    section "mpv"
    "${COMPOSE[@]}" --profile tools run --rm -e PLAYBACK_ONLY="$ONLY" mpv "${DENO[@]}" /work/playback/mpv.ts \
        || log "the mpv run stopped early; its results may be missing"
fi

section "report"
"${COMPOSE[@]}" logs --no-color server >"$OUT/server.log" 2>/dev/null || true
REPORT_EC=0
"${COMPOSE[@]}" --profile tools run --rm -e PLAYBACK_ACCEPT="$ACCEPT" mpv "${DENO[@]}" /work/playback/report.ts \
    || REPORT_EC=$?
if ((ACCEPT)) && ((!LIBRARY)) && [[ -s "$OUT/modes.json" ]]; then
    cp "$OUT/modes.json" "$CHECK_DIR/modes.json"
    ok "recorded the negotiations in [$CHECK_DIR/modes.json]"
fi
exit "$REPORT_EC"
