in vec2 position;

out vec2 v_uv;

const vec2[4] QUAD_POS = vec2[](
  vec2(-1., -1.),
  vec2( 1., -1.),
  vec2( 1.,  1.),
  vec2(-1.,  1.)
);

void main() {
  //vec2 p = QUAD_POS[gl_VertexID];

  gl_Position = vec4(position, 0., 1.);

  float x = position.x > 0 ? 1.0 : 0;
  float y = position.y > 0 ? 0 : 1.0; // fixes having to flip the image
  v_uv = vec2(x, y);
}
