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

Six further movies ('Untitled Test Movie NN') are also intentionally unmatchable. They
exist to be relinked by hand in the UI, which is the quickest way to give the dev catalog
titles that share a cast: point several of them at real films with an actor in common and
the 'More with <actor>' rail on the title page has something to show. They also give the
lists enough rows to exercise infinite scroll.

Pass --cases to add the catalog shapes specific checks need: one title in two qualities
(Paper Skies, 2160p and 720p), a show with a Specials season and three regular seasons
(Skyline), a show with only a Specials season (Lantern), and a film filed one year before
its TMDB year (Casablanca, filed under 1942; TMDB has 1943). Casablanca is the one title
outside the Blender set; like every other fixture it is a synthetic clip.

Pass --playback to generate only the playback check set (run_playback_check.sh) into
PLAYBACK_DIR instead: fourteen 30 minute 640x360 movies, each 'movies/<Title> (<Year>)/<Title>
(<Year>).<ext>', MKV unless named:
  Harbor Lights (2001)  H.264 8-bit, AAC stereo
  Deep Field (2002)     H.264 8-bit, AC3 5.1(side)
  Cold Cases (2003)     H.264 8-bit, E-AC3 5.1(side)
  Sunrise (2004)        H.264 8-bit, DTS 5.1(side)
  Silent Signal (2005)  HEVC 10-bit, E-AC3 5.1(side), an embedded English SRT
  Neon Harbor (2006)    H.264 8-bit, AAC stereo (eng) and AC3 5.1(side) (fra), an external
                        '.en.srt'
  Paper Skies (2007)    H.264 8-bit, AAC stereo (eng) and AAC stereo (spa), an English PGS
                        and a German DVB subtitle with one line from 60 s to 68 s
  Old Reel (2008)       MPEG-4 Part 2 with B-frames (XVID tag), MP3 stereo, AVI
  Night Ferry (2009)    MS MPEG-4 v3, MP3 stereo, AVI
  Glass Garden (2010)   MPEG-4 Part 2 with B-frames (XVID tag), AC3 5.1(side)
  Low Tide (2011)       VP9, Opus stereo, WebM
  Iron Bridge (2012)    H.264 8-bit, FLAC stereo
  Blue Hour (2013)      H.264 8-bit, AAC stereo, MP4
  Copper Canyon (2014)  H.264 8-bit, AAC stereo, AVI
Each audio track is a tone of its own frequency, so a player can tell which one plays.
Every text subtitle has a 5 s line 'Line at <s>' every 10 s from 60 s. The image subtitles
come from crates/media/tests/fixtures/supsample.mkv (PLAYBACK_PGS). A '.complete' file
holding the set version is written last; the check regenerates the set when it is missing
or older.

After generating, trigger a library scan (admin UI, or POST /api/v1/libraries/{id}/scan)
so the server ingests the new files.

This writes only into the media dirs and is additive by default. Pass --reset to clear
them first. It does not touch the databases; the smoke test generates its own fixtures
separately.

Usage: $0 [--reset] [--real] [--cases] [--playback] [-h|--help]

Options:
  --reset        delete existing files under the movies and tv dirs before generating
  --real         download real CC BY 3.0 clips (Big Buck Bunny + Elephants Dream)
                 instead of synthetic stand-ins; needs curl or wget plus unzip; cached
                 under media/.cache and shared with the smoke test (no re-download)
  --cases        also generate the catalog shapes listed above: two qualities of one
                 title, a show with specials, a show with only specials, and a film
                 one year off its TMDB year
  --playback     generate only the playback check set into PLAYBACK_DIR (see above)
  -h, --help     show this help and exit

Environment overrides:
  MOVIES_DIR             movies library dir (default <deployment/dev>/media/movies)
  TV_DIR                 tv library dir     (default <deployment/dev>/media/tv)
  PLAYBACK_DIR           playback check set (default <deployment/dev>/media/playback)
  PLAYBACK_PGS           PGS source for the image subtitles
                         (default <repo>/crates/media/tests/fixtures/supsample.mkv)
  UNMATCHED_MOVIES       how many 'Untitled Test Movie NN' fixtures to generate (default 6);
                         raise it to give the lists enough rows for infinite scroll
  SHADOWMASK_CLIP_CACHE  download cache dir (default <deployment/dev>/media/.cache)"

RESET=0
REAL=0
CASES=0
PLAYBACK=0
for arg in "$@"; do
    case "$arg" in
        -h|--help) printf '%s\n' "$HELP"; exit 0 ;;
        --reset) RESET=1 ;;
        --real) REAL=1 ;;
        --cases) CASES=1 ;;
        --playback) PLAYBACK=1 ;;
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
PLAYBACK_DIR="${PLAYBACK_DIR:-$DEV_DIR/media/playback}"
PLAYBACK_PGS="${PLAYBACK_PGS:-$DEV_DIR/../../crates/media/tests/fixtures/supsample.mkv}"
UNMATCHED_MOVIES="${UNMATCHED_MOVIES:-6}"
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
ffmpeg -hide_banner -encoders 2>/dev/null | grep -w libx265 >/dev/null \
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

PLAYBACK_SET_VERSION=5
PLAYBACK_SECONDS=1800
PLAYBACK_H264=(-c:v libx264 -preset veryfast -crf 32 -g 48 -pix_fmt yuv420p)
PLAYBACK_HEVC10=(-c:v libx265 -preset ultrafast -pix_fmt yuv420p10le -x265-params "keyint=48:log-level=error")
PLAYBACK_XVID=(-c:v mpeg4 -vtag XVID -bf 2 -q:v 5 -g 48)
PLAYBACK_MSMPEG4=(-c:v msmpeg4 -q:v 5 -g 48)
PLAYBACK_VP9=(-c:v libvpx-vp9 -deadline realtime -cpu-used 8 -row-mt 1 -b:v 500k -g 48)
PLAYBACK_MP3=(-c:a libmp3lame -b:a 128k -ac 2)
PLAYBACK_SIDE='pan=5.1(side)|c0=c0|c1=c0|c2=c0|c3=c0|c4=c0|c5=c0'

playback_video() { printf 'testsrc2=size=640x360:rate=24000/1001:duration=%s' "$PLAYBACK_SECONDS"; }
playback_tone() { printf 'sine=frequency=%s:sample_rate=48000:duration=%s' "$1" "$PLAYBACK_SECONDS"; }
srt_clock() { printf '%02d:%02d:%02d,000' $(($1 / 3600)) $(($1 % 3600 / 60)) $(($1 % 60)); }

playback_srt() {
    local out="$1" s n=1
    mkdir -p "$(dirname "$out")"
    : >"$out"
    for ((s = 60; s + 5 <= PLAYBACK_SECONDS; s += 10)); do
        printf '%d\n%s --> %s\nLine at %d\n\n' "$n" "$(srt_clock "$s")" "$(srt_clock $((s + 5)))" "$s" >>"$out"
        n=$((n + 1))
    done
}

playback_file() {
    local name="$1" ext="$2"; shift 2
    local dir="$PLAYBACK_DIR/movies/$name"
    mkdir -p "$dir"
    ffmpeg -nostdin -loglevel error -y -f lavfi -i "$(playback_video)" "$@" "$dir/$name.$ext"
    FILE_COUNT=$((FILE_COUNT + 1))
    note "wrote [$dir/$name.$ext]"
}

playback_movie() {
    local name="$1"; shift
    playback_file "$name" mkv "$@"
}

gen_playback_set() {
    [[ -f "$PLAYBACK_PGS" ]] || die "the PGS source [$PLAYBACK_PGS] is missing; set PLAYBACK_PGS"
    local srt="$PLAYBACK_DIR/lines.srt"
    playback_srt "$srt"
    playback_movie "Harbor Lights (2001)" -f lavfi -i "$(playback_tone 440)" \
        -map 0:v -map 1:a "${PLAYBACK_H264[@]}" -c:a aac -b:a 128k -ac 2
    playback_movie "Deep Field (2002)" -f lavfi -i "$(playback_tone 330)" \
        -map 0:v -map 1:a "${PLAYBACK_H264[@]}" -af "$PLAYBACK_SIDE" -c:a ac3 -b:a 256k
    playback_movie "Cold Cases (2003)" -f lavfi -i "$(playback_tone 392)" \
        -map 0:v -map 1:a "${PLAYBACK_H264[@]}" -af "$PLAYBACK_SIDE" -c:a eac3 -b:a 256k
    playback_movie "Sunrise (2004)" -f lavfi -i "$(playback_tone 523)" \
        -map 0:v -map 1:a "${PLAYBACK_H264[@]}" -af "$PLAYBACK_SIDE" -c:a dca -strict -2 -b:a 768k
    playback_movie "Silent Signal (2005)" -f lavfi -i "$(playback_tone 294)" -i "$srt" \
        -map 0:v -map 1:a -map 2:s "${PLAYBACK_HEVC10[@]}" -af "$PLAYBACK_SIDE" -c:a eac3 -b:a 256k \
        -c:s srt -metadata:s:s:0 language=eng
    playback_movie "Neon Harbor (2006)" -f lavfi -i "$(playback_tone 587)" -f lavfi -i "$(playback_tone 660)" \
        -filter_complex "[2:a]${PLAYBACK_SIDE}[side]" -map 0:v -map 1:a -map "[side]" "${PLAYBACK_H264[@]}" \
        -c:a:0 aac -b:a:0 128k -ac:a:0 2 -c:a:1 ac3 -b:a:1 256k \
        -metadata:s:a:0 language=eng -metadata:s:a:1 language=fra
    playback_srt "$PLAYBACK_DIR/movies/Neon Harbor (2006)/Neon Harbor (2006).en.srt"
    FILE_COUNT=$((FILE_COUNT + 1))
    playback_movie "Paper Skies (2007)" -f lavfi -i "$(playback_tone 698)" -f lavfi -i "$(playback_tone 784)" \
        -fix_sub_duration -itsoffset 59 -i "$PLAYBACK_PGS" \
        -map 0:v -map 1:a -map 2:a -map 3:s:0 -map 3:s:0 "${PLAYBACK_H264[@]}" -c:a aac -b:a 128k -ac 2 \
        -metadata:s:a:0 language=eng -metadata:s:a:1 language=spa \
        -c:s:0 copy -c:s:1 dvbsub -metadata:s:s:0 language=eng -metadata:s:s:1 language=deu
    playback_file "Old Reel (2008)" avi -f lavfi -i "$(playback_tone 349)" \
        -map 0:v -map 1:a "${PLAYBACK_XVID[@]}" "${PLAYBACK_MP3[@]}"
    playback_file "Night Ferry (2009)" avi -f lavfi -i "$(playback_tone 466)" \
        -map 0:v -map 1:a "${PLAYBACK_MSMPEG4[@]}" "${PLAYBACK_MP3[@]}"
    playback_movie "Glass Garden (2010)" -f lavfi -i "$(playback_tone 554)" \
        -map 0:v -map 1:a "${PLAYBACK_XVID[@]}" -af "$PLAYBACK_SIDE" -c:a ac3 -b:a 256k
    playback_file "Low Tide (2011)" webm -f lavfi -i "$(playback_tone 622)" \
        -map 0:v -map 1:a "${PLAYBACK_VP9[@]}" -c:a libopus -b:a 96k -ac 2
    playback_movie "Iron Bridge (2012)" -f lavfi -i "$(playback_tone 740)" \
        -map 0:v -map 1:a "${PLAYBACK_H264[@]}" -c:a flac -ac 2
    playback_file "Blue Hour (2013)" mp4 -f lavfi -i "$(playback_tone 831)" \
        -map 0:v -map 1:a "${PLAYBACK_H264[@]}" -c:a aac -b:a 128k -ac 2 -movflags +faststart
    playback_file "Copper Canyon (2014)" avi -f lavfi -i "$(playback_tone 880)" \
        -map 0:v -map 1:a "${PLAYBACK_H264[@]}" -c:a aac -b:a 128k -ac 2
    rm -f "$srt"
}

if ((PLAYBACK)); then
    section "playback -> [$PLAYBACK_DIR]"
    rm -rf "$PLAYBACK_DIR/movies" "$PLAYBACK_DIR/.complete"
    mkdir -p "$PLAYBACK_DIR/movies" "$PLAYBACK_DIR/tv"
    gen_playback_set
    printf '%s\n' "$PLAYBACK_SET_VERSION" >"$PLAYBACK_DIR/.complete"
    ok "playback set done"
    exit 0
fi

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
for ((n = 1; n <= UNMATCHED_MOVIES; n++)); do
    add_movie "$(printf 'Untitled Test Movie %02d' "$n")" $((2000 + n)) "" mp4
done
ok "placed [$UNMATCHED_MOVIES] unmatchable movies for manual relinking"
ok "movies done"

section "tv -> [$TV_DIR]"
add_show() {
    local show="$1" season="$2"; shift 2
    for eps in "$@"; do
        local sdir; sdir=$(printf '%s/Season %02d' "$TV_DIR/$show" "$season")
        for ((ep = 1; ep <= eps; ep++)); do
            local tag; tag=$(printf 'S%02dE%02d' "$season" "$ep")
            local ext=mkv
            (( ep % 3 == 0 )) && ext=mp4
            gen_fixture "$sdir/$show $tag.$ext" "$(pick PATTERNS "$i")" "$(pick FREQS "$i")"
            i=$((i + 1))
        done
        season=$((season + 1))
    done
    ok "show [$show] done"
}
add_show "Pioneer One" 1 3
add_show "Untitled Test Show" 1 2 2

if ((CASES)); then
    section "cases -> [$MOVIES_DIR] [$TV_DIR]"
    add_movie "Paper Skies" 2007 "2160p" mkv
    add_movie "Paper Skies" 2007 "720p"  mkv
    ok "placed [Paper Skies (2007)] in 2160p and 720p, each file with its own quality"
    add_movie "Casablanca"  1942 ""      mp4
    ok "placed [Casablanca (1942)], one year before its TMDB year"
    add_show "Skyline" 0 2 3 2 1
    add_show "Lantern" 0 2
    ok "placed [Skyline] with specials and three seasons, [Lantern] with only specials"
fi

section "next steps"
note "for real artwork + metadata, set [SHADOWMASK_TMDB_API_KEY] before starting the stack"
note "trigger a scan so the server ingests the new files:"
note "  - admin UI: open a library and click [Trigger scan]"
note "  - or: POST /api/v1/libraries/{id}/scan"
ok "done"
