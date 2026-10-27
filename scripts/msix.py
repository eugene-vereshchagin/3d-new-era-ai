#!/usr/bin/env python3
"""Packs the Windows build as an MSIX for the Microsoft Store.

    python scripts/msix.py <version> <dir with newera.exe and newera-gui.exe> <out.msix>

Runs on Windows (makeappx comes with the Windows SDK the CI runners have).
The package is unsigned: the Store signs what it publishes. The logos are
drawn from assets/icon-1024.png; the manifest is packaging/msix/AppxManifest.xml.
"""

import glob
import os
import re
import shutil
import subprocess
import sys
import tempfile

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# name: (width, height) — the sizes the manifest names, at scale 100.
LOGOS = {
    "StoreLogo": (50, 50),
    "Square44x44Logo": (44, 44),
    "Square150x150Logo": (150, 150),
    "Square310x310Logo": (310, 310),
    "Wide310x150Logo": (310, 150),
}


def store_version(version: str) -> str:
    """`3.2.0` → `3.2.0.0`: the Store reserves the fourth part."""
    parts = version.lstrip("v").split(".")
    if len(parts) != 3 or not all(p.isdigit() for p in parts):
        sys.exit(f"not a release version: {version}")
    return ".".join(parts + ["0"])


def logo(icon: Image.Image, size: tuple[int, int]) -> Image.Image:
    """The icon centred on a transparent canvas, as large as fits."""
    side = min(size)
    canvas = Image.new("RGBA", size, (0, 0, 0, 0))
    mark = icon.resize((side, side), Image.LANCZOS)
    canvas.paste(mark, ((size[0] - side) // 2, (size[1] - side) // 2), mark)
    return canvas


def makeappx() -> str:
    found = sorted(glob.glob(r"C:\Program Files (x86)\Windows Kits\10\bin\10.*\x64\makeappx.exe"))
    if not found:
        sys.exit("makeappx.exe not found: is the Windows SDK installed?")
    return found[-1]


def main() -> None:
    version, build, out = sys.argv[1:4]
    layout = tempfile.mkdtemp(prefix="msix-")
    for exe in ("newera.exe", "newera-gui.exe"):
        shutil.copy(os.path.join(build, exe), layout)
    for name in ("LICENSE-MIT", "LICENSE-APACHE"):
        shutil.copy(os.path.join(ROOT, name), layout)

    assets = os.path.join(layout, "Assets")
    os.makedirs(assets)
    icon = Image.open(os.path.join(ROOT, "assets", "icon-1024.png")).convert("RGBA")
    for name, size in LOGOS.items():
        logo(icon, size).save(os.path.join(assets, f"{name}.png"))

    with open(os.path.join(ROOT, "packaging", "msix", "AppxManifest.xml"), encoding="utf-8") as f:
        manifest = f.read().replace("__VERSION__", store_version(version))
    assert not re.search(r"__[A-Z]+__", manifest), "a placeholder was left in the manifest"
    with open(os.path.join(layout, "AppxManifest.xml"), "w", encoding="utf-8") as f:
        f.write(manifest)

    subprocess.run([makeappx(), "pack", "/o", "/d", layout, "/p", out], check=True)
    print(f"packed {out} ({os.path.getsize(out) // 1024} KiB)")


if __name__ == "__main__":
    main()
