#!/usr/bin/env bash
# Writes packaging/flatpak/cargo-sources.json from Cargo.lock: every crate the
# build needs, by URL and checksum, so flatpak-builder (and Flathub) compile
# with no network. Uses flatpak-builder-tools' generator at a pinned commit.
#
#   scripts/flatpak-cargo-sources.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
tools_commit=9d86d336a39b9af38b86f114b6ce5820fe260f91
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl -fsSL -o "$work/flatpak-cargo-generator.py" \
  "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/$tools_commit/cargo/flatpak-cargo-generator.py"
python3 -m venv "$work/venv"
"$work/venv/bin/pip" install --quiet aiohttp tomlkit
"$work/venv/bin/python" "$work/flatpak-cargo-generator.py" "$root/Cargo.lock" \
  -o "$root/packaging/flatpak/cargo-sources.json"
echo "wrote packaging/flatpak/cargo-sources.json"
