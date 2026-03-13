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
  let line_x = abs(fract(uv.x * 18.0) - 0.5);
  let line_y = abs(fract(uv.y * 10.0) - 0.5);
  let glow = smoothstep(0.07, 0.0, min(line_x, line_y));
  let base = vec3<f32>(0.03, 0.05, 0.09);
  let accent = vec3<f32>(0.13, 0.79, 0.74) * glow;

  return vec4<f32>(base + accent, 1.0);
}
