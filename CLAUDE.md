# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Rust
cargo check                                          # fast workspace check
cargo build -p backlayerd                           # build daemon
cargo test --workspace                              # run all tests
cargo test -p backlayer-config                      # test one crate
cargo test -p scene-runner composes                 # run tests matching a name filter
cargo run -p backlayerd -- --serve                  # run daemon (persistent mode)
cargo run -p backlayerd                             # run daemon (one-shot probe)
cargo run -p backlayerctl -- doctor                 # diagnose session/daemon health (--json for machine output)
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

Tests live in `backlayerd` (ipc, runtime), `scene-runner`, `web-runner`, `backlayer-config`, `backlayer-types`, `backlayer-hyprland`, `backlayer-kde`, and the image/video renderer crates.

## Architecture

Backlayer is a Hyprland-first animated wallpaper runtime. The daemon owns all runtime state; the UI is display-only.

### Process model

- **`backlayerd`** (`apps/backlayerd`): long-running daemon. Owns config, monitor discovery, wallpaper-to-monitor assignment, renderer process supervision, and a Unix socket IPC server. Entry: `--serve` for persistent mode, no args for one-shot probe; unrecognized args are rejected with usage output.
- **`backlayerctl`** (`apps/backlayerctl`): diagnostic CLI. `doctor` checks compositor environment, config, socket health, monitors, runtime sessions, renderer events, video dependencies, and assets.
- **Runner workers** (each a separate supervised process per output):
  - `shader-runner` — WGSL shaders via `wgpu`
  - `video-runner` — video via FFmpeg (`ffmpeg`/`ffprobe` required); respects FPS cap and pause policies; `libmpv` integration is future work
  - `scene-runner` — native `.backlayer` scene graphs; also handles Workshop `scene.pkg` extraction
  - `web-runner` — minimal local HTML extraction for Workshop web items
- **`apps/ui/src-tauri`**: Tauri shell. All daemon calls go through `daemon_request()` over the Unix socket (`~/.config/backlayer/backlayer.sock`). Tauri commands are async wrappers using `spawn_blocking`.
- **`apps/ui/src`**: React frontend. Nearly everything — including the full-screen Scene Composer — lives in `App.tsx` (~5,700 lines); `api.ts` and `types.ts` mirror the daemon IPC contract. Falls back to mock data when Tauri/daemon is unavailable (browser-only mode).

### Crate responsibilities

- `backlayer-types`: shared domain types — `DaemonRequest`, `DaemonResponse`, `DaemonState`, `AssetMetadata`, scene document types, `CompositorClient` trait
- `backlayer-config`: config load/save, asset discovery (workspace and installed layouts like `/usr/share/backlayer/assets`), `.backlayer` package format, socket path resolution
- `backlayer-hyprland`: `hyprctl monitors` parsing, stable `monitor_id` derivation (`hypr:` prefix), implements `CompositorClient`
- `backlayer-kde`: Wayland-native monitor discovery via `wl_output`/sctk — hosts `WaylandOutputClient`, which implements `CompositorClient` for both KDE (`kde:` prefix) and the generic layer-shell fallback (`wl:` prefix); fullscreen detection returns `false` on both
- `backlayer-wayland`: `smithay-client-toolkit` layer-shell session abstraction, output binding
- `backlayer-renderer-{image,shader,video}`: renderer contracts (not the runner processes themselves)

### Compositor detection

`detect_compositor()` in `apps/backlayerd/src/main.rs` checks `XDG_CURRENT_DESKTOP` and `HYPRLAND_INSTANCE_SIGNATURE` to select `HyprlandClient`, `KdeClient` (KDE/Plasma), or — for any other Wayland session (`WAYLAND_DISPLAY` set; Niri, Sway, river, ...) — the generic `WaylandOutputClient` fallback. Non-Wayland sessions (X11, TTY) fail fast with an "unsupported session" error rather than defaulting to Hyprland. The selected `Arc<dyn CompositorClient>` is threaded through `IpcServer` and `RuntimeManager`. Fullscreen detection (`pause_on_fullscreen`) returns `false` on every non-Hyprland path.

### IPC

Tauri → daemon: newline-framed JSON over Unix socket. Request type: `DaemonRequest` (serde). Response type: `DaemonResponse`. The Tauri side lives in `apps/ui/src-tauri/src/main.rs`; the server side in `apps/backlayerd/src/ipc.rs`.

### Asset format

Native wallpapers are single-file `.backlayer` packages. Demo assets live in `assets/`. User-created assets land in `~/.config/backlayer/assets`. Workshop imports land in `~/.config/backlayer/imports/wallpaper-engine`.

Animated shader assets must declare a `ProbeUniforms` block (`time_seconds`, `width`, `height`, `_padding`) and set `animated = true` in config.

### Scene Composer ↔ scene-runner parity

The Scene Composer preview (in `App.tsx`) and `scene-runner` implement the same scene semantics twice — preview parity is a headline focus for the v0.3.0 release, not polish. `docs/scene-semantics-spec.md` is the normative definition of scene evaluation (units, time base, curves, blending); `docs/scene-parity-audit.md` catalogs known divergences. When changing particle/effect/sprite behavior on either side, update the spec and apply the matching change on the other (or record the divergence in the audit).

### KDE Plasma plugin (`integrations/kde-plasma`)

A Plasma 6 wallpaper plugin (QML) that reads `~/.config/backlayer/config.toml` on a timer and renders `image`, `video` (QtMultimedia), and a simplified `scene` path natively in Plasma — it is a Plasma-specific adapter, not a reuse of the layer-shell runtime. `shader`/`web` are unsupported there. This is an explicitly approved narrow exception to the Hyprland-first scope rule.

### Packaging and CI

- `packaging/arch`: PKGBUILD, `.desktop` file, install scripts for Arch/CachyOS
- `packaging/aur`: AUR-ready layouts for `backlayer` and `backlayer-git`
- `packaging/systemd`: `systemd --user` units for daemon recovery
- `.github/workflows`: `pages.yml` deploys the `site/` landing page from `main`; `release-arch-package.yml` builds the Arch package

## Config paths

| Path | Purpose |
|------|---------|
| `~/.config/backlayer/config.toml` | runtime config |
| `~/.config/backlayer/backlayer.sock` | IPC socket |
| `~/.config/backlayer/assets` | user-created native wallpapers |
| `~/.config/backlayer/imports/wallpaper-engine` | imported Workshop items |

## Working rules (from AGENTS.md)

- `TODO.md` is the source of truth for execution status. It is a milestone roadmap (currently driving toward a v0.3.0 first public release); check it before substantial changes and check off tasks in the same changeset.
- `PROJECT_SUMMARY.md` is the source of truth for product direction.
- Do not expand scope to other desktop environments during MVP work (the KDE Plasma plugin bridge is the one approved exception).
- `CHANGELOG.md` must be updated (under `Unreleased`) for every user-visible change.
- Version bumps require a `CHANGELOG.md` update in the same changeset.
- Keep docs aligned with implementation reality. Reference docs live in `docs/` (architecture, configuration, install, limitations, manual-testing, systemd, animation-probe, scene-parity-audit, scene-semantics-spec).
