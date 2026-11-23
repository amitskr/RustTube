#!/bin/sh
# Installs RustTube for the current user (no root needed).
# Usage: ./install.sh            install
#        ./install.sh --uninstall remove
set -e
here="$(cd "$(dirname "$0")" && pwd)"
bin="$HOME/.local/bin"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"

if [ "$1" = "--uninstall" ]; then
    rm -f "$bin/rusttube" "$apps/rusttube.desktop" "$icons/rusttube.png"
    echo "RustTube removed. Your data is still in ~/.local/share/RustTube"
    exit 0
fi

mkdir -p "$bin" "$apps" "$icons"
install -m 755 "$here/rusttube" "$bin/rusttube"
install -m 644 "$here/rusttube.png" "$icons/rusttube.png"
sed "s|^Exec=rusttube|Exec=$bin/rusttube|" "$here/rusttube.desktop" > "$apps/rusttube.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$apps" >/dev/null 2>&1 || true

echo "RustTube installed to $bin/rusttube"
if ! ldconfig -p 2>/dev/null | grep -q "libmpv\.so"; then
    echo
    echo "RustTube plays video through libmpv, which isn't installed yet:"
    echo "  Debian/Ubuntu:  sudo apt install libmpv2   (Ubuntu 22.04: libmpv1)"
    echo "  Fedora:         sudo dnf install mpv-libs"
    echo "  Arch:           sudo pacman -S mpv"
fi
echo "Optional, for downloads above 360p: yt-dlp and ffmpeg from your package manager."
