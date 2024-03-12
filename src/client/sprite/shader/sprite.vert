#version 320 es

in vec2 a_Position;
in vec2 a_UV;

uniform mat3 u_Transform;

out highp vec2 f_UV;
out highp vec2 f_Pos;

void main() {
  vec4 v_Position = vec4(u_Transform * vec3(a_Position.xy, 1.0), 1.0);
  f_UV = a_UV;
  f_Pos = v_Position.xy;
  gl_Position = v_Position;
}
