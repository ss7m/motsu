in vec2 position;
in float crop_left;
in float crop_right;
in float crop_top;
in float crop_bottom;

out vec2 v_uv;

void main() {
  gl_Position = vec4(position, 0., 1.);

  float x = position.x > 0 ? crop_right : crop_left;
  float y = position.y > 0 ? crop_top : crop_bottom;

  v_uv = vec2(x, y);
}
