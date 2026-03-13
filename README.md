# Backlayer

Backlayer is a Hyprland-first animated wallpaper runtime for Linux.

The repo is split into:

- `apps/backlayerd`: daemon entrypoint
- `apps/ui`: React manager app
- `apps/ui/src-tauri`: Tauri desktop host for the manager
- `crates/backlayer-types`: shared domain types
- `crates/backlayer-config`: config loading and persistence
- `crates/backlayer-hyprland`: Hyprland integration
- `crates/backlayer-wayland`: layer-shell and Wayland integration
- `crates/backlayer-renderer-image`: image renderer contracts
- `crates/backlayer-renderer-video`: video renderer contracts
- `crates/backlayer-renderer-shader`: shader renderer contracts
- `apps/video-runner`: dedicated child process for the current video preview-fallback path
- `apps/scene-runner`: dedicated child process for native Backlayer scene playback and imported-scene fallback
- `apps/web-runner`: dedicated child process for the current web preview-fallback path

## Local setup

Requirements:

- Rust toolchain
- Node.js
- pnpm

Install frontend dependencies:

```bash
pnpm install
```

Run the manager UI in a browser:

```bash
pnpm ui:dev
```

The browser build uses mock runtime data when Tauri/daemon access is unavailable.

Build the Rust workspace:

```bash
cargo check
```

Run the daemon stub:

```bash
cargo run -p backlayerd
```

Run the daemon in IPC server mode:

```bash
cargo run -p backlayerd -- --serve
```

Run the manager app through Tauri:

```bash
pnpm ui:tauri:dev
```

Sample asset metadata is under `assets/demo.neon-grid/backlayer.toml`, `assets/demo.tide-pulse/backlayer.toml`, and `assets/demo.sunset-stripes/backlayer.toml`.
Configuration format is documented in `docs/configuration.md`.
Process supervision is documented in `docs/systemd.md`.
Animated shader debugging is documented in `docs/animation-probe.md`.
Local Wallpaper Engine import behavior is documented in `docs/configuration.md`.
Native scene assets created through the manager are stored under `~/.config/backlayer/assets`.

## Current decisions

- Workspace layout: Rust workspace at the repo root
- Config format: TOML at `~/.config/backlayer/config.toml`
- Wallpaper metadata: per-asset `backlayer.toml`
- UI/daemon IPC: Unix domain socket carrying JSON messages
- Monitor mapping: stable Hyprland-derived `monitor_id` with output-name fallback
- Wayland bootstrap: `smithay-client-toolkit` layer-shell probe in `backlayer-wayland`
- MVP target: Hyprland only

## Current runtime status

- Hyprland monitor discovery is wired through `hyprctl monitors -j`
- The Wayland crate can connect to the compositor, bind `zwlr_layer_shell_v1`, create a background surface, and receive an initial configure
- The Wayland probe can bind a background layer surface to a specific output name such as `eDP-1`
- The Wayland crate now exposes a persistent background-surface session abstraction for a bound output
- The daemon exposes a Unix-socket JSON API at `~/.config/backlayer/backlayer.sock` in `--serve` mode
- The manager UI renders real monitors, assignments, assets, and daemon connection status when launched through Tauri
- The manager now distinguishes `daemon unavailable` from browser fallback mode and keeps polling so it can reconnect automatically after the daemon returns
- The manager can assign wallpapers per monitor and update pause/FPS policy through the daemon API, with config persistence
- The daemon now exposes a per-output runtime session plan so the UI can show whether assigned wallpapers are ready, unsupported, or failed
- The daemon now keeps a bounded recent runtime event log so the UI can show pause/resume and worker transition history without relying on terminal output
- In `--serve` mode, the daemon now keeps image and shader renderer workers alive per assigned output instead of only evaluating them during startup
- Live image and shader workers now attempt bounded automatic restarts after unexpected render/session failures, with restart events surfaced through the same runtime history path
- Shader sessions now require a valid on-disk WGSL asset before the daemon reports them as ready
- Shader runtime bootstrap now creates a `wgpu` device, compiles the assigned WGSL asset, binds the Wayland layer-shell surface, and submits a real shader frame during session startup
- In `--serve` mode, shader wallpapers now run in a dedicated `shader-runner` child process rather than inside `backlayerd`
- In `--serve` mode, imported video wallpapers now run in a dedicated `video-runner` child process while real playback is still pending
- In `--serve` mode, imported scene wallpapers now run in a dedicated `scene-runner` child process while real scene playback is still pending
- In `--serve` mode, imported web wallpapers now run in a dedicated `web-runner` child process while real web playback is still pending
- `scene-runner` now supports a narrow first subset for imported scenes: it can render the first local image referenced by `scene.json`, or the first local sibling image, before falling back to the imported preview image
- `scene-runner` can now also compose a minimal layered `scene.json` subset when it finds multiple local image layers with simple position, size/scale, and opacity fields
- `scene-runner` now understands basic `scene.pkg` bundles too: it can extract `scene.json`, follow simple model/material/texture references, decode common `.tex` payloads (including common BC1/DXT1 packaged textures and embedded image payloads), and composite those into a static scene result
- `web-runner` now supports a narrow first real subset for local HTML wallpapers: it can render the first local referenced image or a parsed background color before falling back to the imported preview image
- Shader assets can now opt into animation with `animated = true` in `backlayer.toml`
- The daemon can now import local Wallpaper Engine workshop item directories into `~/.config/backlayer/imports/wallpaper-engine`
- Imported Workshop items are classified as `video`, `scene`, or `web` assets and surfaced in the UI with compatibility badges and warnings
- The manager now suggests common Steam workshop roots automatically and uses imported preview images when available
- Imported Workshop items can now be reimported from their original source path through the inspector
- Imported Workshop items can now also be removed cleanly from Backlayer-managed storage through the inspector
- Workshop compatibility is disabled by default. Set `BACKLAYER_ENABLE_WORKSHOP=1` to show Workshop import/integration surfaces and load Backlayer-managed imported Workshop assets.
- The wallpaper browser now supports search plus source/type/compatibility filters, which makes larger imported libraries usable
- The wallpaper browser now uses a custom asset context menu for apply/create-scene/delete actions instead of relying on the native webview context menu
- Imported `video`, `scene`, and `web` items now fall back to their preview image as a static wallpaper when no real runtime exists yet
- That preview fallback now uses `video-runner` for `video`, `scene-runner` for `scene`, and `web-runner` for `web`
- The Settings modal now reports runtime dependency state for video so it is explicit when Backlayer is still in preview-fallback mode because `mpv`/`libmpv` is missing or playback integration is not finished
- Imported `scene` and `web` items are still compatibility-tracked items; real scene/web runtimes are not implemented yet
- The shader runner respects the configured `fps_limit`
- The shader runner also pauses frame submission when `pause_on_fullscreen` or `pause_on_battery` is enabled
- Image runtime bootstrap now decodes supported image assets, uploads them as GPU textures, binds the Wayland layer-shell surface, and submits a textured frame during session startup
- Image assets can now declare `image_fit = "cover" | "contain" | "stretch" | "center"` in `backlayer.toml`
- The manager now includes a full-screen Scene Composer for image wallpapers, which can generate native real-time `scene` assets and save them into `~/.config/backlayer/assets`
- Scene Composer is now a full-screen workspace opened from the main toolbar, and it can start from either a library image wallpaper or a picked local image file
- Scene Composer now uses a viewport-first editor layout with tabbed `Layers` / `Assets` / `Add` tools plus a progressive properties panel for the selected node
- Existing native scene assets can now be reopened and edited in the Scene Composer instead of only creating new scene assets
- Scene Composer now authors a native scene document with sprite, effect, and particle nodes instead of exporting generated overlay PNGs
- `scene-runner` now plays those native scenes as a real-time node graph with effects and particle emitters instead of a static layered composite
- Native scene playback now supports multi-image sprite sources plus a broader first preset set including fog effects and snow emitters
- Native scene emitters now support positioned origins, explicit shapes (`point`, `box`, `line`, `circle`), direction control, burst emission, lifetime/speed ranges, over-life curves, and particle tint in both the Scene Composer and native runtime
- Native scene effect nodes like glow, fog, vignette, and scanlines now support color tint in both the Scene Composer and native runtime
- Native scene rendering now uses a GPU-native sprite/effect pipeline in `scene-runner`, and particle drawing now goes through a GPU-native particle pass too
- Native scene playback is still an early engine slice; timeline/keyframe animation, particle image sprites, and stronger editor/runtime parity are still pending
- User-managed native assets under `~/.config/backlayer/assets` and imported Workshop assets can now be deleted from the browser context menu
- A `systemd --user` unit template is included for process-level crash recovery of `backlayerd`
- A standalone `animation-probe` binary now exists to isolate animated Wayland/`wgpu` rendering from the daemon while debugging animation crashes
