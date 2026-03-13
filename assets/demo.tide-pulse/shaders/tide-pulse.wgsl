struct ProbeUniforms {
  time_seconds: f32,
  width: f32,
  height: f32,
  _padding: f32,
}

@group(0) @binding(0)
var<uniform> probe: ProbeUniforms;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, 1.0),
  );

  let xy = positions[vertex_index];
  return vec4<f32>(xy, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
  let resolution = vec2<f32>(max(probe.width, 1.0), max(probe.height, 1.0));
  let uv = position.xy / resolution;
  let centered = uv - vec2<f32>(0.5, 0.5);
  let aspect = probe.width / max(probe.height, 1.0);
  let warped = vec2<f32>(centered.x * aspect, centered.y);
  let time = probe.time_seconds;

  let ripple = sin(length(warped) * 24.0 - time * 2.8) * 0.5 + 0.5;
  let sweep = sin((warped.x + warped.y) * 10.0 + time * 1.6) * 0.5 + 0.5;
  let crest = smoothstep(0.58, 1.0, max(ripple, sweep));
  let haze = smoothstep(1.1, 0.08, length(warped));

  let base = vec3<f32>(0.02, 0.05, 0.09);
  let surf = vec3<f32>(0.08, 0.38, 0.76) * ripple;
  let glow = vec3<f32>(0.62, 0.88, 0.98) * crest;
  let color = (base + surf + glow) * haze;

  return vec4<f32>(color, 1.0);
}
