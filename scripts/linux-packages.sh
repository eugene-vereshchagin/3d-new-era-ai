#!/usr/bin/env bash
# Turns the release's Linux tarball into the packages people install by
# double-clicking: a .deb (Debian, Ubuntu, Mint, Pop!_OS), an .rpm (Fedora,
# RHEL, openSUSE) and an AppImage (any distro, nothing to install). Every one
# holds the very binary the tarball ships, with the same menu entry, icons,
# .newera file type and AppStream metadata install-desktop.sh puts in place.
#
#   scripts/linux-packages.sh newera-linux-x64.tar.gz 3.2.0 dist/
#
# Writes newera-linux-x64.deb, newera-linux-x64.rpm and
# newera-linux-x64.AppImage into the output folder. Needs dpkg-deb, rpmbuild
# and appimagetool on the PATH (release.yml installs them).
set -euo pipefail

tarball=$(realpath "$1")
version=$2
out=$(mkdir -p "$3" && realpath "$3")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

tar -xzf "$tarball" -C "$work"
src="$work/newera"
[ -x "$src/newera" ] || { echo "no newera binary in $tarball" >&2; exit 1; }

# One tree in the system's own places, shared by the .deb and the .rpm.
root="$work/root"
install -Dm755 "$src/newera" "$root/usr/bin/newera"
install -Dm644 "$src/share/applications/newera.desktop" "$root/usr/share/applications/io.github.leandrodaf.newera.desktop"
install -Dm644 "$src/share/mime/packages/newera.xml" "$root/usr/share/mime/packages/io.github.leandrodaf.newera.xml"
install -Dm644 "$src/share/metainfo/io.github.leandrodaf.newera.metainfo.xml" "$root/usr/share/metainfo/io.github.leandrodaf.newera.metainfo.xml"
# The metadata names the entry by its installed file.
sed -i 's|<launchable type="desktop-id">newera.desktop</launchable>|<launchable type="desktop-id">io.github.leandrodaf.newera.desktop</launchable>|' \
  "$root/usr/share/metainfo/io.github.leandrodaf.newera.metainfo.xml"
mkdir -p "$root/usr/share/icons"
cp -r "$src/share/icons/hicolor" "$root/usr/share/icons/"
install -Dm644 "$src/LICENSE-MIT" "$root/usr/share/licenses/newera/LICENSE-MIT"
install -Dm644 "$src/LICENSE-APACHE" "$root/usr/share/licenses/newera/LICENSE-APACHE"
install -Dm644 "$src/README.md" "$root/usr/share/doc/newera/README.md"
size_kb=$(du -sk "$root" | cut -f1)

summary="Home design editor with a built-in MCP server for AI agents"
describe="A home design editor (walls, doors, windows, joinery, lighting, cameras) with an MCP
server inside, so an AI agent works the same tools you do and every change shows up
on screen, with undo. Free and open source."

# --- .deb -------------------------------------------------------------------
# Debian names for what the window loads at run time: the binary links only
# libc and libgcc, and opens the graphics stack itself.
deb="$work/deb"
cp -a "$root" "$deb"
mkdir -p "$deb/DEBIAN"
install -Dm644 "$src/LICENSE-MIT" "$deb/usr/share/doc/newera/copyright"
cat > "$deb/DEBIAN/control" <<EOF
Package: newera
Version: $version
Architecture: amd64
Maintainer: Leandro Ferreira <contato@3dneweraai.com>
Installed-Size: $size_kb
Depends: libc6 (>= 2.35), libgcc-s1, libvulkan1, libegl1, libx11-6, libxcursor1, libxi6, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libwayland-egl1, libfontconfig1, libdbus-1-3
Recommends: mesa-vulkan-drivers | vulkan-icd, xdg-desktop-portal
Section: graphics
Priority: optional
Homepage: https://3dneweraai.com/
Description: $summary
$(printf '%s\n' "$describe" | sed 's/^/ /')
EOF
dpkg-deb --root-owner-group --build "$deb" "$out/newera-linux-x64.deb"

# --- .rpm -------------------------------------------------------------------
# No library names here: Fedora and openSUSE spell them differently, and any
# desktop that can show a window has them. The binary itself asks only for
# glibc 2.35.
rpmtop="$work/rpmbuild"
mkdir -p "$rpmtop"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,BUILDROOT}
cat > "$rpmtop/SPECS/newera.spec" <<EOF
Name:           newera
Version:        $version
Release:        1
Summary:        $summary
License:        MIT OR Apache-2.0
URL:            https://3dneweraai.com/
BuildArch:      x86_64
AutoReqProv:    no
Requires:       glibc >= 2.35

%description
$describe

%install
cp -a $root/. %{buildroot}/

%files
/usr/bin/newera
/usr/share/applications/io.github.leandrodaf.newera.desktop
/usr/share/mime/packages/io.github.leandrodaf.newera.xml
/usr/share/metainfo/io.github.leandrodaf.newera.metainfo.xml
/usr/share/icons/hicolor/*/apps/newera.*
/usr/share/icons/hicolor/*/mimetypes/application-x-newera.*
%license /usr/share/licenses/newera/LICENSE-MIT
%license /usr/share/licenses/newera/LICENSE-APACHE
%doc /usr/share/doc/newera/README.md

%post
update-mime-database /usr/share/mime >/dev/null 2>&1 || :
update-desktop-database /usr/share/applications >/dev/null 2>&1 || :
gtk-update-icon-cache -q /usr/share/icons/hicolor >/dev/null 2>&1 || :

%postun
update-mime-database /usr/share/mime >/dev/null 2>&1 || :
update-desktop-database /usr/share/applications >/dev/null 2>&1 || :
gtk-update-icon-cache -q /usr/share/icons/hicolor >/dev/null 2>&1 || :
EOF
rpmbuild --quiet --define "_topdir $rpmtop" --define "_build_id_links none" -bb "$rpmtop/SPECS/newera.spec"
cp "$rpmtop"/RPMS/x86_64/newera-*.rpm "$out/newera-linux-x64.rpm"

# --- AppImage ---------------------------------------------------------------
# One file that runs anywhere: the AppDir is the same tree, with the entry and
# the icon at its root where the AppImage runtime looks for them.
app="$work/AppDir"
cp -a "$root" "$app"
cp "$app/usr/share/applications/io.github.leandrodaf.newera.desktop" "$app/io.github.leandrodaf.newera.desktop"
cp "$app/usr/share/icons/hicolor/256x256/apps/newera.png" "$app/newera.png"
ln -s newera.png "$app/.DirIcon"
cat > "$app/AppRun" <<'EOF'
#!/bin/sh
here=$(dirname "$(readlink -f "$0")")
exec "$here/usr/bin/newera" "$@"
EOF
chmod +x "$app/AppRun"
ARCH=x86_64 appimagetool --no-appstream "$app" "$out/newera-linux-x64.AppImage" >/dev/null
chmod +x "$out/newera-linux-x64.AppImage"

ls -l "$out"/newera-linux-x64.{deb,rpm,AppImage}
