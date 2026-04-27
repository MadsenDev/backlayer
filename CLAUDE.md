# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Rust
cargo check                                          # fast workspace check
cargo build -p backlayerd                           # build daemon
cargo test -p backlayer-config -p backlayer-types -p scene-runner  # run tests
cargo run -p backlayerd -- --serve                  # run daemon (persistent mode)
cargo run -p backlayerd                             # run daemon (one-shot probe)
cargo run -p animation-probe                        # isolated shader crash debug tool

# Workshop mode
BACKLAYER_ENABLE_WORKSHOP=1 cargo run -p backlayerd -- --serve

# UI
pnpm install                # install deps
pnpm ui:dev                 # browser-only UI (mock data fallback)
pnpm ui:tauri:dev           # full Tauri + daemon UI
pnpm ui:build               # production UI build
pnpm ui:tauri:build         # production Tauri build
BACKLAYER_ENABLE_WORKSHOP=1 pnpm ui:tauri:dev
```

## Architecture

Backlayer is a Hyprland-first animated wallpaper runtime. The daemon owns all runtime state; the UI is display-only.

### Process model

- **`backlayerd`** (`apps/backlayerd`): long-running daemon. Owns config, monitor discovery, wallpaper-to-monitor assignment, renderer process supervision, and a Unix socket IPC server. Entry: `--serve` for persistent mode, no args for one-shot probe.
- **Runner workers** (each a separate supervised process per output):
  - `shader-runner` — WGSL shaders via `wgpu`
  - `video-runner` — video via FFmpeg (`ffmpeg`/`ffprobe` required); `libmpv` integration is future work
  - `scene-runner` — native `.backlayer` scene graphs; also handles Workshop `scene.pkg` extraction
  - `web-runner` — minimal local HTML extraction for Workshop web items
- **`apps/ui/src-tauri`**: Tauri shell. All daemon calls go through `daemon_request()` over the Unix socket (`~/.config/backlayer/backlayer.sock`). Tauri commands are async wrappers using `spawn_blocking`.
- **`apps/ui/src`**: React frontend (`App.tsx`, `api.ts`, `types.ts`). Falls back to mock data when Tauri/daemon is unavailable (browser-only mode).

### Crate responsibilities

- `backlayer-types`: shared domain types — `DaemonRequest`, `DaemonResponse`, `DaemonState`, `AssetMetadata`, scene document types, `CompositorClient` trait
- `backlayer-config`: config load/save, asset discovery, `.backlayer` package format, socket path resolution
- `backlayer-hyprland`: `hyprctl monitors` parsing, stable `monitor_id` derivation (`hypr:` prefix), implements `CompositorClient`
- `backlayer-kde`: Wayland-native monitor discovery via `wl_output`/sctk, implements `CompositorClient` (`kde:` prefix, fullscreen detection returns `false`)
- `backlayer-wayland`: `smithay-client-toolkit` layer-shell session abstraction, output binding
- `backlayer-renderer-{image,shader,video}`: renderer contracts (not the runner processes themselves)

### Compositor detection

`detect_compositor()` in `apps/backlayerd/src/main.rs` checks `XDG_CURRENT_DESKTOP` and `HYPRLAND_INSTANCE_SIGNATURE` to select `HyprlandClient` or `KdeClient`. The selected `Arc<dyn CompositorClient>` is threaded through `IpcServer` and `RuntimeManager`. KDE's `pause_on_fullscreen` always returns `false` (fullscreen detection not yet implemented for KDE).

### IPC

Tauri → daemon: newline-framed JSON over Unix socket. Request type: `DaemonRequest` (serde). Response type: `DaemonResponse`. The Tauri side lives in `apps/ui/src-tauri/src/main.rs`; the server side in `apps/backlayerd/src/ipc.rs`.

### Asset format

Native wallpapers are single-file `.backlayer` packages. Demo assets live in `assets/`. User-created assets land in `~/.config/backlayer/assets`. Workshop imports land in `~/.config/backlayer/imports/wallpaper-engine`.

Animated shader assets must declare a `ProbeUniforms` block (`time_seconds`, `width`, `height`, `_padding`) and set `animated = true` in config.

### Config paths

| Path | Purpose |
|------|---------|
| `~/.config/backlayer/config.toml` | runtime config |
| `~/.config/backlayer/backlayer.sock` | IPC socket |
| `~/.config/backlayer/assets` | user-created native wallpapers |
| `~/.config/backlayer/imports/wallpaper-engine` | imported Workshop items |

## Working rules (from AGENTS.md)

- `TODO.md` is the source of truth for execution status. Check it before substantial changes; check off tasks in the same changeset.
- `PROJECT_SUMMARY.md` is the source of truth for product direction.
- Do not expand scope to other desktop environments during MVP work.
- `CHANGELOG.md` must be updated (under `Unreleased`) for every user-visible change.
- Version bumps require a `CHANGELOG.md` update in the same changeset.
- Keep docs aligned with implementation reality.
