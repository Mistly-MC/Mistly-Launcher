#!/usr/bin/env bash
#  ███╗   ███╗██╗███████╗████████╗██╗     ██╗   ██╗
#  ████╗ ████║██║██╔════╝╚══██╔══╝██║     ╚██╗ ██╔╝
#  ██╔████╔██║██║███████╗   ██║   ██║      ╚████╔╝
#  ██║╚██╔╝██║██║╚════██║   ██║   ██║       ╚██╔╝
#  ██║ ╚═╝ ██║██║███████║   ██║   ███████╗   ██║
#  ╚═╝     ╚═╝╚═╝╚══════╝   ╚═╝   ╚══════╝   ╚═╝
#
#  Mistly Launcher · https://mistly.cc
#  Code : https://github.com/Mistly-MC/Mistly-Launcher
#  Discord : https://discord.gg/rdQJUgvUh3
#  Misty License 1.0 : free to use, modify and share, not to sell (LICENSE.txt)
#
# Prépare la version Linux de Mistly : publish/Misty-<version>/ et
# publish/Misty-<version>-linux-x86_64.tar.gz (portable + mise à jour automatique).
#
# Le dossier Misty-<version> doit être à la racine de l'archive (voir
# crates/misty-core/src/systeme/update.rs). La version est lue dans Cargo.toml à la racine.
#
# Dépendances de compilation (Debian / Ubuntu) :
#   sudo apt-get install -y pkg-config libfontconfig1-dev libxkbcommon-dev libgtk-3-dev
set -euo pipefail

racine="$(cd "$(dirname "$0")/.." && pwd)"
version="$(grep -m1 -E '^version = "' "$racine/Cargo.toml" | sed -E 's/^version = "([^"]+)".*/\1/')"
[ -n "$version" ] || { echo "Version introuvable dans Cargo.toml" >&2; exit 1; }
echo "Misty $version (Linux)"

cd "$racine"
cargo build --release -p misty

publish="$racine/publish"
dossier="$publish/Misty-$version"
archive="$publish/Misty-$version-linux-x86_64.tar.gz"
rm -rf "$dossier" "$archive"
mkdir -p "$dossier"
install -m 755 target/release/Misty "$dossier/Misty"
install -m 644 LICENSE.txt "$dossier/LICENSE.txt"
# icône des raccourcis et du menu des applications (systeme/shortcut.rs)
install -m 644 crates/misty-app/assets/icone.png "$dossier/icone.png"

tar -C "$publish" -czf "$archive" "Misty-$version"
echo "Archive : $archive"
