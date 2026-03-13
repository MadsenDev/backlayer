# Animation Probe

`animation-probe` is a standalone Wayland shader test binary for debugging animated rendering outside `backlayerd`.

It exists so animated shader work can be isolated from the daemon, UI, and runtime manager while we narrow native crashes.

## Run

```bash
cargo run -p animation-probe
```

Optional arguments:

```bash
cargo run -p animation-probe -- eDP-1 30
```

Arguments:

- first arg: output name, like `eDP-1`
- second arg: FPS, default `30`

## What it does

- creates one background layer-shell surface
- creates one `wgpu` surface/device/pipeline
- uploads a tiny time/resolution uniform buffer
- renders one animated WGSL shader continuously

## Why it exists

If animation crashes here too, the problem is likely in:

- repeated frame submission
- the animated WGSL path
- uniform buffer updates
- or a native Wayland/`wgpu` interaction

If it only crashes in `backlayerd`, the problem is more likely in:

- worker lifecycle
- restart logic
- or runtime manager integration
