#!/usr/bin/env bash
set -euo pipefail

# Explicit test-only installer. Modifies the ACTIVE Ryujinx modpack XML,
# with a dated backup stored OUTSIDE patches/xml in Attack_Evolution/backups.
# Run only with Ryujinx closed and from the plugin's feature branch.
cd "$(dirname "$0")/.."

if [[ -n "${RYUJINX_DIR:-}" ]]; then
    ryujinx_dir="$RYUJINX_DIR"
else
    if ! command -v cmd.exe >/dev/null || ! command -v wslpath >/dev/null; then
        echo "Ryujinx path unknown; set RYUJINX_DIR." >&2
        exit 1
    fi
    appdata="$(cmd.exe /c 'echo %APPDATA%' 2>/dev/null | tr -d '\r')"
    ryujinx_dir="$(wslpath -u "$appdata")/Ryujinx"
fi

xml="$ryujinx_dir/sdcard/engage/mods/Emblem_Rework/patches/xml/Skill.xml"
if [[ ! -f "$xml" ]]; then
    echo "Current Skill.xml not found: $xml" >&2
    exit 1
fi
# Refuse stale XML versions so the user's hand-authored + row is not lost.
if ! grep -Fq 'Sid="SID_セリカエンゲージ技＋"' "$xml"; then
    echo "The current Skill.xml is missing the authored full-width + root." >&2
    echo "First sync your current engage_modpack XML, then retry." >&2
    exit 1
fi

tmp_dir="$(mktemp -d)"
tmp="$tmp_dir/Skill.celica-plus.xml"
trap 'rm -f -- "$tmp"; rmdir -- "$tmp_dir"' EXIT
python3 tools/generate_celica_plus.py "$xml" "$tmp"

backup_dir="$ryujinx_dir/sdcard/engage/mods/Attack_Evolution/backups"
mkdir -p -- "$backup_dir"
backup="$backup_dir/Skill.xml.before-plus.$(date +%Y%m%d_%H%M%S).bak"
cp -p -- "$xml" "$backup"
cp -- "$tmp" "$xml"
echo "Test XML installed: $xml"
echo "Original XML backed up: $backup"
echo "Note: your local engage_modpack Git working tree is now modified."
