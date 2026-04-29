# AUR Packaging

This directory contains two AUR package tracks:

- `backlayer`: release-based package files generated from a GitHub release source tarball
- `backlayer-git`: VCS package files that build from the latest repository state

## Layout

```text
packaging/aur/
  backlayer/
    PKGBUILD.in
    PKGBUILD
    .SRCINFO
  backlayer-git/
    PKGBUILD
    .SRCINFO
  render-release.sh
```

## Release package workflow

1. Create the release source tarball:

```bash
./packaging/release/create-source-tarball.sh
```

2. Render the release AUR files:

```bash
./packaging/aur/render-release.sh
```

That writes:

- `packaging/aur/backlayer/PKGBUILD`
- `packaging/aur/backlayer/.SRCINFO`

The generated release package expects a GitHub release asset at:

`https://github.com/MadsenDev/backlayer/releases/download/v<version>/backlayer-<version>-source.tar.gz`

## AUR submission notes

This repo does not publish to AUR automatically.

The usual manual flow is:

1. Clone the AUR package repo
2. Copy the prepared `PKGBUILD` and `.SRCINFO`
3. Commit
4. Push to the AUR remote

For users, the most likely commands would be:

- `yay -S backlayer`
- `yay -S backlayer-git`

## Daemon behavior

The package installs `backlayerd.service` as a user service and also includes a post-install message that recommends:

```bash
systemctl --user enable --now backlayerd.service
```

`backlayer-ui` can still spawn `backlayerd --serve` on demand if the daemon socket is missing, but the service remains the preferred long-running setup.
