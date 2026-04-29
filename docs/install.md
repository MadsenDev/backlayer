# Install Flow

Backlayer currently ships as a Hyprland-first development MVP.

## Runtime Requirements

- Hyprland on Wayland
- `ffmpeg`
- `ffprobe`
- Rust toolchain
- Node.js
- `pnpm`

Optional:

- `BACKLAYER_ENABLE_WORKSHOP=1` if you want the local Workshop import path visible

## Local Development Install

Install workspace dependencies:

```bash
pnpm install
```

Start the daemon:

```bash
cargo run -p backlayerd -- --serve
```

Start the manager UI:

```bash
pnpm ui:tauri:dev
```

The daemon socket is created at:

```text
~/.config/backlayer/backlayer.sock
```

## Autostart

Backlayer includes a `systemd --user` unit:

- `packaging/systemd/backlayerd.service`

See [systemd.md](./systemd.md) for the exact enable/install steps.

## Managed Asset Paths

- Native assets: `~/.config/backlayer/assets`
- Imported Workshop assets: `~/.config/backlayer/imports/wallpaper-engine`

## Current Packaging Reality

This repo now includes a first Arch/CachyOS package layout under `packaging/arch`.

Current supported setup paths are:

1. run/build from the workspace
2. build an Arch-style package with `cd packaging/arch && makepkg -si -f -p PKGBUILD`
3. enable the provided `systemd --user` service for daemon autostart when you want login-time startup
4. keep runtime dependencies available on the host system

For packaged installs, the app binary can also cold-start `backlayerd` on launch if the daemon socket is missing, but the user service remains the preferred persistent setup.

The repo also includes:

- GitHub Releases automation for a prebuilt Arch package artifact
- AUR-ready package files under `packaging/aur`
