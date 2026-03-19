# Changelog

All notable changes to Backlayer should be documented in this file.

The format is intentionally simple for now and follows an `Unreleased` section plus tagged versions.

## [Unreleased]

### Added

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

### Fixed

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
