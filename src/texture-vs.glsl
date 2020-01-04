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
  v_uv = position * .5 + .5; // transform the position of the vertex into UV space
}
