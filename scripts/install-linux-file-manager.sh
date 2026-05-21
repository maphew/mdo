#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Register mdo with Linux file managers for the current user.

Usage:
  install-linux-file-manager.sh [--exe PATH] [--set-default] [--no-nautilus-script]

Options:
  --exe PATH             Path to mdo. If omitted, uses PATH, then
                         ../target/release/mdo relative to this script.
  --set-default          Make mdo the default handler for Markdown files.
                         By default it is only added as an "Open With" option.
  --no-nautilus-script   Do not install the GNOME Files/Nautilus Scripts entry.
  -h, --help             Show this help.

Installs:
  ~/.local/share/applications/mdo.desktop
  ~/.local/share/icons/hicolor/scalable/apps/mdo.svg
  ~/.local/share/nautilus/scripts/Render with mdo   (when Nautilus exists)
EOF
}

exe_path=""
set_default=0
install_nautilus_script=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        --exe)
            [[ $# -ge 2 ]] || { echo "Missing value for --exe" >&2; exit 2; }
            exe_path="$2"
            shift 2
            ;;
        --set-default)
            set_default=1
            shift
            ;;
        --no-nautilus-script)
            install_nautilus_script=0
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

resolve_exe() {
    local hint="$1"

    if [[ -n "$hint" ]]; then
        if [[ ! -x "$hint" ]]; then
            echo "mdo is not executable at: $hint" >&2
            exit 1
        fi
        realpath "$hint"
        return
    fi

    if command -v mdo >/dev/null 2>&1; then
        command -v mdo
        return
    fi

    local local_build="$script_dir/../target/release/mdo"
    if [[ -x "$local_build" ]]; then
        realpath "$local_build"
        return
    fi

    echo "Could not locate mdo. Build it with 'cargo build --release', install it on PATH, or pass --exe PATH." >&2
    exit 1
}

quote_desktop_exec_arg() {
    # Desktop Entry Exec quoting: wrap in double quotes and escape the chars
    # that are special inside quoted arguments.
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//\$/\\\$}"
    value="${value//\`/\\\`}"
    printf '"%s"' "$value"
}

exe="$(resolve_exe "$exe_path")"
desktop_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
desktop_file="$desktop_dir/mdo.desktop"
icon_dir="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
icon_file="$icon_dir/mdo.svg"
mkdir -p "$desktop_dir"
mkdir -p "$icon_dir"

cat > "$icon_file" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <rect width="256" height="256" rx="48" fill="#f7f7f7"/>
  <text x="128" y="151"
        text-anchor="middle"
        dominant-baseline="middle"
        font-family="Segoe UI Symbol, Noto Sans Symbols 2, Noto Sans Symbols, DejaVu Sans, sans-serif"
        font-size="176"
        font-weight="700"
        fill="#1e66e2">Ⓜ</text>
</svg>
EOF

quoted_exe="$(quote_desktop_exec_arg "$exe")"
cat > "$desktop_file" <<EOF
[Desktop Entry]
Type=Application
Name=mdo
GenericName=Markdown HTML Previewer
Comment=Render Markdown as HTML and open it in the default browser
Exec=$quoted_exe --open %f
Icon=mdo
Terminal=false
NoDisplay=true
MimeType=text/markdown;text/x-markdown;
Categories=Utility;TextTools;
StartupNotify=false
EOF
chmod 0644 "$desktop_file"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$desktop_dir" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" >/dev/null 2>&1 || true
fi

if [[ "$set_default" -eq 1 ]] && command -v xdg-mime >/dev/null 2>&1; then
    xdg-mime default mdo.desktop text/markdown || true
    xdg-mime default mdo.desktop text/x-markdown || true
fi

if [[ "$install_nautilus_script" -eq 1 ]] && command -v nautilus >/dev/null 2>&1; then
    nautilus_dir="${XDG_DATA_HOME:-$HOME/.local/share}/nautilus/scripts"
    nautilus_script="$nautilus_dir/Render with mdo"
    mkdir -p "$nautilus_dir"
    cat > "$nautilus_script" <<EOF
#!/usr/bin/env bash
set -euo pipefail

exe=$(printf '%q' "$exe")

if [[ -n "\${NAUTILUS_SCRIPT_SELECTED_FILE_PATHS:-}" ]]; then
    while IFS= read -r path; do
        [[ -n "\$path" ]] || continue
        "\$exe" --open "\$path" &
    done <<< "\$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS"
else
    for uri in "\$@"; do
        path="\${uri#file://}"
        "\$exe" --open "\$path" &
    done
fi
EOF
    chmod 0755 "$nautilus_script"
fi

echo "Installed desktop entry: $desktop_file"
echo "Installed icon: $icon_file"
if [[ "$install_nautilus_script" -eq 1 ]] && command -v nautilus >/dev/null 2>&1; then
    echo "Installed Nautilus script: ${XDG_DATA_HOME:-$HOME/.local/share}/nautilus/scripts/Render with mdo"
    echo "In GNOME Files, use right-click -> Scripts -> Render with mdo."
fi
if [[ "$set_default" -eq 1 ]]; then
    echo "mdo is now the default Markdown handler."
else
    echo "mdo is available from Open With without changing your default Markdown handler."
fi
