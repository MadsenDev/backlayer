# Known Limitations

These are the important current MVP limitations, not aspirational future ideas.

## Platform Scope

- Backlayer is Hyprland-only for the current MVP.
- KDE, GNOME, X11, and non-wlroots environments are out of scope right now.
- On KDE Plasma Wayland specifically, Backlayer can appear to "work" while Plasma keeps ownership of wallpaper composition and only shows a static preview/fallback image instead of the live surface.

## Video

- Video playback currently uses an FFmpeg-based decode path in `video-runner`.
- `libmpv` integration is not done yet.
- Hardware-accelerated playback is not proven yet.

## Native Scenes

- Particle image sprites are not finished in the native scene runtime.
- Scene Composer preview parity with the applied wallpaper is improved but not perfect.
- The editor is not yet a full creator-grade tool.

## Workshop Compatibility

- Workshop support is opt-in with `BACKLAYER_ENABLE_WORKSHOP=1`.
- Compatibility is partial.
- `web` and imported `scene` support are still best-effort, not full Wallpaper Engine parity.
- `application` wallpapers are not a target for the MVP.

## Runtime Hardening

- Monitor change handling is now daemon-driven during `--serve`, but multi-monitor and hotplug behavior still need more manual validation.
- Input passthrough is implemented at the layer-shell level and still needs explicit release validation across the current wallpaper types.

## Packaging

- There is no polished distro package/install story yet.
- The supported setup path is still workspace-driven plus `systemd --user`.
