# Configuration

Backlayer stores runtime configuration in:

`~/.config/backlayer/config.toml`

The current format is TOML and is intentionally small for the MVP.

## Example

```toml
[[assignments]]
monitor_id = "hypr:chimei-innolux-corporation:0x14c9:chimei-innolux-corporation-0x14c9"

[assignments.wallpaper]
id = "demo.neon-grid"
name = "Neon Grid"
kind = "shader"
animated = false
entrypoint = "assets/demo.neon-grid/shaders/neon-grid.wgsl"

[pause]
pause_on_fullscreen = true
pause_on_battery = true
fps_limit = 30

[ipc]
kind = "unix_socket"
path = "~/.config/backlayer/backlayer.sock"
```

## Notes

- `assignments` maps a stable monitor identifier to a wallpaper asset.
- `monitor_id` is derived from Hyprland monitor metadata and is intended to survive output-name changes better than `eDP-1` or `DP-1`.
- `kind` currently supports `image`, `video`, `shader`, `scene`, and `web`.
- `animated` currently matters for shader assets. Set it to `true` when the WGSL source expects the built-in uniform block exposed by Backlayer.
- `image_fit` currently matters for image assets. Supported values are `cover`, `contain`, `stretch`, and `center`.
- `entrypoint` is the asset-relative runtime target, such as a shader file or media file.
- `source_kind` distinguishes native Backlayer assets from imported Wallpaper Engine items.
- `compatibility.status` is one of `supported`, `partial`, or `unsupported`, and `compatibility.warnings` explains known fidelity gaps.
- Imported Wallpaper Engine items may also include `preview_image` and `import_metadata` describing the original local source path, workshop id, and source manifest.
- Native scene assets created inside Backlayer are stored under `~/.config/backlayer/assets` and discovered alongside repo-local demo assets.
- `ipc` currently assumes a Unix domain socket for UI-to-daemon communication.
- The daemon can still match legacy output-name assignments as a fallback, but new config should use `monitor_id`.
- The repo currently includes `demo.neon-grid` and `demo.ember-scan` for static shaders, `demo.tide-pulse` for an animated shader, and `demo.sunset-stripes` for the image path.
- The repo currently uses `demo.sunset-stripes` as the single image demo; image fit is now configured per assignment from the UI instead of through duplicate demo assets.
- The manager can now create native `scene` assets through the Scene Composer UI. Scene Composer can start from either an existing image wallpaper in the library or a picked local image file.
- Native scene assets created inside Backlayer can also be reopened and edited in the Scene Composer.
- Native Backlayer scenes now use a `backlayer_scene_v2` document in `scene.json`, with image sources plus ordered sprite, effect, and particle-emitter nodes.
- `scene-runner` now plays those native scenes as a real-time node graph instead of a stack of exported overlay images.
- Native scene playback now supports multi-image sprite sources plus a broader first preset set, including fog effects and snow emitters.
- Native scene emitters now support positioned origins, explicit emitter shapes (`point`, `box`, `line`, `circle`), burst emission, lifetime/speed ranges, over-life size/alpha/color curves, direction control, and particle tint in both the Scene Composer and `scene-runner`.
- Native scene effect nodes now support color tint too, so glow/fog/vignette/scanlines can be recolored from the Scene Composer and retain that tint at runtime.
- Native scene rendering now uses a GPU-native sprite/effect path in `scene-runner`, and particle drawing now goes through a GPU-native particle pass too.
- Native scene playback is still an early engine slice: full timeline/keyframe animation, particle image sprites, and tighter editor/runtime parity do not exist yet.
- The daemon can import local Wallpaper Engine workshop item folders into `~/.config/backlayer/imports/wallpaper-engine` through the UI `Import` flow or the `import_workshop_path` IPC request.
- Imported Workshop items can be refreshed from their original local source path through the inspector `Reimport from source` action or the `reimport_asset` IPC request.
- Imported Workshop items can be removed from Backlayer-managed storage through the inspector `Remove import` action or the `remove_asset` IPC request.
- Workshop compatibility is disabled by default. Set `BACKLAYER_ENABLE_WORKSHOP=1` to enable Workshop import suggestions/actions and load imported Workshop assets from managed storage.
- The UI also suggests common Steam workshop roots automatically when it detects them, including standard Steam and Flatpak Steam locations for app `431960` (Wallpaper Engine).
- Current import classification recognizes common Wallpaper Engine-style `project.json` metadata plus heuristic detection for `scene.pkg`, `index.html`, and common video file extensions.
- Imported `video`, `scene`, and `web` items are preserved and surfaced in the asset browser.
- When one of those imported items includes a preview image, Backlayer now uses that preview as a static wallpaper fallback until a real runtime exists.
- Imported `video` items currently hold that preview through a dedicated `video-runner` child process in `--serve` mode, imported `scene` items do the same through `scene-runner`, and imported `web` items do the same through `web-runner`.
- `scene-runner` currently supports a narrow local subset: it can render the first local image referenced inside `scene.json`, or the first local sibling image next to `scene.pkg` / `scene.json`, before falling back to the imported preview image.
- `scene-runner` also supports a first layered subset for `scene.json`: multiple local image layers can be composited in order using simple `x`, `y`, `width`, `height`, `scale`, and `opacity` fields.
- For common `scene.pkg` bundles, `scene-runner` can now extract the packaged `scene.json`, resolve simple model/material/texture chains, decode embedded PNG/JPEG/GIF/WebP payloads and common BC1/DXT1-style packaged `.tex` textures, and composite those layers into a static scene result.
- `web-runner` currently supports a narrow local HTML subset: it can render the first local `<img src=...>` or `background-image:url(...)` it finds, or a parsed hex `background` / `background-color`, before falling back to the imported preview image.
- The Settings modal surfaces the current video runtime dependency state so you can see whether Backlayer is limited to preview fallback because `mpv`/`libmpv` is unavailable or because playback integration is still unfinished.
- When an imported item includes a preview image, the Tauri manager uses that real local preview in the asset browser instead of a synthetic placeholder.
- Animated shader assets should declare a WGSL uniform block matching:
```wgsl
struct ProbeUniforms {
  time_seconds: f32,
  width: f32,
  height: f32,
  _padding: f32,
}

@group(0) @binding(0)
var<uniform> probe: ProbeUniforms;
```
- `pause_on_battery` currently affects the live shader worker path and uses Linux power-supply state from `/sys/class/power_supply`.
