#!/bin/sh
set -eu

if [ "$#" -eq 0 ]; then
    printf 'Usage: %s <server-command> [args...]\n' "$0" >&2
    printf 'Example: %s cargo run -p server\n' "$0" >&2
    exit 64
fi

printf 'Shadowmask first-run bootstrap\n'

printf 'Admin username [admin]: '
read -r admin_username
admin_username=${admin_username:-admin}

printf 'Admin password: '
stty -echo 2>/dev/null || true
read -r admin_password
stty echo 2>/dev/null || true
printf '\n'

if [ -z "$admin_password" ]; then
    printf 'Admin password must not be empty.\n' >&2
    exit 1
fi

printf 'TMDB API key (optional, press enter to skip): '
read -r tmdb_api_key

: "${SHADOWMASK_BOOTSTRAP_DIR:=deployment/dev/config/bootstrap}"

SHADOWMASK_ADMIN_USERNAME="$admin_username"
SHADOWMASK_ADMIN_PASSWORD="$admin_password"
SHADOWMASK_BOOTSTRAP_MODE=init-and-start
export SHADOWMASK_ADMIN_USERNAME SHADOWMASK_ADMIN_PASSWORD SHADOWMASK_BOOTSTRAP_MODE SHADOWMASK_BOOTSTRAP_DIR

if [ -n "$tmdb_api_key" ]; then
    SHADOWMASK_TMDB_API_KEY="$tmdb_api_key"
    export SHADOWMASK_TMDB_API_KEY
fi

printf 'Starting server in bootstrap mode (%s) with dir %s\n' "$SHADOWMASK_BOOTSTRAP_MODE" "$SHADOWMASK_BOOTSTRAP_DIR"
exec "$@"
