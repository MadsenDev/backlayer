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
  let uv = position.xy / vec2<f32>(1920.0, 1080.0);
  let centered = uv - vec2<f32>(0.5, 0.5);
  let radius = length(centered * vec2<f32>(1.0, 1.35));
  let sweep = smoothstep(0.02, 0.0, abs(fract(uv.y * 18.0) - 0.5));
  let vertical = smoothstep(0.18, 0.0, abs(uv.x - 0.58));
  let glow = max(sweep * 0.45, vertical * 0.9);
  let vignette = smoothstep(0.95, 0.18, radius);

  let base = vec3<f32>(0.03, 0.015, 0.01);
  let ember = vec3<f32>(0.98, 0.34, 0.08) * glow;
  let wash = vec3<f32>(0.24, 0.06, 0.02) * (1.0 - radius * 0.9);
  let color = (base + wash + ember) * vignette;

  return vec4<f32>(color, 1.0);
}
