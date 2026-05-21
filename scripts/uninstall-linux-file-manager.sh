#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    cat <<'EOF'
Remove the mdo Linux file-manager integration for the current user.

Usage:
  uninstall-linux-file-manager.sh

Removes:
  ~/.local/share/applications/mdo.desktop
  ~/.local/share/icons/hicolor/scalable/apps/mdo.svg
  ~/.local/share/nautilus/scripts/Render with mdo

It also removes mdo.desktop from Markdown MIME defaults/associations
in per-user mimeapps.list files.
EOF
    exit 0
fi

desktop_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
desktop_file="$desktop_dir/mdo.desktop"
icon_root="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"
icon_file="$icon_root/scalable/apps/mdo.svg"
nautilus_script="${XDG_DATA_HOME:-$HOME/.local/share}/nautilus/scripts/Render with mdo"

remove_file() {
    local path="$1"
    if [[ -e "$path" ]]; then
        rm -f -- "$path"
        echo "Removed: $path"
    else
        echo "Skip   : $path (not present)"
    fi
}

remove_file "$desktop_file"
remove_file "$icon_file"
remove_file "$nautilus_script"

for file in \
    "${XDG_CONFIG_HOME:-$HOME/.config}/mimeapps.list" \
    "${XDG_DATA_HOME:-$HOME/.local/share}/applications/mimeapps.list"
do
    [[ -f "$file" ]] || continue
    tmp="$(mktemp)"
    awk '
        /^\[Default Applications\]$/ { section = "default"; print; next }
        /^\[Added Associations\]$/ { section = "added"; print; next }
        /^\[/ { section = ""; print; next }
        section == "default" && ($0 ~ /^text\/markdown=mdo.desktop/ || $0 ~ /^text\/x-markdown=mdo.desktop/) { next }
        section == "added" && ($0 ~ /^text\/markdown=/ || $0 ~ /^text\/x-markdown=/) {
            sub(/(^text\/[^=]+=|;)mdo.desktop;/, "\\1")
            sub(/(^text\/[^=]+=|;)mdo.desktop$/, "\\1")
            if ($0 ~ /^text\/[^=]+=$/) next
        }
        { print }
    ' "$file" > "$tmp"
    mv "$tmp" "$file"
    echo "Updated: $file"
done

if command -v update-desktop-database >/dev/null 2>&1 && [[ -d "$desktop_dir" ]]; then
    update-desktop-database "$desktop_dir" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1 && [[ -d "$icon_root" ]]; then
    gtk-update-icon-cache -q "$icon_root" >/dev/null 2>&1 || true
fi

echo "Done."
