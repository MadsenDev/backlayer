# Backlayer TODO

## Foundation

- [x] Define final workspace layout for daemon, renderers, compositor adapter, and UI
- [x] Choose crate/package structure for Rust core and Tauri frontend
- [x] Decide config file format and storage location
- [x] Define wallpaper asset model and metadata format
- [x] Define IPC contract between UI and daemon
- [x] Add shared IPC request/response types
- [x] Add sample asset directory structure to validate metadata decisions

## Wayland / Layer-Shell

- [x] Set up a minimal Wayland client using `smithay-client-toolkit`
- [x] Create a background layer-shell surface on Hyprland
- [ ] Verify input passthrough behavior
- [x] Bind surfaces to specific outputs/monitors
- [ ] Handle monitor hotplug and removal cleanly
- [ ] Validate correct behavior across multi-monitor layouts

## Renderer: Image

- [x] Implement static image wallpaper rendering
- [x] Add scaling and positioning modes
- [ ] Add slideshow support
- [ ] Add GIF support or explicitly defer it

## Native Scene Composer

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
- [x] Add real-time native scene playback for sprite, effect, and particle nodes
- [x] Add a live editor preview based on the new scene graph instead of CSS overlays
- [x] Add layer reordering and removal in the Scene Composer UI
- [x] Add multi-image sprite sources inside the Scene Composer
- [x] Expand native scene presets beyond the initial effect/emitter set
- [x] Move native scene rendering from CPU compositing to a GPU-native sprite/effect pipeline
- [x] Move native scene particle rendering from CPU texture uploads to a GPU-native particle pipeline
- [x] Add positioned, directed, and tinted particle emitters to the Scene Composer and runtime
- [x] Add color/tint controls for native scene effect nodes like glow and fog
- [ ] Add particle sprite/image support for native scene emitters
- [x] Add explicit emitter shapes and region controls for native particle nodes
- [x] Add burst emission plus lifetime and speed ranges for native particle nodes
- [x] Add over-life size, alpha, and color curves for native particle nodes
- [x] Move advanced particle curve editing into a dedicated particle editor workflow
- [ ] Tighten visual parity between the Scene Composer preview and `scene-runner`

## Renderer: Video

- [x] Add local Wallpaper Engine video item import/classification
- [x] Route current video preview fallback through a dedicated runner process
- [ ] Integrate `libmpv`
- [ ] Prove hardware-accelerated playback path
- [ ] Render video wallpapers into layer-shell surfaces
- [ ] Add looping behavior and playback controls needed by the daemon
- [ ] Handle renderer restart on failure

## Renderer: Shader

- [x] Set up `wgpu` rendering pipeline
- [x] Define shader wallpaper asset format
- [ ] Render a simple built-in demo shader
- [x] Load external shader assets
- [x] Support animated shader assets
- [ ] Expose basic shader parameters if needed
- [x] Add an isolated animation probe outside the daemon for crash debugging

## Workshop Compatibility

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
- [ ] Add real `web` runtime support

## Daemon

- [x] Create long-running daemon process
- [x] Load and persist wallpaper configuration
- [x] Map wallpapers to monitors
- [x] Spawn renderer instances per output
- [x] Restart crashed renderers safely
- [x] Expose IPC/API for the UI
- [ ] Add structured logging

## Hyprland Integration

- [x] Parse `hyprctl monitors`
- [x] Add monitor identity mapping that survives common layout changes
- [x] Add polling-based monitor refresh in daemon `--serve` mode
- [ ] Investigate Hyprland socket event integration
- [ ] React to monitor changes without requiring daemon restart

## Performance / Power

- [x] Add FPS limiter
- [x] Add pause-on-fullscreen logic
- [x] Add pause-on-battery behavior or defer explicitly
- [ ] Add idle/resource policy for hidden or inactive outputs
- [ ] Benchmark CPU/GPU usage for each renderer type

## UI

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

## Packaging / Startup

- [x] Add `systemd --user` service for daemon autostart
- [ ] Define install flow for runtime, UI, and assets
- [x] Document runtime dependencies like `mpv`/`libmpv`

## Testing

- [x] Add unit tests for config parsing and monitor mapping
- [ ] Add integration testing strategy for Hyprland environments
- [ ] Add smoke test for daemon startup
- [ ] Test fullscreen pause logic
- [ ] Test multi-monitor assignment behavior
- [ ] Test native scene animation behavior manually
- [ ] Test the redesigned Scene Composer flow manually
- [x] Test animated shader assignment behavior manually
- [ ] Test renderer crash recovery behavior manually

## Documentation

- [x] Write local development setup instructions
- [ ] Document MVP architecture
- [x] Document configuration format
- [x] Document `systemd --user` supervision for daemon crash recovery
- [ ] Document known limitations of the Hyprland-only MVP
- [x] Keep `PROJECT_SUMMARY.md` aligned with implementation reality

## Release Readiness

- [ ] Decide MVP version scope freeze
- [ ] Verify all must-have MVP items are complete
- [ ] Prepare demo assets for screenshots/videos
- [ ] Write first release notes
