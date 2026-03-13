# Backlayer Project Summary

## Vision

Backlayer is a Hyprland-first animated wallpaper runtime for Linux that aims to deliver the core feeling of Wallpaper Engine without expanding into unnecessary platform support or feature bloat too early.

The initial product is intentionally narrow:

- Target `Hyprland` only for the MVP
- Use `wlroots` layer-shell behavior as the rendering foundation
- Focus on wallpapers, not a general desktop customization suite
- Ship a stable runtime before building advanced UX, scripting, or marketplace features

## MVP Goal

Build a usable animated wallpaper system for Hyprland with:

- Background layer-shell surfaces per monitor
- Video wallpaper playback
- Shader-based wallpaper rendering
- Static image and slideshow wallpaper support
- Native scene asset creation for image-based animated compositions
- Native scene authoring now supports multiple image sources per scene plus sprite, effect, and emitter nodes
- A daemon that manages lifecycle, monitor mapping, and runtime rules
- A simple UI to configure wallpapers and performance behavior

If those pieces work reliably together, the MVP is successful.

## Core Product Shape

Backlayer should be structured as a wallpaper runtime made of five core areas:

1. `Layer-shell renderer`
   Creates background surfaces through Wayland layer-shell and attaches rendering output to specific monitors.
2. `Wallpaper runners`
   Separate renderer paths for video, shader, and image wallpapers.
3. `Daemon controller`
   Runs continuously, owns monitor-to-wallpaper state, manages renderer processes, and applies runtime rules.
4. `Hyprland integration`
   Detects outputs and reacts to monitor changes using `hyprctl` first and, later, Hyprland socket events.
5. `Configuration UI`
   A lightweight interface for selecting wallpapers, assigning them to monitors, and setting pause/fps behavior.

In parallel with the runtime, Backlayer can grow a native content path:

6. `Native scene creation`
   A Scene Composer and native scene engine that turn static images into Backlayer-native `scene` assets with a real-time 2D scene graph, effects, and particle systems.

## Architecture Direction

The product should behave like this:

- A user configures wallpapers and rules in the UI
- The daemon stores config and watches monitor state
- The daemon reconciles monitor state during runtime so output changes do not require a full restart
- The daemon creates wallpaper surfaces for each active output
- Renderer backends draw into those surfaces
- The system pauses or throttles rendering based on power/performance rules

At a high level:

```text
backlayer
├─ daemon
├─ renderers
│  ├─ video
│  ├─ shader
│  ├─ image
│  └─ scene
├─ compositor adapter
│  └─ layer-shell / Wayland
└─ UI
   └─ scene composer
```

## Technical Direction

### Recommended stack

- `Rust` for daemon, Wayland integration, and rendering runtime
- `smithay-client-toolkit` and `wayland-client` for Wayland/layer-shell integration
- `wgpu` for shader rendering and modern GPU-backed drawing
- `libmpv` for video wallpaper playback
- `Tauri + React` for a lightweight desktop UI

### Initial repo decisions

- Rust workspace at the repo root with separate crates for config, types, Hyprland integration, Wayland integration, and renderer backends
- Manager UI in `apps/ui` with a Tauri host in `apps/ui/src-tauri`
- Config file path target: `~/.config/backlayer/config.toml`
- IPC direction: Unix domain socket with JSON messages between the UI and daemon
- Wallpaper asset metadata direction: per-asset `backlayer.toml`
- Wayland bootstrap direction: `smithay-client-toolkit` layer-shell client that already proves background-surface creation on Hyprland
- Output targeting direction: layer-shell surfaces can already be bound to a named monitor output before renderer work begins
- Session direction: the Wayland layer now has a persistent background-surface session abstraction that can be pumped over time
- Daemon control direction: `backlayerd --serve` exposes the initial runtime state over a local Unix socket
- Daemon runtime direction: `backlayerd --serve` now spawns and maintains image/shader renderer workers per assigned output
- UI control direction: the manager already supports persisted monitor assignment and pause-policy writes; renderer playback is the next missing runtime layer
- Runtime observability direction: the shared state now includes per-output renderer session status so failures and unsupported backends are visible in the UI
- Asset validation direction: renderer backends now validate assigned assets before sessions are reported as ready, tightening the path toward real playback
- Shader runtime direction: shader sessions can now create a `wgpu` device, compile an external WGSL asset, bind a Wayland layer-shell surface, and submit a real shader frame
- Image runtime direction: image sessions can now decode supported image assets, upload them to the GPU, bind a Wayland layer-shell surface, and submit a real textured frame
- Native scene direction: the manager can now create Backlayer-native `scene` assets from existing image wallpapers by saving a native scene document with sprite, effect, and particle nodes into user-managed asset storage under `~/.config/backlayer/assets`
- Native scene runtime direction: `scene-runner` now treats native Backlayer scenes as a real-time scene graph instead of a stack of pre-rendered overlay images, with GPU-native sprite/effect rendering plus a GPU-native particle pass, and native emitters now support positioned origins, explicit shapes, burst/range controls, over-life curves, direction control, and tint

### Why this scope works

- `Hyprland` gives a clear compositor target
- `wlroots` and layer-shell remove the hardest desktop-placement problem
- `libmpv` avoids reinventing video playback
- `wgpu` makes shader wallpapers viable without committing to a fragile custom GL stack
- A daemon model keeps the system resilient and easier to autostart via `systemd --user`

## MVP Requirements

The MVP should support:

- Per-monitor wallpapers
- Video wallpapers
- Shader wallpapers
- Image wallpapers
- Native scene assets created inside Backlayer
- Input passthrough on wallpaper surfaces
- FPS limiting
- Pause on fullscreen
- Autostart on login

## Explicit Non-Goals For MVP

These should stay out of scope until the runtime is stable:

- Supporting all Linux desktop environments
- KDE / GNOME / X11 compatibility
- Workshop / marketplace integration
- Wallpaper scripting engines
- Full creator-grade editor tooling
- Community content platform features
- Plugin systems

The current exception is a narrow native Scene Composer:

- creating native `scene` assets from static images with real-time nodes, effects, and particle systems is in scope
- full timeline editing, keyframes, scripting, and advanced creator workflows are not

## Quality Bar

Backlayer should feel like a deliberate runtime, not a media player embedded behind windows. That means:

- Surfaces must sit correctly in the background layer
- Input must always pass through
- Multi-monitor behavior must be predictable
- Performance controls must exist from the beginning
- Crash recovery and restart behavior must be owned by the daemon

## Success Criteria

The first version is good enough when a Hyprland user can:

- install Backlayer
- assign different wallpapers to different monitors
- use video, shader, or image wallpapers
- keep idle performance reasonable
- trust it not to interfere with normal desktop input
- leave it running through normal day-to-day usage
