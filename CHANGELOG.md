# Changelog

All notable changes to Backlayer should be documented in this file.

The format is intentionally simple for now and follows an `Unreleased` section plus tagged versions.

## [Unreleased]

### Added

- Native scene composer with sprite, effect, and particle node authoring
- GPU-native scene playback through `scene-runner`
- Advanced native particle controls:
  - emitter shapes
  - burst emission
  - speed and lifetime ranges
  - over-life size, alpha, and color curves
- Dedicated particle editor workflow in the Scene Composer
- Light/dark theme support in the manager UI
- Custom asset context menu and native asset deletion from the UI

### Changed

- README rewritten into a release-facing project overview
- Workshop compatibility is now opt-in via `BACKLAYER_ENABLE_WORKSHOP=1`

### Fixed

- Multiple Scene Composer preview/runtime mismatches around emitters and effects
- Several blocking UI interactions by moving heavy Tauri commands off the main thread
