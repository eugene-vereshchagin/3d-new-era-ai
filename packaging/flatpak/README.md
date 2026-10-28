# Flatpak

[`io.github.leandrodaf.newera.yml`](io.github.leandrodaf.newera.yml) builds 3D New
Era AI from source with no network, the way Flathub requires. The crates come
from `cargo-sources.json`, which
[`scripts/flatpak-cargo-sources.sh`](../../scripts/flatpak-cargo-sources.sh)
writes from `Cargo.lock`. That file is generated, not committed.
[`flatpak.yml`](../../.github/workflows/flatpak.yml) builds, installs and runs
it whenever the manifest, the desktop files or the crates change.

## Build it locally

```sh
scripts/flatpak-cargo-sources.sh
flatpak install -y flathub org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.rust-stable//25.08
flatpak-builder --user --install --force-clean build-dir packaging/flatpak/io.github.leandrodaf.newera.yml
flatpak run io.github.leandrodaf.newera
```

## Submit it to Flathub

Flathub reviews the first submission by hand. After that, the app lives in its
own repository under `flathub/io.github.leandrodaf.newera`, and each release is
a pull request there.

1. Fork [flathub/flathub](https://github.com/flathub/flathub) and branch off
   `new-pr`.
2. Copy the manifest and a fresh `cargo-sources.json` to the fork's root. In the
   copy, replace the `type: dir` source with the release tag:

   ```yaml
   - type: git
     url: https://github.com/leandrodaf/3d-new-era-ai.git
     tag: v3.2.0
     commit: <the tag's commit>
   ```

3. Open a pull request against `new-pr` titled `Add io.github.leandrodaf.newera`.
   The `io.github.leandrodaf` id is verified through the `leandrodaf` GitHub
   account.

## Each release after that

Flathub's update bot only follows sources marked with `x-checker-data`, and it
cannot regenerate the crate list, so a release is updated by hand:

1. In the app's Flathub repository, set the git source's `tag` and `commit` to
   the new release.
2. Run `scripts/flatpak-cargo-sources.sh` on that tag and copy the new
   `cargo-sources.json` over the old one.
3. Open the pull request. Flathub builds it, and merging publishes the update to
   every install.
