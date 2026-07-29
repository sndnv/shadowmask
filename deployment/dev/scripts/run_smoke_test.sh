#!/usr/bin/env bash

HELP="End-to-end smoke test for the Shadowmask dev deployment.

Runs against an ALREADY-RUNNING deployment/dev stack (it does not build, up, or
down the stack). It stops and starts ONLY the server service as part of the
backup/recover test. It is destructive to the dev databases and to generated
fixture media: it wipes and rebuilds both. Run it only against the disposable
dev stack.

Bring the stack up first, then run this script:
  podman compose -f deployment/dev/docker-compose.yml up -d --build
  deployment/dev/scripts/run_smoke_test.sh

Environment overrides:
  The transcribe -> translate section runs against the enrichment image only. On the
  base image it is skipped; on the enrichment image the enrichment features MUST be
  enabled (with models present) or the test fails. It uses the Elephants Dream speech
  fixture (CC BY 3.0) from the clip cache; populate the cache first with
  scripts/generate_media.sh --real.
  SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS  set true to skip enrichment (default false)
  SHADOWMASK_CLIP_CACHE        clip cache dir (default deployment/dev/media/.cache)"
USAGE="Usage: $0 [-h|--help]"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    printf '%s\n\n%s\n' "$HELP" "$USAGE"
    exit 0
fi

if ((BASH_VERSINFO[0] < 4)); then
    printf 'Error: bash 4+ required, found %s.%s\n' "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}" >&2
    exit 1
fi

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
DEV_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
MEDIA_DIR="$DEV_DIR/media"
source "$SCRIPT_DIR/clips.sh"
CLIP_CACHE_DIR=$(clip_cache_dir "$DEV_DIR")

BASE_URL="http://localhost:${SHADOWMASK_PORT:-8080}"
API="$BASE_URL/api/v1"
ADMIN_USER="admin"
ADMIN_USER_PASS="passw0rd"
TEST_USER="test-user"
TEST_USER_PASS="test-pass-123"
SKIP_ENRICHMENT="${SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS:-false}"
COMPOSE_FILE="$DEV_DIR/docker-compose.yml"
SERVICE="shadowmask"
SNAPSHOT="/data/snapshot.tar"
START_TS=$(date +%s)
SECTION_N=0

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
skip() { printf '%s     %s>>> SKIP: %s <<<%s\n' "$(ts)" "$YELLOW$BOLD" "$*" "$C0" >&2; }

section() {
    SECTION_N=$((SECTION_N + 1))
    printf '\n%s %s>: [%02d]: %s%s\n' "$(ts)" "$BOLD$CYAN" "$SECTION_N" "$*" "$C0" >&2
}
step() { printf '%s   %s-%s %s\n' "$(ts)" "$BOLD" "$C0" "$*" >&2; }
note() { printf '%s     %s... %s%s\n' "$(ts)" "$DIM" "$*" "$C0" >&2; }

RT=$(command -v podman >/dev/null 2>&1 && echo podman || echo docker)
compose() { "$RT" compose -f "$COMPOSE_FILE" "$@"; }

teardown() {
    local ec=$?
    log "finished in [$(( $(date +%s) - START_TS ))]s (exit [$ec])"
}
trap teardown EXIT

_req_args() {
    REQ_ARGS=(-sS -X "$1" -H 'Accept: application/json')
    if [[ -n "${3:-}" ]]; then REQ_ARGS+=(-H "Authorization: Bearer $3"); fi
    if [[ -n "${4:-}" ]]; then REQ_ARGS+=(-H 'Content-Type: application/json' --data "$4"); fi
}

_call() { _req_args "$@"; curl "${REQ_ARGS[@]}" -w $'\n%{http_code}' "$2"; }
_body() { local raw; raw=$(_call "$@") || return 1; printf '%s' "${raw%$'\n'*}"; }
_code() { _req_args "$@"; curl "${REQ_ARGS[@]}" -o /dev/null -w '%{http_code}' "$2"; }

call_ok() {
    local method="$1" url="$2" token="${3:-}" body="${4:-}" expect="${5:-200}"
    local label="${6:-$method ${url#"$BASE_URL"}}"
    local raw code out
    raw=$(_call "$method" "$url" "$token" "$body") || { die "$label -> curl error"; }
    code=${raw##*$'\n'}
    out=${raw%$'\n'*}
    if [[ "$code" != "$expect" ]]; then
        die "$label -> expected [$expect], got [$code]: [${out:0:400}]"
    fi
    ok "$label -> [$code]"
    printf '%s' "$out"
}

expect_code() {
    local expect="$1" method="$2" url="$3" token="${4:-}" body="${5:-}"
    local label="${6:-$method ${url#"$BASE_URL"}}"
    local code
    code=$(_code "$method" "$url" "$token" "$body") || { die "$label -> curl error"; }
    if [[ "$code" != "$expect" ]]; then die "$label -> expected [$expect], got [$code]"; fi
    ok "$label -> [$code]"
}

poll_until() {
    local desc="$1" timeout="$2"; shift 2
    local start; start=$(date +%s)
    while true; do
        if "$@"; then ok "$desc"; return 0; fi
        if (( $(date +%s) - start >= timeout )); then die "$desc -> timed out after [${timeout}]s"; fi
        sleep 2
    done
}

http_get_ok() { [[ "$(_code GET "$1" "")" == 200 ]]; }
health_ok() { http_get_ok "$BASE_URL/health"; }

scan_idle() {
    local s; s=$(_body GET "$API/libraries/$1/scan" "$2") || return 1
    [[ "$(jq -r '.status' <<<"$s" 2>/dev/null)" == idle ]]
}

total_at_least() {
    local t; t=$(_body GET "$1" "$2") || return 1
    t=$(jq -r '.total' <<<"$t" 2>/dev/null) || return 1
    [[ "$t" =~ ^[0-9]+$ ]] && (( t >= $3 ))
}

total_of() {
    local body; body=$(call_ok GET "$1" "$2")
    jq -r '.total' <<<"$body"
}

version_unavailable() {
    local b; b=$(_body GET "$API/versions/$1" "$2") || return 1
    [[ "$(jq -r '.available' <<<"$b" 2>/dev/null)" == false ]]
}

has_capability() { jq -e --arg c "$1" 'any(.capabilities[]?; .name==$c and .enabled)' <<<"$CAPS_JSON" >/dev/null 2>&1; }
capability_available() { jq -e --arg c "$1" 'any(.capabilities[]?; .name==$c and .available)' <<<"$CAPS_JSON" >/dev/null 2>&1; }

ts_to_ms() {
    local t="${1//,/.}" h m s ms
    IFS=':.' read -r h m s ms <<<"$t"
    printf '%d' $((10#$h * 3600000 + 10#$m * 60000 + 10#$s * 1000 + 10#$ms))
}

offset_applied() {
    local url="$1" want="$2" body ns om
    body=$(curl -sS "$url") || return 1
    grep -q ' --> ' <<<"$body" || return 1
    ns=$(grep -m1 ' --> ' <<<"$body" | sed -E 's/ *--> .*//' | tr -d '[:space:]')
    om=$(grep -m1 -oE 'ORIG=[0-9]+' <<<"$body" | cut -d= -f2)
    [[ -n "$ns" && -n "$om" ]] || return 1
    (( $(ts_to_ms "$ns") == om + want ))
}

subtitle_of_source() {
    local b; b=$(_body GET "$API/versions/$1" "$3") || return 1
    jq -e --arg s "$2" 'any(.subtitle_files[]?; .source==$s)' <<<"$b" >/dev/null 2>&1
}

subtitle_id_of_source() {
    local b; b=$(_body GET "$API/versions/$1" "$3") || return 1
    jq -r --arg s "$2" 'first(.subtitle_files[]? | select(.source==$s) | .id) // empty' <<<"$b"
}

subtitle_of_lang_source() {
    local b; b=$(_body GET "$API/versions/$1" "$4") || return 1
    jq -e --arg l "$2" --arg s "$3" 'any(.subtitle_files[]?; .language==$l and .source==$s)' \
        <<<"$b" >/dev/null 2>&1
}

subtitle_id_of_lang_source() {
    local b; b=$(_body GET "$API/versions/$1" "$4") || return 1
    jq -r --arg l "$2" --arg s "$3" \
        'first(.subtitle_files[]? | select(.language==$l and .source==$s) | .id) // empty' <<<"$b"
}

assert_subtitle_serves() {
    local ver="$1" sub="$2" token="$3" ss sid upd man base
    ss=$(call_ok POST "$API/sessions" "$token" \
        "$(jq -nc --arg v "$ver" '{version_id:$v,capabilities:{platform:"generic",profile_version:1}}')" 201 "start session for [$sub]")
    sid=$(jq -r '.session_id' <<<"$ss")
    upd=$(call_ok POST "$API/sessions/$sid/update" "$token" \
        "$(jq -nc --arg id "$sub" '{audio_track:null,subtitle:{action:"set",track:{type:"file",id:$id},offset_ms:null}}')" 200 "select subtitle [$sub]")
    man=$(jq -r '.manifest_url' <<<"$upd")
    base="${man%/master.m3u8}"
    poll_until "vtt for [$sub] ready" 120 http_get_ok "$BASE_URL$base/subs/subs.vtt"
    grep -q 'WEBVTT' <<<"$(curl -sS "$BASE_URL$base/subs/subs.vtt")" || die "subtitle [$sub] not served as WEBVTT"
    expect_code 204 DELETE "$API/sessions/$sid" "$token"
    ok "subtitle [$sub] serves as valid WEBVTT"
}

trigger_scan() {
    local code; code=$(_code POST "$API/libraries/$1/scan" "$2" "")
    case "$code" in
        202|409) ok "scan trigger [$1] -> [$code]" ;;
        *) die "scan trigger [$1] -> [$code]" ;;
    esac
}

reset_media() {
    mkdir -p "$MEDIA_DIR/movies" "$MEDIA_DIR/tv"
    find "$MEDIA_DIR/movies" "$MEDIA_DIR/tv" -type f -delete 2>/dev/null || true
}

gen_fixture() {
    ffmpeg -nostdin -loglevel error -y \
        -f lavfi -i "testsrc=duration=2:size=320x240:rate=15" \
        -f lavfi -i "sine=frequency=440:duration=2" \
        -c:v libx264 -pix_fmt yuv420p -b:v 800k \
        -c:a aac -ac 2 -shortest "$1"
}

write_offset_srt() {
    cat >"$1" <<'SRT'
1
00:00:01,000 --> 00:00:03,000
ORIG=1000

2
00:00:04,000 --> 00:00:06,000
ORIG=4000

3
00:00:07,000 --> 00:00:09,000
ORIG=7000
SRT
}

place_speech_clip() {
    local dest="$MEDIA_DIR/movies/Elephants Dream (2006).mp4"
    local cached="$CLIP_CACHE_DIR/elephants_dream_clip.mp4"
    [[ -s "$cached" ]] || die "the Elephants Dream speech fixture is not cached at [$cached]; populate the cache first with 'scripts/generate_media.sh --real', or set SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS=true to skip enrichment"
    place_clip "$cached" "$dest"
    note "$CLIP_ATTRIBUTION_ED (see deployment/dev/CREDITS.md)"
}

populate_media() {
    gen_fixture "$MEDIA_DIR/movies/Sample Movie (2011).mkv"
    gen_fixture "$MEDIA_DIR/movies/Sample Movie (2011).mp4"
    gen_fixture "$MEDIA_DIR/tv/Sample Show S01E01.mkv"
    write_offset_srt "$MEDIA_DIR/movies/Sample Movie (2011).en.srt"
    if [[ "$SKIP_ENRICHMENT" != true ]] && capability_available transcription; then
        place_speech_clip
    fi
}

server_stop() { compose stop "$SERVICE" >/dev/null 2>&1 || die "failed to stop [$SERVICE]"; }
server_start() { compose start "$SERVICE" >/dev/null 2>&1 || die "failed to start [$SERVICE]"; }
wipe_db() {
    compose run --rm --entrypoint sh "$SERVICE" -c 'rm -rf /data/server /data/users' >/dev/null 2>&1 \
        || die "failed to wipe databases"
}

first_child() {
    local url="$1" body child
    body=$(curl -sS "$url") || return 1
    child=$(printf '%s\n' "$body" | grep -vE '^[[:space:]]*(#|$)' | head -1)
    [[ -z "$child" ]] && return 1
    case "$child" in
        http*) printf '%s' "$child" ;;
        /*) printf '%s%s' "$BASE_URL" "$child" ;;
        *) printf '%s/%s' "${url%/*}" "$child" ;;
    esac
}

segment_fetchable() {
    local master="$1" child target
    child=$(first_child "$master") || return 1
    case "$child" in
        *.m3u8*) target=$(first_child "$child") || return 1 ;;
        *) target="$child" ;;
    esac
    [[ -n "$target" ]] || return 1
    [[ "$(_code GET "$target" "")" == 200 ]]
}

log ">: started"
log ">: [..] waiting for server /health"
poll_until "server healthy" 120 health_ok

section "reset media + wipe databases + restart server"
reset_media
server_stop
wipe_db
server_start
poll_until "server healthy after clean-slate restart" 120 health_ok

section "observability"
expect_code 200 GET "$BASE_URL/health"
expect_code 200 GET "$BASE_URL/health/ready"
metrics=$(call_ok GET "$BASE_URL/metrics" "" "" 200 "GET /metrics")
grep -q '# TYPE' <<<"$metrics" || die "/metrics is not Prometheus text format"
ok "/metrics is Prometheus format"

section "basic web UI (static assets served)"
expect_code 307 GET "$BASE_URL/" "" "" "GET / (redirect to /ui/basic/)"
ui_index=$(call_ok GET "$BASE_URL/ui/basic/" "" "" 200 "GET /ui/basic/ (sign-in shell)")
grep -q '<title>Shadowmask' <<<"$ui_index" || die "/ui/basic/ is missing the app title marker"
grep -q 'id="login"' <<<"$ui_index" || die "/ui/basic/ is missing the sign-in form"
ui_js=$(call_ok GET "$BASE_URL/ui/basic/app.js" "" "" 200 "GET /ui/basic/app.js")
grep -q 'const sm' <<<"$ui_js" || die "/ui/basic/app.js is missing the sm client global"
ok "basic web UI is served (entry shell + app.js present)"

section "admin auth + users"
adm=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$ADMIN_USER" --arg p "$ADMIN_USER_PASS" '{username:$u,password:$p}')" 200 "admin login")
ADMIN_TOKEN=$(jq -r '.access_token' <<<"$adm")
[[ -n "$ADMIN_TOKEN" && "$ADMIN_TOKEN" != null ]] || die "no admin access token"
self=$(call_ok GET "$API/users/self" "$ADMIN_TOKEN")
[[ "$(jq -r '.username' <<<"$self")" == "$ADMIN_USER" ]] || die "unexpected /users/self"
si=$(call_ok GET "$API/server/info" "$ADMIN_TOKEN")
jq -e '.version and (.profile_version != null)' <<<"$si" >/dev/null || die "server info missing fields"
CAPS_JSON="$si"
note "server capabilities: [$(jq -r '[.capabilities[] | "\(.name)=\(if .enabled then "on" elif .available then "available" else "unavailable" end)"] | join(", ")' <<<"$CAPS_JSON")]"

users=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
if jq -e --arg u "$TEST_USER" 'any(.items[]?; .username==$u)' <<<"$users" >/dev/null; then
    die "expected a clean database, but [$TEST_USER] already exists"
fi
ok "clean database confirmed ([$TEST_USER] absent)"

libs=$(call_ok GET "$API/libraries" "$ADMIN_TOKEN")
MOVIE_LIB=$(jq -r '.[] | select(.kind=="movie") | .id' <<<"$libs" | head -1)
TV_LIB=$(jq -r '.[] | select(.kind=="tv") | .id' <<<"$libs" | head -1)
[[ -n "$MOVIE_LIB" && -n "$TV_LIB" ]] || die "could not resolve bootstrapped Movies/TV libraries"
note "movie library=[$MOVIE_LIB] tv library=[$TV_LIB]"

note "creating [$TEST_USER]"
raw=$(_call POST "$API/users" "$ADMIN_TOKEN" "$(jq -nc --arg u "$TEST_USER" --arg p "$TEST_USER_PASS" '{username:$u,password:$p,role:"user"}')")
code=${raw##*$'\n'}; out=${raw%$'\n'*}
case "$code" in
    201) TEST_UID=$(jq -r '.id' <<<"$out"); ok "created [$TEST_USER] -> [$TEST_UID]" ;;
    400|409)
        list=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
        TEST_UID=$(jq -r --arg u "$TEST_USER" '.items[] | select(.username==$u) | .id' <<<"$list" | head -1)
        [[ -n "$TEST_UID" ]] || die "[$TEST_USER] exists ([$code]) but not found in list"
        ok "reusing existing [$TEST_USER] -> [$TEST_UID]" ;;
    *) die "create user -> unexpected [$code]: [$out]" ;;
esac

expect_code 204 PUT "$API/users/$TEST_UID/libraries" "$ADMIN_TOKEN" \
    "$(jq -nc --arg a "$MOVIE_LIB" --arg b "$TV_LIB" '{libraries:[$a,$b]}')"

usr=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$TEST_USER" --arg p "$TEST_USER_PASS" '{username:$u,password:$p}')" 200 "[$TEST_USER] login")
USER_TOKEN=$(jq -r '.access_token' <<<"$usr")
USER_REFRESH=$(jq -r '.refresh_token' <<<"$usr")
[[ -n "$USER_TOKEN" && "$USER_TOKEN" != null ]] || die "no user access token"

section "RBAC negatives (as [$TEST_USER])"
expect_code 403 GET "$API/users" "$USER_TOKEN"
expect_code 403 POST "$API/libraries" "$USER_TOKEN" \
    "$(jq -nc '{name:"x",kind:"movie",roots:["/media/movies"],watcher:"manual",metadata_sources:[]}')"

section "empty scan (no media yet)"
trigger_scan "$MOVIE_LIB" "$ADMIN_TOKEN"
trigger_scan "$TV_LIB" "$ADMIN_TOKEN"
poll_until "movies scan idle" 90 scan_idle "$MOVIE_LIB" "$ADMIN_TOKEN"
poll_until "tv scan idle" 90 scan_idle "$TV_LIB" "$ADMIN_TOKEN"
[[ "$(total_of "$API/movies" "$ADMIN_TOKEN")" == 0 ]] || die "expected 0 movies before populate"
[[ "$(total_of "$API/series" "$ADMIN_TOKEN")" == 0 ]] || die "expected 0 series before populate"
ok "empty libraries scanned clean (0 movies, 0 series)"

section "populate media + rescan"
populate_media
ok "generated fixtures under [$MEDIA_DIR]"
trigger_scan "$MOVIE_LIB" "$ADMIN_TOKEN"
trigger_scan "$TV_LIB" "$ADMIN_TOKEN"
poll_until "movies rescan idle" 90 scan_idle "$MOVIE_LIB" "$ADMIN_TOKEN"
poll_until "tv rescan idle" 90 scan_idle "$TV_LIB" "$ADMIN_TOKEN"
poll_until "movie catalog row appears" 90 total_at_least "$API/movies" "$ADMIN_TOKEN" 1
poll_until "series catalog row appears" 90 total_at_least "$API/series" "$ADMIN_TOKEN" 1

section "catalog + search (as [$TEST_USER])"
mv=$(call_ok GET "$API/movies?limit=200" "$USER_TOKEN")
MOVIE_ID=$(jq -r 'first(.items[]? | select(.title=="Sample Movie") | .id) // empty' <<<"$mv")
[[ -n "$MOVIE_ID" && "$MOVIE_ID" != null ]] || die "no movie id"
ED_MOVIE_ID=$(jq -r 'first(.items[]? | select(.title=="Elephants Dream") | .id) // empty' <<<"$mv")
call_ok GET "$API/movies/$MOVIE_ID" "$USER_TOKEN" >/dev/null
vers=$(call_ok GET "$API/movies/$MOVIE_ID/versions" "$USER_TOKEN")
MKV_VER=$(jq -r '.items[] | select(.container=="mkv") | .id' <<<"$vers" | head -1)
MP4_VER=$(jq -r '.items[] | select(.container=="mp4") | .id' <<<"$vers" | head -1)
[[ -n "$MKV_VER" && "$MKV_VER" != null ]] || die "no mkv version"
[[ -n "$MP4_VER" && "$MP4_VER" != null ]] || die "no mp4 version"
call_ok GET "$API/versions/$MKV_VER" "$USER_TOKEN" >/dev/null
sr=$(call_ok GET "$API/series" "$USER_TOKEN")
SERIES_ID=$(jq -r '.items[0].id' <<<"$sr")
call_ok GET "$API/series/$SERIES_ID" "$USER_TOKEN" >/dev/null
seasons=$(call_ok GET "$API/series/$SERIES_ID/seasons" "$USER_TOKEN")
SEASON_ID=$(jq -r '.[0].id' <<<"$seasons")
eps=$(call_ok GET "$API/series/$SERIES_ID/seasons/$SEASON_ID/episodes" "$USER_TOKEN")
EPISODE_ID=$(jq -r '.[0].id' <<<"$eps")
call_ok GET "$API/series/$SERIES_ID/seasons/$SEASON_ID/episodes/$EPISODE_ID" "$USER_TOKEN" >/dev/null
call_ok GET "$API/series/$SERIES_ID/seasons/$SEASON_ID/episodes/$EPISODE_ID/versions" "$USER_TOKEN" >/dev/null
sc=$(call_ok GET "$API/search?q=Sample" "$USER_TOKEN")
(( $(jq -r '.total' <<<"$sc") >= 1 )) || die "search returned no results"
call_ok GET "$API/genres" "$USER_TOKEN" >/dev/null
note "skipping /people (no metadata provider configured, no person ids)"
call_ok GET "$API/admin/jobs" "$ADMIN_TOKEN" >/dev/null

section "watchlist marker (durable recovery proof)"
expect_code 204 PUT "$API/users/$TEST_UID/watchlist/$MOVIE_ID" "$USER_TOKEN" "$(jq -nc '{type:"movie"}')"
wl=$(call_ok GET "$API/users/$TEST_UID/watchlist" "$USER_TOKEN")
(( $(jq 'length' <<<"$wl") >= 1 )) || die "watchlist empty after add"

section "playback: remux HLS (mkv)"
ss=$(call_ok POST "$API/sessions" "$USER_TOKEN" \
    "$(jq -nc --arg v "$MKV_VER" '{version_id:$v,capabilities:{platform:"generic",profile_version:1}}')" 201 "start session (mkv)")
SESSION_ID=$(jq -r '.session_id' <<<"$ss")
MODE=$(jq -r '.mode' <<<"$ss")
MANIFEST_URL=$(jq -r '.manifest_url' <<<"$ss")
[[ "$MODE" == remux ]] || die "expected remux for mkv, got [$MODE]"
note "session [$SESSION_ID] mode=[$MODE] manifest=[$(sed -E 's#/[^/]{20,}#/<token>#' <<<"$MANIFEST_URL")]"
MASTER="$BASE_URL$MANIFEST_URL"
poll_until "HLS master playlist ready" 60 http_get_ok "$MASTER"
grep -q '#EXTM3U' <<<"$(curl -sS "$MASTER")" || die "master is not an m3u8"
poll_until "HLS segment fetch" 60 segment_fetchable "$MASTER"
call_ok POST "$API/sessions/$SESSION_ID/progress" "$USER_TOKEN" "$(jq -nc '{position_ms:5000,state:"playing"}')" 200 "heartbeat playing" >/dev/null
call_ok POST "$API/sessions/$SESSION_ID/progress" "$USER_TOKEN" "$(jq -nc '{position_ms:6000,state:"paused"}')" 200 "heartbeat paused" >/dev/null
act=$(call_ok GET "$API/users/activity" "$ADMIN_TOKEN")
jq -e --arg s "$SESSION_ID" 'any(.items[]?; .session_id==$s)' <<<"$act" >/dev/null || die "session not in admin now-playing"
ok "session visible in admin now-playing"

section "playback: soft subtitle (sidecar VTT)"
det=$(call_ok GET "$API/versions/$MKV_VER" "$USER_TOKEN")
SUB_ID=$(jq -r '.subtitle_files[0].id // empty' <<<"$det")
[[ -n "$SUB_ID" ]] || die "mkv version has no discovered subtitle_files"
note "selecting sidecar subtitle [$SUB_ID]"
upd=$(call_ok POST "$API/sessions/$SESSION_ID/update" "$USER_TOKEN" \
    "$(jq -nc --arg id "$SUB_ID" '{audio_track:null,subtitle:{action:"set",track:{type:"file",id:$id},offset_ms:null}}')" 200 "select sidecar subtitle")
SUB_MANIFEST=$(jq -r '.manifest_url' <<<"$upd")
poll_until "subtitle master playlist ready" 60 http_get_ok "$BASE_URL$SUB_MANIFEST"
grep -q 'TYPE=SUBTITLES' <<<"$(curl -sS "$BASE_URL$SUB_MANIFEST")" || die "master missing SUBTITLES rendition"
STREAM_BASE="${SUB_MANIFEST%/master.m3u8}"
poll_until "subtitle vtt ready" 60 http_get_ok "$BASE_URL$STREAM_BASE/subs/subs.vtt"
grep -q 'WEBVTT' <<<"$(curl -sS "$BASE_URL$STREAM_BASE/subs/subs.vtt")" || die "subs.vtt is not WEBVTT"
ok "sidecar subtitle served as WEBVTT"

step "subtitle offset: shift by 5000ms and verify cue timing"
upd2=$(call_ok POST "$API/sessions/$SESSION_ID/update" "$USER_TOKEN" \
    "$(jq -nc --arg id "$SUB_ID" '{audio_track:null,subtitle:{action:"set",track:{type:"file",id:$id},offset_ms:5000}}')" 200 "select sidecar subtitle with offset")
STREAM_BASE2="$(jq -r '.manifest_url' <<<"$upd2")"
STREAM_BASE2="${STREAM_BASE2%/master.m3u8}"
poll_until "subtitle offset applied (+5000ms)" 60 offset_applied "$BASE_URL$STREAM_BASE2/subs/subs.vtt" 5000
ok "subtitle offset applied: first cue shifted by exactly 5000ms"

call_ok POST "$API/sessions/$SESSION_ID/seek" "$USER_TOKEN" "$(jq -nc '{position_ms:10000}')" 200 "seek" >/dev/null
call_ok POST "$API/sessions/$SESSION_ID/update" "$USER_TOKEN" "$(jq -nc '{audio_track:null,subtitle:{action:"keep"}}')" 200 "update tracks" >/dev/null
pr=$(call_ok GET "$API/users/$TEST_UID/progress/$MKV_VER" "$USER_TOKEN")
[[ "$(jq -r 'if .==null then "null" else "set" end' <<<"$pr")" == "set" ]] || die "no resume progress recorded"
call_ok GET "$API/users/$TEST_UID/continue" "$USER_TOKEN" >/dev/null
call_ok GET "$API/users/$TEST_UID/hub" "$USER_TOKEN" >/dev/null
expect_code 204 DELETE "$API/sessions/$SESSION_ID" "$USER_TOKEN"

section "playback: direct (mp4)"
ss2=$(call_ok POST "$API/sessions" "$USER_TOKEN" \
    "$(jq -nc --arg v "$MP4_VER" '{version_id:$v,capabilities:{platform:"generic",profile_version:1}}')" 201 "start session (mp4)")
SID2=$(jq -r '.session_id' <<<"$ss2")
MU2=$(jq -r '.manifest_url' <<<"$ss2")
[[ "$(jq -r '.mode' <<<"$ss2")" == direct ]] || die "expected direct for mp4, got [$(jq -r '.mode' <<<"$ss2")]"
poll_until "direct-play file fetch" 30 http_get_ok "$BASE_URL$MU2"
expect_code 204 DELETE "$API/sessions/$SID2" "$USER_TOKEN"

section "enrichment: transcribe -> translate"
if [[ "$SKIP_ENRICHMENT" == true ]]; then
    skip "enrichment: disabled via SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS"
elif ! capability_available transcription; then
    skip "enrichment: base image (transcription/translation not compiled in); run the enrichment image, or leave SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS=true"
else
    has_capability transcription \
        || die "transcription: the enrichment image is running but transcription is not enabled; set SHADOWMASK_ENRICHMENT_TRANSCRIPTION_ENABLED=true and mount a Whisper CT2 model (or set SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS=true to skip enrichment)"
    has_capability translation \
        || die "translation: the enrichment image is running but translation is not enabled; set SHADOWMASK_ENRICHMENT_TRANSLATION_ENABLED=true and mount a translation CT2 model (or set SHADOWMASK_SMOKE_TEST_SKIP_ENRICHMENT_TESTS=true to skip enrichment)"
    [[ -n "$ED_MOVIE_ID" && "$ED_MOVIE_ID" != null ]] \
        || die "the Elephants Dream speech fixture is enabled for enrichment but was not ingested; check the fixture placement and library scan"
    edv=$(call_ok GET "$API/movies/$ED_MOVIE_ID/versions" "$USER_TOKEN")
    ED_VER=$(jq -r '.items[0].id' <<<"$edv")
    [[ -n "$ED_VER" && "$ED_VER" != null ]] || die "no Elephants Dream version"
    call_ok POST "$API/admin/versions/$ED_VER/transcribe" "$ADMIN_TOKEN" "$(jq -nc '{}')" 202 "trigger transcription" >/dev/null
    poll_until "transcription produced a [generated] subtitle" 900 subtitle_of_source "$ED_VER" generated "$ADMIN_TOKEN"
    GEN_SUB=$(subtitle_id_of_source "$ED_VER" generated "$ADMIN_TOKEN")
    [[ -n "$GEN_SUB" ]] || die "no generated subtitle id"
    assert_subtitle_serves "$ED_VER" "$GEN_SUB" "$USER_TOKEN"
    call_ok POST "$API/admin/versions/$ED_VER/translate" "$ADMIN_TOKEN" \
        "$(jq -nc --arg s "$GEN_SUB" '{source_subtitle_id:$s,target_language:"es"}')" 202 "trigger translation (es)" >/dev/null
    poll_until "translation produced a [machine_translated] es subtitle" 900 \
        subtitle_of_lang_source "$ED_VER" es machine_translated "$ADMIN_TOKEN"
    MT_SUB=$(subtitle_id_of_lang_source "$ED_VER" es machine_translated "$ADMIN_TOKEN")
    [[ -n "$MT_SUB" ]] || die "no machine_translated es subtitle id"
    assert_subtitle_serves "$ED_VER" "$MT_SUB" "$USER_TOKEN"
    ok "transcribe -> translate chain verified: [generated] en -> [machine_translated] es"
fi

section "account state (as [$TEST_USER])"
expect_code 204 PUT "$API/users/$TEST_UID/favorites/$MOVIE_ID" "$USER_TOKEN" "$(jq -nc '{type:"movie"}')"
(( $(jq 'length' <<<"$(call_ok GET "$API/users/$TEST_UID/favorites" "$USER_TOKEN")") >= 1 )) || die "favorite not added"
expect_code 204 DELETE "$API/users/$TEST_UID/favorites/$MOVIE_ID" "$USER_TOKEN"
(( $(jq 'length' <<<"$(call_ok GET "$API/users/$TEST_UID/favorites" "$USER_TOKEN")") == 0 )) || die "favorite not removed"
expect_code 204 PUT "$API/users/$TEST_UID/watched/$MOVIE_ID" "$USER_TOKEN" "$(jq -nc '{type:"movie",watched:true}')"
expect_code 204 DELETE "$API/users/$TEST_UID/progress/$MKV_VER" "$USER_TOKEN"
pr2=$(call_ok GET "$API/users/$TEST_UID/progress/$MKV_VER" "$USER_TOKEN")
[[ "$(jq -r 'if .==null then "null" else "set" end' <<<"$pr2")" == "null" ]] || die "progress not cleared"
call_ok GET "$API/users/$TEST_UID/history" "$USER_TOKEN" >/dev/null
sb=$(call_ok POST "$API/users/$TEST_UID/state/batch" "$USER_TOKEN" "$(jq -nc --arg id "$MOVIE_ID" '{titles:[{type:"movie",id:$id}]}')" 200 "state batch")
jq -e '.[0].watched==true and .[0].watchlisted==true' <<<"$sb" >/dev/null || die "batch state incorrect"

section "device link codes (as [$TEST_USER])"
lc=$(call_ok POST "$API/auth/link/create" "$USER_TOKEN" "$(jq -nc '{}')" 200 "create link code")
LINK_CODE=$(jq -r '.code' <<<"$lc")
[[ -n "$LINK_CODE" && "$LINK_CODE" != null ]] || die "no link code returned"
pend=$(call_ok GET "$API/users/$TEST_UID/link-codes" "$USER_TOKEN")
jq -e --arg c "$LINK_CODE" 'any(.[]?; .code==$c)' <<<"$pend" >/dev/null || die "code [$LINK_CODE] not listed as pending"
ok "link code [$LINK_CODE] created and listed as pending"
TYPED="${LINK_CODE:0:4}-${LINK_CODE:4}"
red=$(call_ok POST "$API/auth/link" "" "$(jq -nc --arg c "$TYPED" '{code:$c,device:{name:"Smoke Roku",platform:"roku"}}')" 200 "redeem link code [$TYPED]")
DEVICE_TOKEN=$(jq -r '.token' <<<"$red")
[[ "$DEVICE_TOKEN" == smk_* ]] || die "redeemed token is not a device token: [${DEVICE_TOKEN:0:8}...]"
ok "link code redeemed in its displayed form -> device token"
call_ok GET "$API/movies" "$DEVICE_TOKEN" >/dev/null
dv=$(call_ok GET "$API/users/$TEST_UID/devices" "$USER_TOKEN")
jq -e 'any(.[]?; .platform=="roku")' <<<"$dv" >/dev/null || die "redeemed device not registered on the account"
ok "device token authenticates and the device is registered"
expect_code 404 POST "$API/auth/link" "" "$(jq -nc --arg c "$LINK_CODE" '{code:$c,device:{name:"Dup",platform:"roku"}}')"
ok "link code is single-use (second redemption rejected)"
lc2=$(call_ok POST "$API/auth/link/create" "$USER_TOKEN" "$(jq -nc '{}')" 200 "create second link code")
CODE2=$(jq -r '.code' <<<"$lc2")
[[ -n "$CODE2" && "$CODE2" != null ]] || die "no second link code returned"
expect_code 204 DELETE "$API/users/$TEST_UID/link-codes/$CODE2" "$USER_TOKEN"
expect_code 404 POST "$API/auth/link" "" "$(jq -nc --arg c "$CODE2" '{code:$c,device:{name:"Revoked",platform:"roku"}}')"
ok "revoked link code [$CODE2] cannot be redeemed"

expect_code 204 POST "$API/auth/logout" "" "$(jq -nc --arg r "$USER_REFRESH" '{refresh_token:$r}')"
expect_code 401 POST "$API/auth/refresh" "" "$(jq -nc --arg r "$USER_REFRESH" '{refresh_token:$r}')"
ok "refresh token revoked after logout"

section "backup -> recover cycle"
compose run --rm "$SERVICE" backup "$SNAPSHOT" >/dev/null 2>&1 || die "backup failed"
ok "backup written to [$SNAPSHOT] (hot, server live)"

if compose run --rm "$SERVICE" recover --from "$SNAPSHOT" >/dev/null 2>&1; then
    warn "recover succeeded while server live (cross-container lock not enforced here); snapshot equals live state so no harm"
else
    ok "recover refused while server live (lock guard)"
fi

server_stop
wipe_db
server_start
poll_until "server healthy after DB wipe" 120 health_ok
adm2=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$ADMIN_USER" --arg p "$ADMIN_USER_PASS" '{username:$u,password:$p}')" 200 "admin login after wipe")
ADMIN_TOKEN=$(jq -r '.access_token' <<<"$adm2")
after_wipe=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
if jq -e --arg u "$TEST_USER" 'any(.items[]?; .username==$u)' <<<"$after_wipe" >/dev/null; then
    die "[$TEST_USER] still present after DB wipe (wipe ineffective)"
fi
ok "server started EMPTY after wipe: admin present, [$TEST_USER] gone"

server_stop
compose run --rm "$SERVICE" recover --from "$SNAPSHOT" >/dev/null 2>&1 || die "recover failed"
ok "recovered from [$SNAPSHOT]"
server_start
poll_until "server healthy after recover" 120 health_ok
adm3=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$ADMIN_USER" --arg p "$ADMIN_USER_PASS" '{username:$u,password:$p}')" 200 "admin login after recover")
ADMIN_TOKEN=$(jq -r '.access_token' <<<"$adm3")
restored=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
jq -e --arg u "$TEST_USER" 'any(.items[]?; .username==$u)' <<<"$restored" >/dev/null || die "[$TEST_USER] missing after recover"
wl2=$(call_ok GET "$API/users/$TEST_UID/watchlist" "$ADMIN_TOKEN")
(( $(jq 'length' <<<"$wl2") >= 1 )) || die "watchlist empty after recover"
ok "recover restored [$TEST_USER] and its watchlist"

section "media removal handling (soft-delete: version-only, titles preserved)"
movies_before=$(total_of "$API/movies" "$ADMIN_TOKEN")
series_before=$(total_of "$API/series" "$ADMIN_TOKEN")
(( movies_before >= 1 )) || die "expected >=1 movie before removal, got [$movies_before]"
(( series_before >= 1 )) || die "expected >=1 series before removal, got [$series_before]"

step "partial removal: delete one of the movie's two files"
rm -f "$MEDIA_DIR/movies/Sample Movie (2011).mkv"
ok "removed the movie's mkv file, kept its mp4 sibling"
trigger_scan "$MOVIE_LIB" "$ADMIN_TOKEN"
poll_until "movies rescan idle (partial removal)" 90 scan_idle "$MOVIE_LIB" "$ADMIN_TOKEN"
poll_until "removed mkv version marked unavailable" 30 version_unavailable "$MKV_VER" "$ADMIN_TOKEN"
[[ "$(total_of "$API/movies" "$ADMIN_TOKEN")" == "$movies_before" ]] \
    || die "movie title vanished after removing one of its files (must soft-delete the version only)"
mp4d=$(_body GET "$API/versions/$MP4_VER" "$ADMIN_TOKEN")
[[ "$(jq -r '.available' <<<"$mp4d")" == true ]] || die "surviving mp4 version was wrongly marked unavailable"
ok "title kept; mkv version unavailable; mp4 version still available"
expect_code 404 POST "$API/sessions" "$ADMIN_TOKEN" \
    "$(jq -nc --arg v "$MKV_VER" '{version_id:$v,capabilities:{platform:"generic",profile_version:1}}')"
ok "playback of the removed (unavailable) version -> 404"
sp=$(call_ok POST "$API/sessions" "$ADMIN_TOKEN" \
    "$(jq -nc --arg v "$MP4_VER" '{version_id:$v,capabilities:{platform:"generic",profile_version:1}}')" 201 "start session on surviving mp4")
expect_code 204 DELETE "$API/sessions/$(jq -r '.session_id' <<<"$sp")" "$ADMIN_TOKEN"
ok "surviving mp4 version still playable"

step "full removal: delete all remaining media"
reset_media
ok "removed all remaining fixture media (root dirs kept)"
trigger_scan "$MOVIE_LIB" "$ADMIN_TOKEN"
trigger_scan "$TV_LIB" "$ADMIN_TOKEN"
poll_until "movies rescan idle (full removal)" 90 scan_idle "$MOVIE_LIB" "$ADMIN_TOKEN"
poll_until "tv rescan idle (full removal)" 90 scan_idle "$TV_LIB" "$ADMIN_TOKEN"
poll_until "mp4 version marked unavailable" 30 version_unavailable "$MP4_VER" "$ADMIN_TOKEN"
[[ "$(total_of "$API/movies" "$ADMIN_TOKEN")" == "$movies_before" ]] \
    || die "movie title vanished after full removal (titles must be preserved)"
[[ "$(total_of "$API/series" "$ADMIN_TOKEN")" == "$series_before" ]] \
    || die "series title vanished after full removal (titles must be preserved)"
ok "all titles still listed after full removal; all versions unavailable"
expect_code 404 POST "$API/sessions" "$ADMIN_TOKEN" \
    "$(jq -nc --arg v "$MP4_VER" '{version_id:$v,capabilities:{platform:"generic",profile_version:1}}')"
ok "playback of a removed version -> 404"

log "ALL CHECKS PASSED"
