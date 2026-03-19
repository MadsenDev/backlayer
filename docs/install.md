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

This repo does not yet ship distro-specific packages.

The supported install story right now is:

1. run/build from the workspace
2. enable the provided `systemd --user` service for daemon autostart
3. keep runtime dependencies available on the host system
