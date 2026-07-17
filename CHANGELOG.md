# Changelog

All notable changes to Backlayer should be documented in this file.

The format is intentionally simple for now and follows an `Unreleased` section plus tagged versions.

## [Unreleased]

### Added

- Scene-semantics spec (`docs/scene-semantics-spec.md`): the single normative definition of coordinate space, pixel-unit scaling, time base, curve interpolation/sanitation, blend modes, color handling, draw order, and occluder behavior that both the Scene Composer preview and `scene-runner` implement

- `backlayerctl doctor` diagnostic command (with `--json` output) that checks the session/compositor environment, config load status, daemon socket health, monitors, runtime sessions, recent renderer events, video dependencies, and discovered assets
- `backlayerd --help` and `--version` flags with proper usage output

- Project landing page for GitHub Pages under `site/` (self-contained static HTML) with a `Deploy GitHub Pages` workflow that publishes it from `main`
- Generic Wayland layer-shell compositor fallback: on any non-Hyprland, non-KDE Wayland session (Niri, Sway, river, ...) the daemon now uses Wayland-native monitor discovery (`wl:` monitor-id prefix) with the standard layer-shell rendering path; fullscreen detection is not available on this path yet
- Scene parity audit (`docs/scene-parity-audit.md`) documenting every known visual divergence between the Scene Composer preview and `scene-runner`, including newly found runtime bugs (sprite rotation ignored, rain occlusion axis, sRGB color handling) and preview bugs (missing blend modes, particle draw order)
- KDE Plasma 6 wallpaper bridge foundation under `integrations/kde-plasma` with a real Plasma wallpaper plugin package, install script, and animated placeholder `main.qml`
- Arch/CachyOS packaging helpers, Arch post-install guidance, and an AUR-ready packaging layout for both release and `-git` package tracks
- Built-in `demo.prism-loop` native video asset for end-to-end video playback testing
- Unified Create flow in the manager for native image, scene, shader, and video wallpapers
- Release-facing docs for install flow, architecture, known limitations, and manual verification
- Native `.backlayer` single-file package support for Backlayer-created wallpapers
- Sprite-based particle occluders and landing surfaces for native scenes
- Viewport-drawn custom particle collider regions for sprite nodes in the Scene Composer
- Standalone particle area nodes for scene-level occlusion and landing regions
- Polygon support for standalone particle area nodes
- Unified viewport manipulation tools and direct handles for sprite, emitter, and particle-area nodes in the Scene Composer

### Changed

- Scene pixel-unit fields (sprite offsets, drift/orbit amplitudes, emitter size/speed/gravity) are now interpreted in document space and scaled by the canvas/document long-edge ratio in both the composer preview and `scene-runner`, so scenes keep the authored composition at any monitor resolution instead of rendering particles and offsets relatively smaller on higher-resolution outputs
- `scene-runner` now advances its particle simulation with the real wall-clock frame delta (clamped to 100 ms) like the composer preview, instead of a fixed nominal step per rendered frame that slowed particles down under dropped frames
- `scene-runner` now clamps emitter region/line parameters and minimum speed exactly like the composer preview, and an unset line angle now follows the emitter's custom direction instead of the preset default
- `backlayerd` now fails fast with a clear "Wayland session required" error on unknown environments (X11, TTY, KDE on X11) instead of silently defaulting to the Hyprland client
- `backlayerd` now rejects unrecognized command-line arguments with usage output instead of silently running the one-shot probe mode
- Scene Composer preview particles now run the same stateful simulation as `scene-runner` — burst emission, warm start, per-frame gravity/drag integration, and permanent surface landing now behave in the editor exactly as they do on the wallpaper
- `scene-runner` now applies the composer's per-preset default size/alpha/color curves and curve sanitation when a scene document omits or misorders them
- Product docs now carry a milestone-based roadmap targeting a first public v0.3.0 release, with Scene Composer ↔ `scene-runner` visual parity elevated to a headline focus and KDE Plasma bridge work sequenced after the Hyprland MVP ships
- Built-in asset discovery now works for installed package layouts such as `/usr/share/backlayer/assets`, not only workspace checkouts
- KDE Plasma bridge foundation now uses the wallpaper-specific QML import module and safer shell-restart fallback ordering in the installer docs/script
- Product docs now track the explicitly requested narrow KDE Plasma plugin bridge exception while preserving Hyprland-first runtime scope
- `video-runner` now respects the daemon FPS cap plus pause-on-fullscreen and pause-on-battery policy
- daemon/UI steady-state refresh work is now much lighter, with reduced asset polling and lower idle wakeups in shader/video/scene runners
- `backlayerd --serve` now reconciles monitor changes in the background instead of waiting for UI request traffic
- Backlayer-native create/edit/discovery flows now prefer `.backlayer` packages over plain asset folders
- Native scene editing now opens the composer shell immediately and hydrates scene images from cached file paths instead of eagerly base64-encoding the full scene payload
- Scene saving now reuses unchanged scene image files instead of re-uploading every image during edits
- Scene Composer preview work is now capped to a lower internal resolution with a steadier preview loop, and browser card previews now lazy-load only when visible
- Scene Composer sprites can now hide particles behind foreground art or act as landing surfaces for snow and dust
- Scene Composer particle occluder/surface sprites can now use a custom drawn region instead of full sprite bounds
- Scene Composer can now place occluder and landing regions directly on the scene without needing a sprite node
- Standalone particle areas can now be authored as polygons for more useful collider and occluder shapes
- Known limitations docs now explicitly call out KDE Plasma's tendency to keep wallpaper ownership and show only static fallback previews outside the Hyprland MVP scope

### Fixed

- Native scene sprites now honor `rotation_deg` on the wallpaper, matching the Scene Composer preview
- Native scene rain occlusion segments now follow the streak's actual long axis, so rain hides behind occluders where the preview shows it
- Native scene effect and particle colors are now srgb-decoded before GPU upload, so wallpaper colors match the authored hex instead of rendering brighter
- Native scene rain streaks now render with an opaque center and feathered edges instead of an inverted (hollow-centered) feather
- Native scene emitter `min_life` now floors at 0.2s like the composer preview, removing a `max_life = 0` NaN
- Scene Composer preview now draws particles on top of all sprites and effects, composites them additively with the runtime's radial feather, honors sprite blend modes, matches the runtime's glow/vignette falloff curves, no longer tints custom glow colors toward the default, and uses the runtime's sprite behavior constants
- Native scene particle occlusion now uses the rendered particle footprint more closely, so rain and other stretched particles hide behind occluders much more reliably

## [0.2.0] - 2026-03-13

### Added

- Native scene composer with sprite, effect, and particle node authoring
- GPU-native scene playback through `scene-runner`
- Advanced native particle controls:
  - emitter shapes
  - burst emission
  - speed and lifetime ranges
  - over-life size, alpha, and color curves
- Dedicated particle editor workflow in the Scene Composer
- First-pass video playback in `video-runner` using FFmpeg decode + GPU surface rendering
- Light/dark theme support in the manager UI
- Custom asset context menu and native asset deletion from the UI

### Changed

- README rewritten into a release-facing project overview
- Workshop compatibility is now opt-in via `BACKLAYER_ENABLE_WORKSHOP=1`
- Project version bumped to `0.2.0`

### Fixed

- Multiple Scene Composer preview/runtime mismatches around emitters and effects
- Several blocking UI interactions by moving heavy Tauri commands off the main thread
