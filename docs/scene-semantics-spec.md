# Scene Semantics Spec

This document is the single source of truth for how a `backlayer_scene_v2`
document is evaluated. Two independent implementations follow it — the Scene
Composer preview (Canvas2D, `apps/ui/src/App.tsx`) and the native runtime
(wgpu/WGSL, `apps/scene-runner/src/main.rs`) — and any change to scene
semantics must be made against this spec first, then applied to both sides
(or recorded as a divergence in `docs/scene-parity-audit.md`).

Normative language: "must" items are the contract both renderers implement
today. "Known divergence" items are accepted, documented gaps tracked in the
parity audit.

## 1. Document model

A scene is a JSON document with `schema = "backlayer_scene_v2"`,
`version = 2`, a `width`/`height` pair, an `images` table (`key` → relative
path, with the required key `base`), and an ordered `nodes` list of `sprite`,
`effect`, `emitter`, and `particle_area` nodes.

- `width`/`height` are the **document reference size**. The composer writes
  the base image's pixel dimensions here. They define the space pixel-unit
  fields are authored in (§2); they are *not* the output resolution.
- Node order is meaningful: sprites and effects render in document order
  (§6).

## 2. Coordinate space and units

- Origin is the top-left corner; +x right, +y down. Angles are degrees in
  the document, converted to radians at evaluation; 0° points +x and
  positive angles rotate toward +y (clockwise on screen).
- Each renderer draws onto a **canvas** of its own size: the preview uses
  its viewport bitmap (long edge ≤ 1280 px, monitor aspect), the runtime
  uses the output surface resolution. The document reference size never
  constrains the canvas.

Fields are either **normalized** or **pixel-unit**:

- **Normalized fields** are fractions of the canvas and scale implicitly:
  emitter `origin_x`/`origin_y`, `region_width`/`region_height` (of canvas
  width/height), `region_radius` (of the canvas short edge), `line_length`
  (of canvas width), particle-area `region` rects and polygon `points`, and
  sprite `particle_region` (relative to the sprite's laid-out rect).
- **Pixel-unit fields** are authored in document space and must be
  multiplied by the **unit scale**

  ```
  unit_scale = max(canvas_width, canvas_height) / max(document_width, document_height)
  ```

  (both terms floored at 1). The pixel-unit fields are: sprite `x`/`y`
  offsets; behavior `amount_x`/`amount_y` (drift) and `amount`/`amount_y`
  (orbit); emitter `size`, `min_speed`/`max_speed` (px/s), and
  `gravity_x`/`gravity_y` (px/s²). Pulse `amount` is a unitless scale delta
  and is **not** scaled. Time-domain fields (`min_life`, `max_life`, `drag`,
  `emission_rate`, behavior `speed`/`phase`) are never scaled.
- Minimum-size floors (`max(size, 1)` for particle radii) are applied in
  canvas pixels *after* scaling — a particle never collapses below one
  physical pixel on either renderer.
- Sprite `fit` (`cover`/`contain`/`stretch`/`center`) resolves against the
  canvas, then `scale` (floored at 0.1) multiplies the fitted size except in
  `center` mode. The runtime rounds the laid-out rect to whole pixels; the
  preview keeps floats (accepted sub-pixel divergence).

## 3. Time base

- Behaviors and effects are functions of continuous wall-clock time since
  renderer start (`time_seconds`); a paused renderer does not rewind or
  freeze this clock, so behavior/effect phases jump forward on resume.
- The particle simulation integrates with the **real wall-clock delta
  between rendered frames, clamped to 100 ms**. Dropped frames therefore
  advance the simulation by real elapsed time (up to the clamp), and a
  resume after pause steps at most 100 ms.
- On its first frame each renderer may prime the simulation with one nominal
  frame step.
- Emitters warm-start: when an emitter has no particles and a positive
  emission rate, `round(emission_rate × average_life)` particles (capped at
  `max_particles`) are spawned with random ages so steady state is reached
  immediately. Spawned ages back-date position by closed-form integration of
  velocity, gravity, and drag.

## 4. Curves and parameter resolution

- Scalar curves (`size_curve`, `alpha_curve`) and color curves
  (`color_curve`) are lists of stops evaluated by linear interpolation in
  life progress `t = life / max_life ∈ [0, 1]`.
- Sanitation (both sides, before evaluation): clamp stop `x` to [0, 1] and
  scalar `y` to [0, 2.5]; sort stops by `x`; pad endpoints so the curve
  covers `x = 0` and `x = 1`. Color stops with invalid hex fall back to
  `#ffffff`.
- A document that omits a curve gets the per-preset default curves (the same
  tables on both sides); the composer additionally bakes these defaults into
  saved documents.
- Unset emitter parameters resolve to per-preset defaults, then clamp:
  `origin_x`/`origin_y` → [0, 1]; `region_width`/`region_height` → [0, 1];
  `region_radius`, `line_length` → [0.01, 1]; `min_speed` ≥ 0;
  `max_speed` ≥ `min_speed`; `min_life` ≥ 0.2 (also the NaN guard);
  `max_life` ≥ `min_life`. An unset `line_angle_deg` follows the emitter's
  resolved direction (custom `direction_deg` if set, else the preset
  default).
- Emitter randomness uses a per-emitter deterministic seed derived from
  `(node id, preset)`. The two renderers use different PRNGs, so particle
  patterns match **statistically, not particle-for-particle** — exact
  stochastic reproducibility is explicitly out of scope.

## 5. Color and blending

- All authored colors are 6-digit sRGB hex. On an sRGB render target the
  runtime decodes components to linear before upload so colors match their
  authored values on screen.
- Background clear color is `#05070a`.
- Blend modes per node type:
  - **Sprites** (`blend`): `alpha` and `multiply` render with premultiplied
    source-over ("alpha") blending; `add` and `screen` render additively
    (`src_alpha·src + dst`). Exact `screen`/`multiply` arithmetic is an
    accepted approximation on both sides until a shader path exists for
    them.
  - **Effects** always alpha-blend.
  - **Particles** always blend additively; circles get a radial
    `smoothstep(1, 0, r)` feather and rain streaks feather over their outer
    18 %.
- Known divergence (audit 1.3): Canvas2D composites in gamma space while
  wgpu blends in linear space, so stacked translucency and soft gradients
  differ slightly. Closing this requires the shared-GPU-preview track and is
  out of scope for renderer-side fixes.

## 6. Draw order

1. Sprites and effects, in document order.
2. All particles from all emitters, in one pass on top.
3. (Preview only) selection/authoring overlays, never part of the scene.

## 7. Occluders and landing surfaces

- Blockers come from sprites flagged `particle_occluder`/`particle_surface`
  (rect = laid-out sprite rect, optionally narrowed by `particle_region`)
  and from `particle_area` nodes (normalized rect or polygon).
- Blocker geometry is evaluated at time 0 — behavior animation never moves
  a blocker.
- Occlusion hides a particle when its center±radius intersects an occluder
  (rain tests its streak segment instead).
- Landing: a particle lands when it crosses a surface's top edge between
  frames (`previous_y ≤ surface_y` and `y + radius ≥ surface_y`). Snow and
  dust land permanently (`y` pinned, `vx ×= 0.15`, `vy = 0`); rain and
  embers are killed on contact.

## 8. Change discipline

When touching any behavior described here: update this spec in the same
changeset, apply the change to both renderers (or log the divergence in
`docs/scene-parity-audit.md`), and cover runtime-side rules with unit tests
in `apps/scene-runner` where they are testable without a compositor.
