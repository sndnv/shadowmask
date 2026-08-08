#!/usr/bin/env bash

HELP="Generate synthetic fixture media for the Shadowmask dev deployment.

Populates the dev media library dirs with a batch of small movies and TV episodes
so the catalog has something to browse and play. Every file is generated on the fly
with ffmpeg (lavfi test sources plus a sine tone); no real content is downloaded or
copied. Clips are tiny (a couple of seconds, low resolution) so a full run is fast
and uses little disk.

One movie ('HDR Sample') is a 10-bit HDR10 (PQ / bt2020) clip so the HDR to SDR tone-mapping
transcode path can be exercised; generating it requires an ffmpeg built with the libx265 encoder.

Pass --real to additionally download two real Creative Commons BY 3.0 clips in place
of their synthetic stand-ins: Big Buck Bunny (Blender) and a short Elephants Dream
clip (Blender / Netherlands Media Art Institute, trimmed to 90s; it has clear speech
for the transcription smoke test). Downloads are cached under media/.cache and reused
on later runs and by the smoke test, so nothing is re-downloaded.

Attribution is in deployment/dev/CREDITS.md.

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
  --real         download real CC BY 3.0 clips (Big Buck Bunny + Elephants Dream)
                 instead of synthetic stand-ins; needs curl or wget plus unzip; cached
                 under media/.cache and shared with the smoke test (no re-download)
  -h, --help     show this help and exit

Environment overrides:
  MOVIES_DIR             movies library dir (default <deployment/dev>/media/movies)
  TV_DIR                 tv library dir     (default <deployment/dev>/media/tv)
  SHADOWMASK_CLIP_CACHE  download cache dir (default <deployment/dev>/media/.cache)"

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
source "$SCRIPT_DIR/clips.sh"
CLIP_CACHE_DIR=$(clip_cache_dir "$DEV_DIR")
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
ffmpeg -hide_banner -encoders 2>/dev/null | grep -qw libx265 \
    || die "ffmpeg has no libx265 encoder; it is required to generate the HDR fixture (install an ffmpeg built with libx265)"

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

gen_hdr_fixture() {
    local out="$1" dur="${2:-3}"
    mkdir -p "$(dirname "$out")"
    ffmpeg -nostdin -loglevel error -y \
        -f lavfi -i "testsrc2=duration=${dur}:size=640x360:rate=15" \
        -f lavfi -i "sine=frequency=440:duration=${dur}" \
        -vf format=yuv420p10le \
        -c:v libx265 -pix_fmt yuv420p10le \
        -x265-params "colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc:range=limited:hdr10-opt=1:log-level=none" \
        -tag:v hvc1 -c:a aac -ac 2 -shortest "$out"
    FILE_COUNT=$((FILE_COUNT + 1))
    note "wrote HDR10 [$out]"
}

write_srt() {
    local out="$1"
    mkdir -p "$(dirname "$out")"
    printf '1\n00:00:01,000 --> 00:00:04,000\nBig Buck Bunny (dummy subtitle)\n\n2\n00:00:05,000 --> 00:00:08,000\nGenerated for subtitle testing.\n' >"$out"
    FILE_COUNT=$((FILE_COUNT + 1))
    note "wrote sidecar [$out]"
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
    [[ -n "$(clip_downloader)" ]] || die "--real needs curl or wget on PATH"
    command -v unzip >/dev/null 2>&1 || die "--real needs unzip on PATH (apt install unzip)"
    bbb=$(ensure_clip "$CLIP_CACHE_DIR" "big_buck_bunny_320x180.mp4" "$CLIP_URL_BBB" zip) \
        || die "could not fetch [Big Buck Bunny]"
    note "$CLIP_ATTRIBUTION_BBB (see deployment/dev/CREDITS.md)"
    place_clip "$bbb" "$MOVIES_DIR/Big Buck Bunny (2008)/Big Buck Bunny (2008).mp4"
    FILE_COUNT=$((FILE_COUNT + 1))
    ok "placed [Big Buck Bunny (2008)]"
    write_srt "$MOVIES_DIR/Big Buck Bunny (2008)/Big Buck Bunny (2008).en.srt"
    ed=$(ensure_clip "$CLIP_CACHE_DIR" "elephants_dream_clip.mp4" "$CLIP_URL_ED" trim:90) \
        || die "could not fetch [Elephants Dream]"
    note "$CLIP_ATTRIBUTION_ED (see deployment/dev/CREDITS.md)"
    place_clip "$ed" "$MOVIES_DIR/Elephants Dream (2006)/Elephants Dream (2006).mp4"
    FILE_COUNT=$((FILE_COUNT + 1))
    ok "placed [Elephants Dream (2006)]"
else
    add_movie "Big Buck Bunny"  2008 "1080p" mkv
    add_movie "Big Buck Bunny"  2008 "720p"  mp4
    write_srt "$MOVIES_DIR/Big Buck Bunny (2008)/Big Buck Bunny (2008) 1080p.en.srt"
    add_movie "Elephants Dream" 2006 ""      mp4
fi
add_movie "Sintel"          2010 "1080p" mkv
add_movie "Tears of Steel"  2012 "2160p" mkv
add_movie "Cosmos Laundromat" 2015 "1080p" mkv
add_movie "Spring"          2019 ""      mp4
gen_hdr_fixture "$MOVIES_DIR/HDR Sample (2024)/HDR Sample (2024).mkv"
ok "placed [HDR Sample (2024)] (HDR10 / PQ, for tone-map testing)"
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
