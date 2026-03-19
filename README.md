# Backlayer

Backlayer is a Hyprland-first animated wallpaper runtime for Linux.

It is built around a long-running daemon, Wayland layer-shell surfaces, dedicated renderer processes, a native scene composer for creating animated wallpapers from images, and a native `.backlayer` single-file wallpaper format.

## Current Scope

Backlayer currently targets:

- `Hyprland`
- `wlroots` layer-shell
- native `image`, `shader`, and `scene` wallpapers
- a Tauri manager UI

Workshop compatibility exists behind `BACKLAYER_ENABLE_WORKSHOP=1`, but native support remains the primary direction.

## What Works Now

- Per-monitor wallpaper assignment through `backlayerd`
- Static image wallpapers with fit controls
- First-pass video wallpapers through `video-runner` using `ffmpeg` / `ffprobe`
- WGSL shader wallpapers, including animated shaders
- Native scene wallpapers with:
  - sprite nodes
  - effect nodes
  - particle emitters
  - a full-screen Scene Composer
- Native create flow for:
  - still image wallpapers
  - video wallpapers
  - WGSL shader wallpapers
  - scene wallpapers through the Scene Composer
- Native wallpapers stored as single-file `.backlayer` assets
- Pause-on-fullscreen and pause-on-battery behavior
- Renderer supervision and `systemd --user` daemon recovery support

## What Is Still In Progress

- `libmpv` integration and hardware-accelerated video playback
- More validation around Hyprland hotplug and multi-monitor behavior
- Particle image sprites in the native scene engine
- Tighter parity between Scene Composer preview and applied wallpaper
- Broader Workshop runtime support

## Repo Layout

- `apps/backlayerd`: daemon entrypoint
- `apps/ui`: React manager UI
- `apps/ui/src-tauri`: Tauri desktop shell
- `apps/scene-runner`: native scene runtime worker
- `apps/shader-runner`: shader runtime worker
- `apps/video-runner`: video runtime worker
- `apps/web-runner`: web runtime worker
- `crates/backlayer-types`: shared domain types
- `crates/backlayer-config`: config loading, persistence, and asset packaging
- `crates/backlayer-hyprland`: Hyprland integration
- `crates/backlayer-wayland`: layer-shell / Wayland integration
- `crates/backlayer-renderer-image`: image renderer contracts
- `crates/backlayer-renderer-video`: video renderer contracts
- `crates/backlayer-renderer-shader`: shader renderer contracts

## Quick Start

Requirements:

- Rust toolchain
- Node.js
- pnpm

Install dependencies:

```bash
pnpm install
```

Run the daemon:

```bash
cargo run -p backlayerd -- --serve
```

Run the manager:

```bash
pnpm ui:tauri:dev
```

Browser-only UI development is also available:

```bash
pnpm ui:dev
```

The browser build falls back to mock data when Tauri/daemon access is unavailable.

## Configuration And Paths

- Config file: `~/.config/backlayer/config.toml`
- Daemon socket: `~/.config/backlayer/backlayer.sock`
- User-created native assets: `~/.config/backlayer/assets`
- Imported Workshop assets: `~/.config/backlayer/imports/wallpaper-engine`

See [docs/configuration.md](docs/configuration.md) for the config format and asset notes.

## Native Scene Composer

The manager includes a full-screen Scene Composer for native scene wallpapers.

Current native scene capabilities:

- base image sprites and overlay sprites
- glow, vignette, scanlines, and fog effects
- particle emitters with:
  - `point`, `box`, `line`, and `circle` shapes
  - burst emission
  - speed and lifetime ranges
  - over-life size, alpha, and color curves
  - positioned origins, direction control, and tint

Current limitations:

- particle image sprites are not finished in the runtime
- preview/runtime parity still needs work
- the editor is not yet a full creator-grade tool

## Workshop Compatibility

Workshop compatibility is disabled by default.

Enable it explicitly:

```bash
BACKLAYER_ENABLE_WORKSHOP=1 cargo run -p backlayerd -- --serve
BACKLAYER_ENABLE_WORKSHOP=1 pnpm ui:tauri:dev
```

When enabled, Backlayer can import local Wallpaper Engine workshop items into managed storage and classify them as `video`, `scene`, or `web`, but compatibility is still partial.

## Development Commands

Build the Rust workspace:

```bash
cargo check
```

Run focused tests:

```bash
cargo test -p backlayer-config -p backlayer-types -p scene-runner
```

Build the UI:

```bash
pnpm ui:build
```

## Related Docs

- [PROJECT_SUMMARY.md](PROJECT_SUMMARY.md)
- [TODO.md](TODO.md)
- [CHANGELOG.md](CHANGELOG.md)
- [docs/install.md](docs/install.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/limitations.md](docs/limitations.md)
- [docs/manual-testing.md](docs/manual-testing.md)
- [docs/configuration.md](docs/configuration.md)
- [docs/systemd.md](docs/systemd.md)
- [docs/animation-probe.md](docs/animation-probe.md)

## Built-In Demo Assets

The repo currently ships with native demo assets for each main path:

- `demo.cyberpunk-city` image wallpaper
- `demo.sunset-stripes` image wallpaper
- `demo.prism-loop` video wallpaper
- `demo.neon-grid` static shader
- `demo.ember-scan` static shader
- `demo.tide-pulse` animated shader
