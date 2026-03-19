# MVP Architecture

Backlayer is organized around a daemon-owned wallpaper runtime.

## Main Pieces

### `backlayerd`

Owns:

- persisted config
- monitor discovery
- monitor-to-wallpaper assignment
- pause/FPS policy
- renderer process supervision
- IPC for the manager UI

`backlayerd --serve` is the normal long-running mode.

## Renderer Workers

Backlayer uses dedicated processes for persistent rendering paths:

- `shader-runner`
- `video-runner`
- `scene-runner`
- `web-runner`

Static image wallpapers still go through the daemon-owned image renderer path because they render once and stay idle.

## Wayland Layer

`crates/backlayer-wayland` provides the layer-shell session abstraction used by the runtime.

Current behavior:

- binds background surfaces to named outputs
- uses the background layer
- disables keyboard interactivity
- is intended to remain input-transparent

## UI

The manager UI is a Tauri + React app.

It is intentionally not the source of truth for wallpaper state.

The UI:

- asks the daemon for runtime state
- performs assignment and create/edit operations over IPC
- hosts the Scene Composer

## Runtime Flow

1. `backlayerd` loads config and discovers monitors/assets.
2. It builds a runtime plan from current assignments.
3. Persistent backends run in supervised worker processes.
4. The daemon periodically reconciles monitor state while serving.
5. The UI polls runtime state on a coarse cadence and refreshes assets only when needed.

## Current Scope Boundary

This architecture is intentionally scoped to:

- Hyprland
- wlroots layer-shell background surfaces
- native image, shader, video, and scene wallpapers

Broader compositor/Desktop Environment support is not part of the MVP hardening pass.
