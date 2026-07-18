# Manual Verification Matrix

Use this as the release-hardening checklist for the current MVP.

## Daemon And UI

- Start `backlayerd --serve`
- Open the Tauri manager
- Confirm the first runtime snapshot loads
- Confirm the manager reconnects if the daemon is restarted

## Wallpaper Assignment

- Assign a native image wallpaper
- Assign a shader wallpaper
- Assign a video wallpaper
- Assign a native scene wallpaper
- Confirm each appears on the selected monitor

## Runtime Policy

- Enable `pause_on_fullscreen` and confirm animated wallpapers pause
- Enable `pause_on_battery` and confirm animated wallpapers pause on battery power
- Lower `fps_limit` and confirm animation cadence visibly drops

## Monitor Behavior

- Change monitor layout while the daemon is running
- Unplug/replug an external monitor if available
- Confirm assignments stay bound to the correct output identity
- Confirm the daemon does not require a restart to reconcile monitor changes

## Input And Placement

- Confirm wallpaper surfaces stay behind normal windows
- Confirm clicks and keyboard input pass through to normal desktop/application windows

## Recovery

- Kill a shader/video/scene runner process
- Confirm the daemon reports the failure and restarts the renderer path
- Confirm the UI remains connected to the daemon

## Native Create Flows

- Create a native still image wallpaper
- Create a native video wallpaper
- Create a native shader wallpaper
- Create and edit a native scene wallpaper
- Confirm each created asset appears in the browser and can be assigned

## Scene Parity: Composer Preview vs Wallpaper

Author one scene in the Scene Composer that exercises each row, assign it,
and compare the composer preview against the rendered wallpaper
side-by-side. Expect a statistical match for particles (patterns are not
particle-for-particle identical) and slightly softer stacked translucency
at runtime (linear-space blending, spec §5). Reference:
`docs/scene-semantics-spec.md`; log any new divergence in
`docs/scene-parity-audit.md`.

- Sprite placement: offsets, `scale`, `rotation_deg`, and each `fit` mode
  land in the same relative position and size on both sides
- Sprite behaviors: drift, pulse, and orbit move with the same amplitude,
  direction, and cadence
- Sprite blend modes: `add`/`screen` brighten additively; `alpha`/`multiply`
  composite normally
- Effects: glow radius/falloff and pulse cadence, vignette extent and
  softness, scanline band width/drift speed, and fog band position and
  wave motion all match; effect colors match the authored hex on both sides
- Emitters: each preset (rain, snow, dust, embers) matches in particle
  size, speed, direction, spread, and density at the same output aspect
- Particle appearance: over-life size/alpha/color changes match; particles
  glow additively and sit on top of all sprites and effects on both sides
- Burst emission: `burst_count`/`burst_on_start` produce comparable bursts
  at comparable times
- Occluders and surfaces: particles hide behind occluder regions at the
  same boundaries (enable `BACKLAYER_DEBUG_PARTICLE_AREAS=1` to see
  runtime blocker outlines), and snow/dust land and stay landed on the
  same surface edges while rain/embers die on contact
- Resolution independence: assign the same scene to outputs of different
  resolutions and confirm composition, particle sizes, and speeds keep the
  authored proportions
