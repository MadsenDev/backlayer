# Backlayer TODO

This file is the source of truth for execution status. It is organized as a
milestone-driven roadmap: work the milestones top to bottom, check items off in
the same changeset that completes them, and keep the archive at the bottom
intact for history.

Roadmap intent: ship a first public release for the Hyprland MVP, make the
performance story provable, then expand to KDE Plasma where most
gaming-distro users land. Scene Composer ↔ `scene-runner` visual parity is a
headline focus for the release, not a polish item.

## Milestone 1 — v0.3.0: First public release (Hyprland MVP)

Goal: a Hyprland user can install Backlayer from the AUR, assign video,
shader, image, and native scene wallpapers per monitor, trust input
passthrough and multi-monitor behavior, and see the same scene on the
wallpaper that they authored in the Scene Composer.

### Correctness blockers

- [ ] Verify input passthrough behavior on wallpaper surfaces
- [ ] Validate correct behavior across multi-monitor layouts
- [ ] Handle monitor hotplug and removal cleanly
- [ ] Handle video renderer restart on failure

### Compositor fallback

- [x] Add a generic Wayland layer-shell compositor fallback (Niri, Sway,
      river, ...) with Wayland-native monitor discovery so the daemon runs
      outside Hyprland/KDE — needed since the primary dev machine no longer
      runs Hyprland
- [x] Fail fast with a clear "Wayland session required" error on unknown
      environments (X11, TTY) instead of silently defaulting to the Hyprland
      client
- [ ] Manually verify the generic fallback end-to-end on Niri (discovery,
      assignment, layer-shell rendering, input passthrough)

### Diagnostics and CLI

- [x] Reject unrecognized `backlayerd` arguments with usage output instead of
      silently falling back to one-shot probe mode; add `--help`/`--version`
- [x] Add `backlayerctl doctor` (with `--json`) reporting session/compositor
      detection, config load status, daemon socket health, monitors, runtime
      sessions, recent renderer events, video dependencies, and discovered
      assets

### Scene parity: Composer preview ↔ `scene-runner` 1:1

The composer preview is a Canvas2D implementation in the UI while
`scene-runner` renders through a wgpu/WGSL pipeline — two independent
implementations of the same scene semantics. Parity drift is structural, so
this track both fixes today's divergences and moves toward a single source of
truth for scene evaluation.

- [x] Audit and document every visual divergence between the Canvas2D
      composer preview and the wgpu `scene-runner` (sprites, effects,
      emitters, blending, color, timing) — see `docs/scene-parity-audit.md`
- [x] Fix runtime bugs surfaced by the parity audit: sprite `rotation_deg`
      ignored by the sprite pipeline, rain occlusion segments built along the
      wrong axis, sRGB colors uploaded as linear, the `max_life = 0` NaN, and
      the inverted rain streak feather
- [x] Fix preview bugs surfaced by the parity audit: blend modes and additive
      particle compositing never applied, particles drawn in node order
      instead of on top, hardcoded glow gradient outer color, glow/vignette
      falloff curves, and behavior constants that drifted from the runtime's
- [x] Write a single scene-semantics spec both renderers implement:
      coordinate space, units, time base, curve interpolation, and blend
      modes per node type — see `docs/scene-semantics-spec.md`
- [x] Match particle simulation behavior between preview and runtime: spawn
      rates, lifetime/speed ranges, direction/gravity, burst timing, emitter
      shapes, and over-life size/alpha/color curves — the preview now runs
      the same stateful simulation as `scene-runner`, and the runtime adopts
      the composer's default curves and curve sanitation
- [ ] Match effect rendering between preview and runtime: glow, fog, tint,
      and additive vs alpha blending
- [x] Match coordinate space, scaling, and aspect handling between the
      preview viewport and the output surface — pixel-unit fields are now
      document-space and both renderers scale them by the canvas/document
      long-edge ratio; the runtime also adopted the preview's wall-clock
      particle time base and parameter clamps
- [x] Match particle occluder/landing-surface behavior between preview and
      runtime, including custom drawn regions and polygon particle areas —
      landing is now permanent with the runtime's crossing detection and
      slide, and blocker rects are evaluated at time 0 on both sides
- [ ] Evaluate replacing the Canvas2D preview with a shared GPU path (WebGPU
      port of the runner pipeline, or preview frames rendered by the native
      scene engine) so parity holds by construction instead of by discipline
- [ ] Add particle sprite/image support for native scene emitters — landed
      in the composer and `scene-runner` in the same changeset
- [ ] Add a side-by-side preview-vs-runtime checklist to the manual
      verification matrix

### Video performance

- [ ] Prove hardware-accelerated playback path (VA-API via FFmpeg is
      acceptable for v0.3.0)
- [ ] Integrate `libmpv`, or explicitly defer it to v0.4 in the release notes

### Release testing

- [ ] Test fullscreen pause logic
- [ ] Test multi-monitor assignment behavior
- [ ] Test native scene animation behavior manually
- [ ] Test the redesigned Scene Composer flow manually
- [ ] Test renderer crash recovery behavior manually

### Release mechanics

- [ ] Decide MVP version scope freeze
- [ ] Verify all must-have MVP items are complete
- [ ] Prepare demo assets for screenshots/videos
- [ ] Write first release notes
- [ ] Publish the AUR packages produced by the release automation
- [ ] Add compositor-level troubleshooting notes for unsupported desktop
      environments (for example KDE showing static fallback previews)

## Milestone 2 — v0.3.x: Provable performance story

Goal: documented proof that Backlayer gets out of the way while gaming — the
core trust argument for the Linux gaming audience.

- [ ] Benchmark CPU/GPU usage for each renderer type and publish the numbers
      in the README
- [ ] Add idle/resource policy for hidden or inactive outputs
- [ ] Add structured logging
- [ ] Investigate Hyprland socket event integration (replace polling-based
      monitor refresh)
- [ ] Add integration testing strategy for Hyprland environments

## Milestone 3 — v0.4.0: KDE Plasma bridge

Goal: reach the desktop most gaming distros (Bazzite, Nobara, CachyOS
defaults, Steam Deck desktop mode) actually ship. Stays out of scope until
Milestone 1 ships.

- [ ] Add a daemon status bridge for the Plasma wallpaper plugin via IPC or a
      thin `backlayerctl --json` helper
- [ ] Add Plasma containment/screen to Backlayer monitor mapping strategy and
      diagnostics
- [ ] Prototype a live-render bridge path for Plasma wallpaper surfaces
      without changing the Hyprland layer-shell path

## Backlog (unscheduled)

- [ ] Add fullscreen detection for non-Hyprland Wayland compositors
      (zwlr-foreign-toplevel or compositor IPC such as `niri msg`) so
      pause-on-fullscreen works on the generic fallback path
- [ ] Add slideshow support
- [ ] Add GIF support or explicitly defer it
- [ ] Render a simple built-in demo shader
- [ ] Expose basic shader parameters if needed
- [ ] Add real `web` runtime support

## Completed (archive)

### Foundation

- [x] Define final workspace layout for daemon, renderers, compositor adapter, and UI
- [x] Choose crate/package structure for Rust core and Tauri frontend
- [x] Decide config file format and storage location
- [x] Define wallpaper asset model and metadata format
- [x] Add a native `.backlayer` wallpaper package format
- [x] Define IPC contract between UI and daemon
- [x] Add shared IPC request/response types
- [x] Add sample asset directory structure to validate metadata decisions

### Wayland / Layer-Shell

- [x] Set up a minimal Wayland client using `smithay-client-toolkit`
- [x] Create a background layer-shell surface on Hyprland
- [x] Bind surfaces to specific outputs/monitors

### Renderer: Image

- [x] Implement static image wallpaper rendering
- [x] Add scaling and positioning modes

### Native Scene Composer

- [x] Define a native Backlayer `scene` asset format distinct from Workshop imports
- [x] Replace the old overlay-export scene format with a real-time native scene document
- [x] Save native scene assets into Backlayer-managed local storage
- [x] Surface a Scene Composer flow in the UI for image wallpapers
- [x] Add loading and save feedback to the Scene Composer flow
- [x] Make Scene Composer a full-screen workspace with a global entry point
- [x] Let Scene Composer start from either a library image or a picked local image file
- [x] Allow reopening and editing existing native scene assets in the Scene Composer
- [x] Replace the preset checklist composer with a node-based scene editor
- [x] Redesign the Scene Composer into a viewport-first editor with tabbed side tools and progressive properties
- [x] Add unified viewport direct-manipulation tools and handles for sprites, emitters, and particle areas
- [x] Add real-time native scene playback for sprite, effect, and particle nodes
- [x] Add a live editor preview based on the new scene graph instead of CSS overlays
- [x] Add layer reordering and removal in the Scene Composer UI
- [x] Add multi-image sprite sources inside the Scene Composer
- [x] Expand native scene presets beyond the initial effect/emitter set
- [x] Move native scene rendering from CPU compositing to a GPU-native sprite/effect pipeline
- [x] Move native scene particle rendering from CPU texture uploads to a GPU-native particle pipeline
- [x] Add positioned, directed, and tinted particle emitters to the Scene Composer and runtime
- [x] Add color/tint controls for native scene effect nodes like glow and fog
- [x] Add explicit emitter shapes and region controls for native particle nodes
- [x] Add burst emission plus lifetime and speed ranges for native particle nodes
- [x] Add over-life size, alpha, and color curves for native particle nodes
- [x] Move advanced particle curve editing into a dedicated particle editor workflow
- [x] Add sprite-based particle occluders and landing surfaces for native scenes
- [x] Add viewport-drawn custom particle collider regions for sprite nodes
- [x] Add standalone particle area nodes for scene-level occlusion and landing regions
- [x] Add polygon support for standalone particle area nodes

### Renderer: Video

- [x] Add local Wallpaper Engine video item import/classification
- [x] Route current video preview fallback through a dedicated runner process
- [x] Add a first-pass FFmpeg-decoded video playback path in `video-runner`
- [x] Respect FPS limiter and pause rules inside `video-runner`
- [x] Render video wallpapers into layer-shell surfaces
- [x] Add looping behavior for daemon-managed video playback

### Renderer: Shader

- [x] Set up `wgpu` rendering pipeline
- [x] Define shader wallpaper asset format
- [x] Load external shader assets
- [x] Support animated shader assets
- [x] Add an isolated animation probe outside the daemon for crash debugging

### Workshop Compatibility

- [x] Extend asset metadata to track import source, compatibility status, and warnings
- [x] Import local Wallpaper Engine workshop item folders into Backlayer-managed storage
- [x] Classify imported items as `video`, `scene`, or `web`
- [x] Surface imported items and compatibility warnings in the UI
- [x] Add static preview-image fallback for imported `video`, `scene`, and `web` items
- [x] Route current scene preview fallback through a dedicated runner process
- [x] Route current web preview fallback through a dedicated runner process
- [x] Add minimal local HTML extraction support in `web-runner`
- [x] Add minimal scene image extraction support in `scene-runner`
- [x] Add layered `scene.json` image composition support in `scene-runner`
- [x] Add minimal `scene.pkg` extraction and layered composition support in `scene-runner`
- [x] Add heuristic `.tex` texture decoding for common packaged `scene.pkg` scenes
- [x] Re-import/update existing workshop items from their original source path
- [x] Remove imported workshop items cleanly from managed storage
- [x] Add real `scene` runtime support for native Backlayer scenes

### Daemon

- [x] Create long-running daemon process
- [x] Load and persist wallpaper configuration
- [x] Map wallpapers to monitors
- [x] Spawn renderer instances per output
- [x] Restart crashed renderers safely
- [x] Expose IPC/API for the UI
- [x] Reduce daemon request-time monitor refresh churn

### Hyprland Integration

- [x] Parse `hyprctl monitors`
- [x] Add monitor identity mapping that survives common layout changes
- [x] Add polling-based monitor refresh in daemon `--serve` mode
- [x] React to monitor changes without requiring daemon restart

### Performance / Power

- [x] Add FPS limiter
- [x] Add pause-on-fullscreen logic
- [x] Add pause-on-battery behavior or defer explicitly
- [x] Reduce idle wakeups in shader, video, and scene runners
- [x] Decouple asset refresh from steady-state UI polling

### UI

- [x] Scaffold a minimal `Tauri + React` manager app
- [x] Display detected monitors and current wallpaper assignments
- [x] Allow selecting wallpaper assets
- [x] Add search/filter controls suitable for larger imported wallpaper libraries
- [x] Allow assigning wallpapers per monitor
- [x] Add controls for FPS and pause rules
- [x] Add status/error surface for daemon and renderer failures
- [x] Add recent runtime event/history surface for pause/resume and worker transitions
- [x] Stabilize viewport-locked internal scrolling for browser and inspector panes
- [x] Use real image files for native image wallpaper card previews
- [x] Add system-following light/dark theme with native desktop styling
- [x] Redesign wallpaper browser cards for a more consistent library look
- [x] Add a proper startup loading screen before the first runtime snapshot resolves
- [x] Add a custom asset context menu in the wallpaper browser
- [x] Allow deleting user-managed native wallpapers and imported wallpapers from the UI
- [x] Move heavy Tauri UI commands off the main thread to reduce interaction freezes
- [x] Show a startup splash before the main React app loads
- [x] Add a unified Create picker for still image, scene, shader, and video wallpapers
- [x] Make Scene Composer editing open immediately with progressive scene hydration
- [x] Reduce Scene Composer and browser preview work to improve interaction responsiveness

### KDE Plasma Integration

- [x] Add a minimal Plasma 6 wallpaper plugin package that appears as `Backlayer` and renders an animated QML placeholder

### Packaging / Startup

- [x] Add `systemd --user` service for daemon autostart
- [x] Define install flow for runtime, UI, and assets
- [x] Document runtime dependencies like `mpv`/`libmpv`
- [x] Add an Arch/CachyOS package layout for the UI, daemon, runners, assets, Plasma plugin, and user service
- [x] Add GitHub release artifact automation for Arch packages and AUR-ready package files

### Testing

- [x] Add unit tests for config parsing and monitor mapping
- [x] Add smoke test for daemon startup
- [x] Test animated shader assignment behavior manually
- [x] Document a manual verification matrix for the current MVP

### Documentation

- [x] Write local development setup instructions
- [x] Document MVP architecture
- [x] Document configuration format
- [x] Document `systemd --user` supervision for daemon crash recovery
- [x] Document known limitations of the Hyprland-only MVP
- [x] Keep `PROJECT_SUMMARY.md` aligned with implementation reality
