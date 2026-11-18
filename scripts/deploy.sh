#!/usr/bin/env bash
set -euo pipefail

# Run from WSL with Ryujinx closed. Deploy only into this plugin's own mod folder.
cd "$(dirname "$0")/.."

if [[ -n "${RYUJINX_DIR:-}" ]]; then
    ryujinx_dir="$RYUJINX_DIR"
else
    if ! command -v cmd.exe >/dev/null || ! command -v wslpath >/dev/null; then
        echo "Ryujinx path unknown; set RYUJINX_DIR." >&2
        exit 1
    fi
    appdata="$(cmd.exe /c 'echo %APPDATA%' 2>/dev/null | tr -d '\r')"
    if [[ -z "$appdata" || "$appdata" == *'%APPDATA%'* ]]; then
        echo "Cannot locate APPDATA; set RYUJINX_DIR." >&2
        exit 1
    fi
    ryujinx_dir="$(wslpath -u "$appdata")/Ryujinx"
fi

source_nro="target/aarch64-skyline-switch/release/libengage_attack_evolution.nro"
dest_dir="$ryujinx_dir/sdcard/engage/mods/Attack_Evolution"
dest="$dest_dir/libengage_attack_evolution.nro"

cargo skyline build --release

if [[ ! -f "$source_nro" ]]; then
    echo "Compiled NRO not found: $source_nro" >&2
    exit 1
fi

mkdir -p -- "$dest_dir"
if [[ -e "$dest" ]]; then
    cp -p -- "$dest" "$dest.bak"
fi
cp -- "$source_nro" "$dest"
echo "Installed $dest"
