#!/usr/bin/env bash

CLIP_URL_BBB="https://download.blender.org/peach/bigbuckbunny_movies/BigBuckBunny_320x180.mp4.zip"
CLIP_URL_ED="https://archive.org/download/ElephantsDream/ed_1024_512kb.mp4"

CLIP_ATTRIBUTION_BBB="Big Buck Bunny (2008), (C) Blender Foundation, https://peach.blender.org, CC BY 3.0"
CLIP_ATTRIBUTION_ED="Elephants Dream (2006), (C) Blender Foundation / Netherlands Media Art Institute, https://www.elephantsdream.org, CC BY 3.0"

clip_cache_dir() {
    printf '%s' "${SHADOWMASK_CLIP_CACHE:-$1/media/.cache}"
}

clip_downloader() {
    if command -v curl >/dev/null 2>&1; then
        printf 'curl'
    elif command -v wget >/dev/null 2>&1; then
        printf 'wget'
    fi
}

_clip_fetch() {
    local url="$1" out="$2"
    case "$(clip_downloader)" in
        curl) curl -fSL --retry 3 -o "$out" "$url" ;;
        wget) wget -q -O "$out" "$url" ;;
        *) return 69 ;;
    esac
}

ensure_clip() {
    local cache_dir="$1" cache_name="$2" url="$3" mode="${4:-plain}"
    local cached="$cache_dir/$cache_name"
    if [[ -s "$cached" ]]; then
        printf '  ... reusing cached [%s]\n' "$cache_name" >&2
        printf '%s' "$cached"
        return 0
    fi
    mkdir -p "$cache_dir"
    local tmp
    tmp=$(mktemp)
    printf '  ... downloading [%s] from [%s]\n' "$cache_name" "$url" >&2
    _clip_fetch "$url" "$tmp" || {
        rm -f "$tmp"
        return 1
    }
    case "$mode" in
        zip)
            unzip -p "$tmp" >"$cached" || {
                rm -f "$tmp" "$cached"
                return 1
            }
            ;;
        trim:*)
            ffmpeg -nostdin -loglevel error -y -i "$tmp" -t "${mode#trim:}" -c copy "$cached" || {
                rm -f "$tmp" "$cached"
                return 1
            }
            ;;
        *)
            mv "$tmp" "$cached" || {
                rm -f "$tmp" "$cached"
                return 1
            }
            ;;
    esac
    rm -f "$tmp"
    printf '%s' "$cached"
}

place_clip() {
    local cached="$1" dest="$2"
    mkdir -p "$(dirname "$dest")"
    cp "$cached" "$dest"
}
