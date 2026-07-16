# Scene Parity Audit: Composer Preview vs `scene-runner`

Status: audit complete (2026-07-16); quick-win fixes landed the same day —
see the fix log below. This document catalogs every known behavioral and
visual divergence between the Scene Composer preview (Canvas2D,
`apps/ui/src/App.tsx`) and the native scene runtime (wgpu/WGSL,
`apps/scene-runner/src/main.rs`). It is the input for the scene-semantics
spec and the parity fixes tracked in `TODO.md` under Milestone 1.

Line references are as of commit `cd486b4`.

## Fix log (2026-07-16)

Landed as quick wins; the divergence descriptions below are kept as the
historical record.

Runtime fixes:
- **2.1** sprite `rotation_deg` now rotates the sampled rect in the sprite
  shader (inverse-rotated UV lookup around the rect center)
- **4.3** rain occlusion segments now follow the streak's actual long axis
  (`(-sin, cos)` instead of `(cos, sin)`)
- **1.3 (partial)** effect, particle, and clear colors are srgb-decoded
  before upload when the surface format is sRGB, so runtime colors no longer
  render brighter than authored; the gamma-vs-linear *blending space*
  difference remains and is a spec decision
- **4.4 (partial)** `min_life` now floors at 0.2 like the preview, which also
  removes the `max_life = 0` NaN
- **4.6 (new finding)** the rain streak feather in the particle shader was
  inverted — `smoothstep(1.0, 0.82, 1.0 - edge)` produced a transparent
  center and hard outer edges instead of an opaque center with a feathered
  outer 18%; now `1.0 - smoothstep(0.82, 1.0, edge)`

Preview fixes:
- **1.4** particles now draw in a second pass after all sprites and effects,
  matching the runtime's always-on-top particle pass
- **1.5** particles now composite additively (`lighter`) and circular
  particles use a radial gradient matching the runtime's
  `smoothstep(1, 0, r)` feather; streak edge feather remains approximate
- **2.2** sprite blend modes now mirror the runtime's pipeline mapping
  (add/screen → additive, alpha/multiply → source-over); exact
  screen/multiply semantics remain a spec decision
- **2.3** drift/pulse/orbit constants now use the runtime's values, and orbit
  respects `amount_y`
- **3.1 + 3.2** glow uses the runtime's `(1 − 1.65·d)²` falloff and no longer
  hardcodes the default color in the outer gradient stop
- **3.3** vignette uses the runtime's radius fraction (0.42 of the
  half-diagonal) and `^1.8` falloff

Still open from this audit: 1.1 (particle model), 1.2 (pixel units),
1.3 (blending space), 1.6 (time base), 1.7 (randomness), 4.1/4.2 (curve
fallbacks and sanitation), 4.4 (region/line clamps), 4.5 (particle sprites),
and the low-severity items in 2.4 and 3.4.

## How the two renderers relate

There is no shared code. The preview re-implements the entire scene model in
TypeScript on a 2D canvas (`drawComposerSceneFrame`, `App.tsx:4739`); the
runtime implements it again in Rust with three WGSL pipelines
(`main.rs:28,79,142`) plus a CPU particle simulation
(`NativeSceneRuntime::update_emitters`, `main.rs:620`). Every divergence below
is a place where the two re-implementations disagree. Parity currently holds
only by discipline, and it has already drifted.

Severity legend — **High**: visibly changes composition, color, or motion in
common scenes. **Medium**: visible in specific configurations. **Low**:
edge-case or cosmetic.

## 1. Systemic divergences (affect everything)

### 1.1 Particle simulation model — High

- Runtime: persistent, stateful simulation. Particles spawn from an emission
  accumulator (`main.rs:669`), integrate velocity/gravity/drag per frame,
  carry landing state, and die at end of life.
- Preview: stateless closed-form resampling. Each frame re-derives a fixed
  population of particles from `(timeSeconds + phase) % maxLife`
  (`App.tsx:5128-5144`); nothing persists between frames.

Consequences, each visible on its own:
- Burst emission (`burst_count`, `burst_on_start`) works at runtime
  (`main.rs:634`), and is **completely ignored by the preview**.
- Drag: runtime integrates `v *= 1 - drag·dt·0.08` per frame
  (`main.rs:686`); preview applies a one-shot closed-form
  `dragScale = 1 - drag·age·0.08` to the initial velocity
  (`App.tsx:5140`). Trajectories agree only at low drag; with `drag ≳ 1`
  the paths visibly differ.
- Landing on surfaces: runtime landing is permanent (`landed = true`,
  `vx *= 0.15` slide, `main.rs:1363-1369`); preview merely re-pins a
  particle's y each frame while it happens to be inside the blocker's
  vertical extent (`App.tsx:5338-5348`), so landed particles pop back into
  motion when their phase wraps and never accumulate.
- Population: preview always renders the steady-state count and forces a
  minimum of 12 particles (`App.tsx:5108`); runtime ramps up from its warm
  start and honors low emission rates exactly.
- Crossing detection: runtime detects surface crossings frame-to-frame
  (`previous_y <= surface_y && y + r >= surface_y`, `main.rs:1352`); preview
  tests "currently below surface top", which also captures particles spawned
  inside the region.

### 1.2 Coordinate space and pixel units — High

Sprite `x`/`y` offsets, emitter `size`, `min/max_speed`, `gravity_*`, and
particle dimensions are all raw pixel values, but the two sides evaluate them
against different canvas sizes:

- Preview: a logical canvas of `previewTargetSize` (or the base image size),
  downscaled so the long edge is ≤ 1280 px (≤ 960 during interaction),
  `App.tsx:3217-3222`.
- Runtime: the actual monitor surface resolution. The scene document's own
  `width`/`height` are discarded — `canvas_size` is overwritten with the
  output size at startup (`main.rs:260`).

A 4 px particle at 420 px/s occupies 4/1280 of the preview width but 4/2560 of
a 1440p monitor: **particles render at half the relative size and speed on
higher-resolution outputs than the preview showed**. Normalized fields
(origins, regions, areas) scale correctly; every pixel-unit field does not.
The spec must either normalize these units or scale them by
`surface_size / document_size` at runtime.

### 1.3 Blending and color space — High

- Canvas2D composites in gamma-encoded sRGB. wgpu renders to an
  `Rgba8UnormSrgb` target, so all blending happens in linear space.
  Identical alpha values produce different results, most visibly in soft
  gradients (glow, fog) and stacked translucency.
- The runtime parses hex colors as `component / 255` and feeds them directly
  to shaders as if linear (`parse_color_components`, `main.rs:1845`). On an
  sRGB target those get re-encoded on write, so **runtime effect and particle
  colors render brighter than the same hex in the preview**. Correct behavior
  is to srgb-decode the components before upload (or use linear-consistent
  handling in the spec).

### 1.4 Draw order for particles — High

- Runtime: sprites and effects draw in document order, then **all particles
  draw last in a single pass, always on top** (`main.rs:1117-1198`).
- Preview: each emitter's particles draw inline at the emitter's position in
  the node list (`App.tsx:4754-4768`), so a sprite placed after an emitter
  covers its particles in the preview but not on the wallpaper.

### 1.5 Particle blend mode and softness — High

- Runtime particles always use additive blending (`main.rs:980`), circles get
  a full radial `smoothstep(1,0,r)` feather (soft glowing blobs,
  `main.rs:205-212`), and rain rects feather over their outer ~18%.
- Preview particles use plain source-over compositing with hard-edged
  `arc()`/`fillRect()` shapes (`App.tsx:5083-5095`).

Net effect: embers/snow/dust bloom and glow at runtime and look flat and
hard-edged in the preview.

### 1.6 Time base and pause behavior — Medium

- Preview: wall-clock phase sampling; when paused, `timeSeconds` keeps
  advancing, so the scene jumps on resume.
- Runtime: behaviors/effects use wall-clock elapsed time, but the particle
  simulation advances a **fixed dt of one nominal frame interval per rendered
  frame** (`main.rs:295-296`), so dropped frames slow particles down relative
  to behaviors, and fullscreen-pause freezes particles but not behavior/effect
  phases.

### 1.7 Randomness — Low (decision needed)

Runtime uses a 64-bit LCG seeded from a hash of `(emitter id, preset)`
(`main.rs:2262`); preview uses a per-index FNV/LCG-32 seeded additionally from
emitter parameters (`App.tsx:5616-5632`). Particle patterns can never match
particle-for-particle. The spec should decide whether exact stochastic
reproducibility is a goal (shared PRNG + seed) or explicitly out of scope.

## 2. Sprite divergences

### 2.1 `rotation_deg` is ignored at runtime — High (runtime bug)

The preview rotates sprites around their center (`App.tsx:4982`). The runtime
sprite pipeline has no rotation at all — `SpriteUniforms` carries only an
axis-aligned rect (`main.rs:28-77`). Any scene using sprite rotation renders
differently on the wallpaper.

### 2.2 Blend modes ignored in preview — High

`SceneBlendMode` supports `alpha | add | screen | multiply`. The runtime maps
add/screen → additive pipeline and alpha/multiply → alpha pipeline
(`main.rs:1144-1151`) — itself an approximation. The preview never sets
`globalCompositeOperation`, so **all blend modes render as source-over**.
Canvas2D natively supports `lighter`, `screen`, and `multiply`, so the preview
can actually express these more faithfully than the runtime today; the spec
should pin exact semantics for all four.

### 2.3 Behavior constants disagree — Medium

For the same document, animated sprites move differently:

| Behavior | Preview (`App.tsx:4817-4831`) | Runtime (`main.rs:1989-2004`) |
|---|---|---|
| Drift y phase | `cos(phase · 0.9)` | `cos(phase · 0.85)` |
| Pulse opacity | `opacity · (0.88 + (sin+1)·0.06)` | `opacity · (0.9 + (sin+1)·0.05)` |
| Orbit y amplitude | `max(amount·0.6, 0)` — **ignores `amount_y`** | `max(amount_y, amount·0.6)` |

### 2.4 Minor sprite layout differences — Low

- Runtime clamps `scale ≥ 0.1` (`main.rs:1987`); preview uses the raw value.
- Runtime rounds layout to whole pixels (`main.rs:2208-2252`); preview keeps
  floats. Sub-pixel drift only.

## 3. Effect divergences

### 3.1 Glow falloff — Medium

Runtime: `strength = max(0, 1 − 1.65·d)²` with `d = dist / max(w,h)`, single
flat-color pass (`main.rs:118-123`). Preview: three-stop radial gradient
(1.0 / 0.34 / 0) out to radius `max(w,h) · 0.61` (`App.tsx:4998-5010`).
Different profile and different extent.

### 3.2 Glow outer stop hardcodes the default color — Medium (preview bug)

The preview's outermost gradient stop is literally
`rgba(255, 199, 133, 0)` (`App.tsx:5008`). With a custom glow color the
falloff tints toward the default peach in the preview only.

### 3.3 Vignette curve — Medium

Runtime: `pow(clamp((d − 0.42)/0.58), 1.8)` with `d = dist / length(center)`
(`main.rs:124-128`). Preview: linear gradient alpha ramp with an inner radius
of `min(w,h) · 0.22` and no exponent (`App.tsx:5011-5024`). The preview
vignette is noticeably harder-edged.

### 3.4 Scanlines / fog rasterization — Low

Formulas and constants match (96 bands, 0.18 / 0.22 alpha factors), but the
preview draws scanlines as ~0.2-line-height rows per pixel row and fog as
quantized cells of `min(w,h)/180` px (`App.tsx:5025-5059`), vs. per-pixel
shader evaluation at runtime. Fog looks blocky in the preview at small sizes.

## 4. Emitter/particle divergences (beyond the model split)

### 4.1 Default curve fallbacks — High for hand-written docs

When a document has no `size_curve`/`alpha_curve`/`color_curve`:

- Preview falls back to rich per-preset defaults (`App.tsx:285-322`) — e.g.
  embers flare to full alpha at 20% life then fade through
  yellow → orange → dark red.
- Runtime falls back to size = 1, alpha = linear fade `1 − progress`, and a
  flat emitter color (`main.rs:2062-2068`).

The composer bakes its defaults into saved documents
(`createComposerEmitterNode`, `App.tsx:2442`), so composer-created scenes are
unaffected — but any doc missing curves renders very differently. The spec
should pick one fallback (recommend: the runtime's simple one, with the
composer always writing curves explicitly).

### 4.2 Curve input sanitation — Medium

Preview sorts stops by x, clamps y to [0, 2.5], and pads endpoints to x=0/x=1
(`resolveScalarCurve`, `App.tsx:415`). Runtime evaluates the raw list in file
order with no clamps (`main.rs:1853`); unsorted stops silently misbehave.

### 4.3 Rain occlusion segment uses the wrong axis — Medium (runtime bug)

The rain streak quad's long axis is its local Y. The runtime builds the
occlusion segment along `(cos a, sin a)` — the local X axis — scaled by the
long dimension (`main.rs:2093-2101`), i.e. rotated 90° from the streak.
The preview uses `angle + π/2`, the correct long axis
(`App.tsx:5176-5182`). Rain hides behind occluders at visibly different
positions.

### 4.4 Parameter clamps disagree — Low

- `min_life`: preview clamps ≥ 0.2 (`App.tsx:382`); runtime allows 0 — and
  `max_life = 0` produces `0/0 = NaN` progress at runtime
  (`main.rs:2062`), a latent NaN bug.
- `region_radius`, `line_length`: preview clamps to [0.01, 1]; runtime
  unclamped.

### 4.5 Particle sprite images — known gap

Preview already renders textured particles via `particle_image_key`
(`App.tsx:5072-5081`); the runtime ignores the field entirely (only feathered
circles/rects exist). Already tracked as its own Milestone 1 item — land the
runtime side and reconcile sizing/rotation semantics with the preview's.

## 5. What already matches

Worth protecting while fixing the rest: background clear color `#05070a`;
per-preset defaults for origin/direction/shape/region/speed/life/color
(`App.tsx:116-283` ↔ `main.rs:1574-1698`); spawn-position sampling for
point/box/line/circle shapes including `sqrt` disc sampling; spawn size/alpha
jitter (`0.55 + 0.7·r`, `0.55 + 0.45·r`); per-preset render dimensions
(rain 1.2×/8.5×, snow 2.0×, dust 2.2×, embers 2.0×) and alpha scales
(0.92/0.86/0.7/1.0); rain angle-from-velocity (`atan2(vy,vx) − π/2 +
particle_rotation`); occluder/surface geometry helpers (point-in-polygon,
segment tests, `polygon_surface_y`) which are line-for-line ports; and the
curve interpolation math itself.

## 6. Recommended fix order

1. **Quick wins, runtime-side** (small, high visibility): sprite rotation in
   the sprite shader; rain occlusion axis; srgb-decode colors before upload;
   NaN guard on `max_life`.
2. **Quick wins, preview-side**: `globalCompositeOperation` for blend modes;
   particle additive compositing (`lighter`) + radial-gradient feather;
   draw particles after all nodes; fix the glow gradient's hardcoded outer
   stop; align behavior constants (pick the runtime's values).
3. **Spec decisions** (write into the scene-semantics spec before coding):
   pixel-unit scaling rule vs. document size (fixes 1.2); curve fallback and
   sanitation rules (4.1, 4.2); blend-mode exact semantics (2.2); stochastic
   reproducibility stance (1.7); time-base rule (1.6).
4. **Structural**: replace the preview's stateless resampling with the same
   stateful simulation (fixes 1.1 wholesale — burst, drag, landing,
   population). This is the largest piece; a TS port of the ~150-line
   simulation loop keeps the current architecture, while a WebGPU/WASM port
   of the runtime pipeline would make parity hold by construction and is the
   direction the roadmap already points at.

Verification: `BACKLAYER_DEBUG_PARTICLE_AREAS=1` draws blocker outlines at
runtime for comparing occluder placement against the composer overlays. A
side-by-side checklist for the manual verification matrix is tracked as its
own TODO item.
