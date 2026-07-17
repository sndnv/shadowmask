#!/usr/bin/env bash

HELP="Generate synthetic fixture media for the Shadowmask dev deployment.

Populates the dev media library dirs with a batch of small movies and TV episodes
so the catalog has something to browse and play. Every file is generated on the fly
with ffmpeg (lavfi test sources plus a sine tone); no real content is downloaded or
copied. Clips are tiny (a couple of seconds, low resolution) so a full run is fast
and uses little disk.

Pass --real to additionally download the real Big Buck Bunny (a Creative Commons
BY 3.0 short film, roughly 10 minutes) in place of the synthetic Big Buck Bunny
fixture, so a full-length clip can actually be played. The Blender mirror serves it
zipped, so this needs curl or wget plus unzip, and pulls about 62 MB over the network.

Files are laid out the way the scanner expects (it parses the file name, not the
folder, and walks sub-directories recursively):
  movies/<Title> (<Year>)/<Title> (<Year>) [<quality>].<ext>
  tv/<Show>/Season <NN>/<Show> S<NN>E<MM>.<ext>

Movie titles are real, TMDB-matchable names (the Blender open movies, which are public
domain) so that, with SHADOWMASK_TMDB_API_KEY set, a scan enriches them with real
metadata and artwork. One TV show ('Untitled Test Show') is intentionally unmatchable so
the mismatch / manual-resolution path is visible too.

After generating, trigger a library scan (admin UI, or POST /api/v1/libraries/{id}/scan)
so the server ingests the new files.

This writes only into the media dirs and is additive by default. Pass --reset to clear
them first. It does not touch the databases; the smoke test generates its own fixtures
separately.

Usage: $0 [--reset] [--real] [-h|--help]

Options:
  --reset        delete existing files under the movies and tv dirs before generating
  --real         download the real Big Buck Bunny (CC BY 3.0) instead of a synthetic
                 stand-in; needs curl or wget plus unzip, and about 62 MB of transfer
  -h, --help     show this help and exit

Environment overrides:
  MOVIES_DIR     movies library dir (default <deployment/dev>/media/movies)
  TV_DIR         tv library dir     (default <deployment/dev>/media/tv)"

RESET=0
REAL=0
for arg in "$@"; do
    case "$arg" in
        -h|--help) printf '%s\n' "$HELP"; exit 0 ;;
        --reset) RESET=1 ;;
        --real) REAL=1 ;;
        *) printf 'Unknown option: [%s]\n\n%s\n' "$arg" "$HELP" >&2; exit 64 ;;
    esac
done

if ((BASH_VERSINFO[0] < 4)); then
    printf 'Error: bash 4+ required, found [%s.%s].\n' "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}" >&2
    exit 1
fi

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
DEV_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
MOVIES_DIR="${MOVIES_DIR:-$DEV_DIR/media/movies}"
TV_DIR="${TV_DIR:-$DEV_DIR/media/tv}"
START_TS=$(date +%s)
SECTION_N=0
FILE_COUNT=0

if [[ -t 2 && -z "${NO_COLOR:-}" ]]; then
    C0=$'\033[0m'; DIM=$'\033[2m'; BOLD=$'\033[1m'
    GREEN=$'\033[32m'; RED=$'\033[31m'; YELLOW=$'\033[33m'; CYAN=$'\033[36m'
else
    C0=''; DIM=''; BOLD=''; GREEN=''; RED=''; YELLOW=''; CYAN=''
fi

now() { date +"%Y-%m-%dT%H:%M:%SZ"; }
ts() { printf '%s[%s]%s' "$DIM" "$(now)" "$C0"; }
log() { printf '%s %s\n' "$(ts)" "$*" >&2; }
ok() { printf '%s       %sok:%s %s\n' "$(ts)" "$GREEN" "$C0" "$*" >&2; }
warn() { printf '%s     %swarn:%s %s\n' "$(ts)" "$YELLOW" "$C0" "$*" >&2; }
die() { printf '%s     %sFAIL: %s%s\n' "$(ts)" "$RED$BOLD" "$*" "$C0" >&2; exit 1; }

section() {
    SECTION_N=$((SECTION_N + 1))
    printf '\n%s %s>: [%02d]: %s%s\n' "$(ts)" "$BOLD$CYAN" "$SECTION_N" "$*" "$C0" >&2
}
note() { printf '%s     %s... %s%s\n' "$(ts)" "$DIM" "$*" "$C0" >&2; }

teardown() {
    local ec=$?
    log "generated [$FILE_COUNT] file(s) in [$(( $(date +%s) - START_TS ))]s (exit [$ec])"
}
trap teardown EXIT

command -v ffmpeg >/dev/null 2>&1 || die "ffmpeg not found on PATH; install it and retry"

PATTERNS=(testsrc testsrc2 smptebars rgbtestsrc yuvtestsrc smptehdbars)
FREQS=(220 294 330 392 440 523 587 660)

gen_fixture() {
    local out="$1" pattern="$2" freq="$3" dur="${4:-2}"
    mkdir -p "$(dirname "$out")"
    ffmpeg -nostdin -loglevel error -y \
        -f lavfi -i "${pattern}=duration=${dur}:size=320x240:rate=15" \
        -f lavfi -i "sine=frequency=${freq}:duration=${dur}" \
        -c:v libx264 -pix_fmt yuv420p -b:v 500k \
        -c:a aac -ac 2 -shortest "$out"
    FILE_COUNT=$((FILE_COUNT + 1))
    note "wrote [$out]"
}

write_srt() {
    local out="$1"
    mkdir -p "$(dirname "$out")"
    printf '1\n00:00:01,000 --> 00:00:04,000\nBig Buck Bunny (dummy subtitle)\n\n2\n00:00:05,000 --> 00:00:08,000\nGenerated for subtitle testing.\n' >"$out"
    FILE_COUNT=$((FILE_COUNT + 1))
    note "wrote sidecar [$out]"
}

BBB_URL="https://download.blender.org/peach/bigbuckbunny_movies/BigBuckBunny_320x180.mp4.zip"

DOWNLOADER=""
if command -v curl >/dev/null 2>&1; then
    DOWNLOADER=curl
elif command -v wget >/dev/null 2>&1; then
    DOWNLOADER=wget
fi

fetch() {
    local url="$1" out="$2"
    if [[ "$DOWNLOADER" == curl ]]; then
        curl -fSL --retry 3 -o "$out" "$url"
    else
        wget -q -O "$out" "$url"
    fi
}

fetch_movie_zipped() {
    local title="$1" year="$2" quality="$3" ext="$4" url="$5"
    local dir="$MOVIES_DIR/$title ($year)"
    local suffix=""
    [[ -n "$quality" ]] && suffix=" $quality"
    local out="$dir/$title ($year)$suffix.$ext"
    mkdir -p "$dir"
    local tmp; tmp=$(mktemp)
    note "downloading [$title] from [$url]"
    fetch "$url" "$tmp" || { rm -f "$tmp"; die "download failed for [$title]"; }
    unzip -p "$tmp" >"$out" || { rm -f "$tmp"; die "unzip failed for [$title]"; }
    rm -f "$tmp"
    FILE_COUNT=$((FILE_COUNT + 1))
    ok "downloaded + extracted [$out]"
}

pick() {
    local -n arr="$1"
    printf '%s' "${arr[$(( $2 % ${#arr[@]} ))]}"
}

if ((RESET)); then
    section "reset"
    for dir in "$MOVIES_DIR" "$TV_DIR"; do
        if [[ -d "$dir" ]]; then
            find "$dir" -type f -delete 2>/dev/null || true
            find "$dir" -mindepth 1 -type d -empty -delete 2>/dev/null || true
            ok "cleared [$dir]"
        fi
    done
fi

mkdir -p "$MOVIES_DIR" "$TV_DIR"

section "movies -> [$MOVIES_DIR]"
i=0
add_movie() {
    local title="$1" year="$2" quality="$3" ext="$4"
    local dir="$MOVIES_DIR/$title ($year)"
    local suffix=""
    [[ -n "$quality" ]] && suffix=" $quality"
    gen_fixture "$dir/$title ($year)$suffix.$ext" "$(pick PATTERNS "$i")" "$(pick FREQS "$i")"
    i=$((i + 1))
}
if ((REAL)); then
    [[ -n "$DOWNLOADER" ]] || die "--real needs curl or wget on PATH"
    command -v unzip >/dev/null 2>&1 || die "--real needs unzip on PATH (apt install unzip)"
    fetch_movie_zipped "Big Buck Bunny" 2008 "" mp4 "$BBB_URL"
    write_srt "$MOVIES_DIR/Big Buck Bunny (2008)/Big Buck Bunny (2008).en.srt"
else
    add_movie "Big Buck Bunny"  2008 "1080p" mkv
    add_movie "Big Buck Bunny"  2008 "720p"  mp4
    write_srt "$MOVIES_DIR/Big Buck Bunny (2008)/Big Buck Bunny (2008) 1080p.en.srt"
fi
add_movie "Sintel"          2010 "1080p" mkv
add_movie "Tears of Steel"  2012 "2160p" mkv
add_movie "Elephants Dream" 2006 ""      mp4
add_movie "Cosmos Laundromat" 2015 "1080p" mkv
add_movie "Spring"          2019 ""      mp4
ok "movies done"

section "tv -> [$TV_DIR]"
add_show() {
    local show="$1"; shift
    local season=0
    for eps in "$@"; do
        season=$((season + 1))
        local sdir; sdir=$(printf '%s/Season %02d' "$TV_DIR/$show" "$season")
        for ((ep = 1; ep <= eps; ep++)); do
            local tag; tag=$(printf 'S%02dE%02d' "$season" "$ep")
            local ext=mkv
            (( ep % 3 == 0 )) && ext=mp4
            gen_fixture "$sdir/$show $tag.$ext" "$(pick PATTERNS "$i")" "$(pick FREQS "$i")"
            i=$((i + 1))
        done
    done
    ok "show [$show] done"
}
add_show "Pioneer One" 3
add_show "Untitled Test Show" 2 2

section "next steps"
note "for real artwork + metadata, set [SHADOWMASK_TMDB_API_KEY] before starting the stack"
note "trigger a scan so the server ingests the new files:"
note "  - admin UI: open a library and click [Trigger scan]"
note "  - or: POST /api/v1/libraries/{id}/scan"
ok "done"
