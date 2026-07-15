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
  BASE_URL       server base url (default http://localhost:\${SHADOWMASK_PORT:-8080})
  ADMIN_USER     bootstrap admin username (default admin)
  ADMIN_PASS     bootstrap admin password (default passw0rd)
  COMPOSE_FILE   compose file (default deployment/dev/docker-compose.yml)
  SERVICE        compose service name (default shadowmask)"
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

BASE_URL="${BASE_URL:-http://localhost:${SHADOWMASK_PORT:-8080}}"
API="$BASE_URL/api/v1"
ADMIN_USER="${ADMIN_USER:-admin}"
ADMIN_PASS="${ADMIN_PASS:-passw0rd}"
SMOKE_USER="${SMOKE_USER:-smoke_user}"
SMOKE_PASS="${SMOKE_PASS:-smoke-pass-123}"
COMPOSE_FILE="${COMPOSE_FILE:-$DEV_DIR/docker-compose.yml}"
SERVICE="${SERVICE:-shadowmask}"
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

populate_media() {
    gen_fixture "$MEDIA_DIR/movies/Sample Movie (2011).mkv"
    gen_fixture "$MEDIA_DIR/movies/Sample Movie (2011).mp4"
    gen_fixture "$MEDIA_DIR/tv/Sample Show S01E01.mkv"
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

section "admin auth + users"
adm=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$ADMIN_USER" --arg p "$ADMIN_PASS" '{username:$u,password:$p}')" 200 "admin login")
ADMIN_TOKEN=$(jq -r '.access_token' <<<"$adm")
[[ -n "$ADMIN_TOKEN" && "$ADMIN_TOKEN" != null ]] || die "no admin access token"
self=$(call_ok GET "$API/users/self" "$ADMIN_TOKEN")
[[ "$(jq -r '.username' <<<"$self")" == "$ADMIN_USER" ]] || die "unexpected /users/self"
si=$(call_ok GET "$API/server/info" "$ADMIN_TOKEN")
jq -e '.version and (.profile_version != null)' <<<"$si" >/dev/null || die "server info missing fields"

users=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
if jq -e --arg u "$SMOKE_USER" 'any(.items[]?; .username==$u)' <<<"$users" >/dev/null; then
    die "expected a clean database, but [$SMOKE_USER] already exists"
fi
ok "clean database confirmed ([$SMOKE_USER] absent)"

libs=$(call_ok GET "$API/libraries" "$ADMIN_TOKEN")
MOVIE_LIB=$(jq -r '.[] | select(.kind=="movie") | .id' <<<"$libs" | head -1)
TV_LIB=$(jq -r '.[] | select(.kind=="tv") | .id' <<<"$libs" | head -1)
[[ -n "$MOVIE_LIB" && -n "$TV_LIB" ]] || die "could not resolve bootstrapped Movies/TV libraries"
note "movie library=[$MOVIE_LIB] tv library=[$TV_LIB]"

note "creating [$SMOKE_USER]"
raw=$(_call POST "$API/users" "$ADMIN_TOKEN" "$(jq -nc --arg u "$SMOKE_USER" --arg p "$SMOKE_PASS" '{username:$u,password:$p,role:"user"}')")
code=${raw##*$'\n'}; out=${raw%$'\n'*}
case "$code" in
    201) SMOKE_UID=$(jq -r '.id' <<<"$out"); ok "created [$SMOKE_USER] -> [$SMOKE_UID]" ;;
    400|409)
        list=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
        SMOKE_UID=$(jq -r --arg u "$SMOKE_USER" '.items[] | select(.username==$u) | .id' <<<"$list" | head -1)
        [[ -n "$SMOKE_UID" ]] || die "[$SMOKE_USER] exists ([$code]) but not found in list"
        ok "reusing existing [$SMOKE_USER] -> [$SMOKE_UID]" ;;
    *) die "create user -> unexpected [$code]: [$out]" ;;
esac

expect_code 204 PUT "$API/users/$SMOKE_UID/libraries" "$ADMIN_TOKEN" \
    "$(jq -nc --arg a "$MOVIE_LIB" --arg b "$TV_LIB" '{libraries:[$a,$b]}')"

usr=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$SMOKE_USER" --arg p "$SMOKE_PASS" '{username:$u,password:$p}')" 200 "[$SMOKE_USER] login")
USER_TOKEN=$(jq -r '.access_token' <<<"$usr")
USER_REFRESH=$(jq -r '.refresh_token' <<<"$usr")
[[ -n "$USER_TOKEN" && "$USER_TOKEN" != null ]] || die "no user access token"

section "RBAC negatives (as [$SMOKE_USER])"
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

section "catalog + search (as [$SMOKE_USER])"
mv=$(call_ok GET "$API/movies" "$USER_TOKEN")
MOVIE_ID=$(jq -r '.items[0].id' <<<"$mv")
[[ -n "$MOVIE_ID" && "$MOVIE_ID" != null ]] || die "no movie id"
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
expect_code 204 PUT "$API/users/$SMOKE_UID/watchlist/$MOVIE_ID" "$USER_TOKEN" "$(jq -nc '{type:"movie"}')"
wl=$(call_ok GET "$API/users/$SMOKE_UID/watchlist" "$USER_TOKEN")
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
call_ok POST "$API/sessions/$SESSION_ID/seek" "$USER_TOKEN" "$(jq -nc '{position_ms:10000}')" 200 "seek" >/dev/null
call_ok POST "$API/sessions/$SESSION_ID/update" "$USER_TOKEN" "$(jq -nc '{audio_track:null,subtitle:{action:"keep"}}')" 200 "update tracks" >/dev/null
pr=$(call_ok GET "$API/users/$SMOKE_UID/progress/$MKV_VER" "$USER_TOKEN")
[[ "$(jq -r 'if .==null then "null" else "set" end' <<<"$pr")" == "set" ]] || die "no resume progress recorded"
call_ok GET "$API/users/$SMOKE_UID/continue" "$USER_TOKEN" >/dev/null
call_ok GET "$API/users/$SMOKE_UID/hub" "$USER_TOKEN" >/dev/null
expect_code 204 DELETE "$API/sessions/$SESSION_ID" "$USER_TOKEN"

section "playback: direct (mp4)"
ss2=$(call_ok POST "$API/sessions" "$USER_TOKEN" \
    "$(jq -nc --arg v "$MP4_VER" '{version_id:$v,capabilities:{platform:"generic",profile_version:1}}')" 201 "start session (mp4)")
SID2=$(jq -r '.session_id' <<<"$ss2")
MU2=$(jq -r '.manifest_url' <<<"$ss2")
[[ "$(jq -r '.mode' <<<"$ss2")" == direct ]] || die "expected direct for mp4, got [$(jq -r '.mode' <<<"$ss2")]"
poll_until "direct-play file fetch" 30 http_get_ok "$BASE_URL$MU2"
expect_code 204 DELETE "$API/sessions/$SID2" "$USER_TOKEN"

section "account state (as [$SMOKE_USER])"
expect_code 204 PUT "$API/users/$SMOKE_UID/favorites/$MOVIE_ID" "$USER_TOKEN" "$(jq -nc '{type:"movie"}')"
(( $(jq 'length' <<<"$(call_ok GET "$API/users/$SMOKE_UID/favorites" "$USER_TOKEN")") >= 1 )) || die "favorite not added"
expect_code 204 DELETE "$API/users/$SMOKE_UID/favorites/$MOVIE_ID" "$USER_TOKEN"
(( $(jq 'length' <<<"$(call_ok GET "$API/users/$SMOKE_UID/favorites" "$USER_TOKEN")") == 0 )) || die "favorite not removed"
expect_code 204 PUT "$API/users/$SMOKE_UID/watched/$MOVIE_ID" "$USER_TOKEN" "$(jq -nc '{type:"movie",watched:true}')"
expect_code 204 DELETE "$API/users/$SMOKE_UID/progress/$MKV_VER" "$USER_TOKEN"
pr2=$(call_ok GET "$API/users/$SMOKE_UID/progress/$MKV_VER" "$USER_TOKEN")
[[ "$(jq -r 'if .==null then "null" else "set" end' <<<"$pr2")" == "null" ]] || die "progress not cleared"
call_ok GET "$API/users/$SMOKE_UID/history" "$USER_TOKEN" >/dev/null
sb=$(call_ok POST "$API/users/$SMOKE_UID/state/batch" "$USER_TOKEN" "$(jq -nc --arg id "$MOVIE_ID" '{titles:[{type:"movie",id:$id}]}')" 200 "state batch")
jq -e '.[0].watched==true and .[0].watchlisted==true' <<<"$sb" >/dev/null || die "batch state incorrect"
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
adm2=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$ADMIN_USER" --arg p "$ADMIN_PASS" '{username:$u,password:$p}')" 200 "admin login after wipe")
ADMIN_TOKEN=$(jq -r '.access_token' <<<"$adm2")
after_wipe=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
if jq -e --arg u "$SMOKE_USER" 'any(.items[]?; .username==$u)' <<<"$after_wipe" >/dev/null; then
    die "[$SMOKE_USER] still present after DB wipe (wipe ineffective)"
fi
ok "server started EMPTY after wipe: admin present, [$SMOKE_USER] gone"

server_stop
compose run --rm "$SERVICE" recover --from "$SNAPSHOT" >/dev/null 2>&1 || die "recover failed"
ok "recovered from [$SNAPSHOT]"
server_start
poll_until "server healthy after recover" 120 health_ok
adm3=$(call_ok POST "$API/auth/login" "" "$(jq -nc --arg u "$ADMIN_USER" --arg p "$ADMIN_PASS" '{username:$u,password:$p}')" 200 "admin login after recover")
ADMIN_TOKEN=$(jq -r '.access_token' <<<"$adm3")
restored=$(call_ok GET "$API/users?limit=200" "$ADMIN_TOKEN")
jq -e --arg u "$SMOKE_USER" 'any(.items[]?; .username==$u)' <<<"$restored" >/dev/null || die "[$SMOKE_USER] missing after recover"
wl2=$(call_ok GET "$API/users/$SMOKE_UID/watchlist" "$ADMIN_TOKEN")
(( $(jq 'length' <<<"$wl2") >= 1 )) || die "watchlist empty after recover"
ok "recover restored [$SMOKE_USER] and its watchlist"

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
