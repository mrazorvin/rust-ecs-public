#version 320 es

uniform highp mat3 u_WorldMatrix;

uniform highp vec2 u_Position;
uniform highp vec2 u_Size;
uniform highp vec2 u_Offset;
uniform highp vec2 u_LightPos;
uniform highp vec2 u_Origin;
uniform bool u_Shading;
uniform bool u_DisableTopLight;

uniform sampler2D u_Image;
uniform highp float u_ImageWidth;
uniform highp float u_TileWidth;
uniform highp float u_TileHeight;
uniform highp int u_TileId;
uniform bool u_Height;

in highp vec2 f_UV;
in highp vec2 f_Pos;

struct Lights {
  highp float x;
  highp float y;
  highp float time;
  highp float radius;
};

struct LightsIntersection {
  highp float x0;
  highp float y0;
  highp float x1;
  highp float y1;
  highp vec2 light_pos;
  highp float light_radius;
  highp int tile_id;
};

layout (std140) uniform u_Lights {
  Lights data[64];
};

layout (std140) uniform u_LightsIntersection {
  LightsIntersection data_intersection[64];
};

uniform highp int u_LightsLen;
uniform highp int u_LightsIntersectionLen;

highp vec4 get_color(highp vec2 a_Position, highp vec2 a_Size, highp vec2 a_Offset, highp vec2 a_UV) {
  if (u_ImageWidth == 0.0) {
    highp vec4 color = texture(u_Image, a_UV);
    return vec4(color.rgb * 0.60, color.a);
  }

  highp float min_x = a_Offset.x / u_TileWidth;
  highp float min_y = a_Offset.y / u_TileHeight;
  highp float max_x = min_x + a_Size.x / u_TileWidth;
  highp float max_y = min_y + a_Size.y / u_TileHeight;
  highp float uv_offset_x = a_Position.x / u_ImageWidth;
  highp float uv_offset_y = a_Position.y / u_ImageWidth;
  highp float uv_width = a_Size.x / u_ImageWidth;
  highp float uv_height = a_Size.y / u_ImageWidth;

  if (a_UV.x >= min_x && a_UV.x <= max_x && a_UV.y >= min_y && a_UV.y <= max_y) {
    highp float uv_x = (a_UV.x - min_x) / (max_x - min_x);
    highp float uv_y = (a_UV.y - min_y) / (max_y - min_y);

    // Pixel art AA: https://www.shadertoy.com/view/ltBGWc
    // TODO add 1px transparent border between sprites 
    // highp vec2 pix = vec2(uv_offset_x + uv_x * uv_width, uv_offset_y + uv_y * uv_height) * u_ImageWidth;
    // pix = floor(pix) + smoothstep(0.0, 1.0, fract(pix) / fwidth(pix)) - 0.5;
    // texture(u_Image, pix/ u_ImageWidth);

    return texture(u_Image, vec2(uv_offset_x + uv_x * uv_width, uv_offset_y + uv_y * uv_height));
  }

  return vec4(vec3(0.0), 0.0);
}

highp vec4 apply_light(highp vec4 a_Color, highp float a_LightStrength, highp float a_LightMod) {
  return vec4((a_Color.rgb * a_LightStrength) * a_LightMod, a_Color.a);
}

highp vec2 to_cartesian(highp vec2 pos, highp vec2 origin) {
  highp float x = pos.x - origin.x;
  highp float y = pos.y - origin.y;

  return vec2(x * cos(radians(45.0)) - y * sin(radians(45.0)) + origin.x, x * sin(radians(45.0)) + y * cos(radians(45.0)) + origin.y);
}

highp float get_light_strength(highp float a_Light, highp vec2 v_LightPos, highp float radius, highp float strenght, highp float time) {
  highp float v_Light = a_Light;
  highp float v_Length = length(v_LightPos - f_Pos);
  highp float remain_light = (1.0 - v_Light / 1.2);
  if (v_Length <= radius) {
    highp float v_TintStrength = pow(1.0 - v_Length / radius, 4.0) * time;
    if (u_DisableTopLight == true || u_Height) {
      v_Light = max(min(v_Light + strenght * remain_light * v_TintStrength, 1.0), v_Light);
    } else {
      highp float distance = length(to_cartesian(f_Pos - v_LightPos, vec2(1.0 / 1920.0, 1.0 / 1080.0))) / radius;
      highp float color_1 = min(v_Light + strenght * remain_light * v_TintStrength * distance, 1.0);
      highp float color_2 = min(v_Light + strenght * 2.0 * remain_light * v_TintStrength * length(to_cartesian(vec2(0.5, 0.1) - f_UV, vec2(1.0 / u_TileWidth, 1.0 / u_TileHeight))), 1.0);
      v_Light = max(min(mix(color_1, color_2, 1.0 - distance), 1.0), v_Light);

        // v_Light = color_2`;

      if (length(vec2(0.5, 0.0) - f_UV) < 0.6) {
        v_Light = mix(max(min(v_Light + strenght * remain_light * v_TintStrength, 1.0), v_Light), v_Light, length(vec2(0.5, 0.0) - f_UV) / 0.6);
      }
    }
  }

  return v_Light;
}

out highp vec4 o_Color;
void main() {
  highp vec4 v_Color = get_color(u_Position, u_Size, u_Offset, f_UV);
  highp vec2 v_LightPos = (u_WorldMatrix * vec3(u_LightPos.xy, 1.0)).xy;

  highp float v_AlphaMod = 1.0;

  highp vec3 behide_light = u_WorldMatrix * vec3(u_Origin, 1.0);
  if (v_Color.a > 0.9 && (length(v_LightPos - f_Pos) / 0.1 < 1.0 || length(vec2(v_LightPos.x, v_LightPos.y + 0.08) - f_Pos) / 0.1 < 1.0) && !u_Height && behide_light.y < v_LightPos.y) {
    v_AlphaMod = min(pow(length(v_LightPos - f_Pos) / 0.1, 2.0), pow(length(vec2(v_LightPos.x, v_LightPos.y + 0.08) - f_Pos) / 0.1, 2.0));
  }

  // v_AlphaMod = pow(v_AlphaMod, pow(1.0 - max(f_Pos.y - v_LightPos.y, 0.0), 2.0));

  highp float v_HeroLightLength = pow(max(1.0 - length(v_LightPos - f_Pos) / 1.0, 0.0), 2.0);
  highp float v_Light = get_light_strength(0.2, v_LightPos, 1.2, 1.2, 1.0);
  // get_light_strength(0.05, v_LightPos, 0.8);
  // v_Light = ;

  for (int i = 0; i < u_LightsIntersectionLen; i++) {
    highp vec2 p1 = (u_WorldMatrix * vec3(data_intersection[i].x0, data_intersection[i].y0, 1.0)).xy;
    highp vec2 p2 = (u_WorldMatrix * vec3(data_intersection[i].x1, data_intersection[i].y1, 1.0)).xy;

    highp vec2 light_pos = (u_WorldMatrix * vec3(data_intersection[i].light_pos, 1.0)).xy;

    // highp float p1_light = max((1.0 - length((p1 - light_pos.xy) / 0.2)), 0.0);
    // highp float p2_light = max((1.0 - length((p2 - light_pos.xy) / 0.2)), 0.0);

//     highp float str = 0;
//     if () {
// 
//     } else 
//     mix(p1_light, p2_light, min(f_Pos.x / max(p1.x, p2.x), 1.0));

    if (u_TileId == data_intersection[i].tile_id) {
      v_Light = max(v_Light + 0.5 * pow(max((1.0 - length((p1 + p2) / 2.0 - light_pos.xy) / 0.25), 0.0) * data_intersection[i].light_radius, 2.0), v_Light);
    }
  }

  for (int i = 0; i < u_LightsLen; i++) {
    v_Light = get_light_strength(v_Light, (u_WorldMatrix * vec3(data[i].x, data[i].y, 1.0)).xy, data[i].radius, 0.8, data[i].time);
  }

  highp float v_LightCombined = ((0.00 + v_Light));
  // highp vec4 v_BlurColor = BlurColor(u_Position, u_Size, u_Offset, f_UV, v_LightCombined);

  o_Color = apply_light(v_Color, v_LightCombined, 1.0);
  // highp float bloom_luminostiy = dot(vec3(0.30, 0.59, 0.11), v_BlurColor.rgb * o_Color.a);
  // o_Color = mix(vec4(v_BlurColor.rgb, v_BlurColor.a), o_Color, 0.7);
  // o_Color = v_BlurColor;

// 
//   // o_Color = v_BlurColor;
//   // return;
// // 
//   if (pow(bloom_luminostiy, 2.0) >= 1.0 && u_ImageWidth != 0.0) {
//     
//   }
// 
// 

  if (u_Shading) {
    o_Color = vec4(o_Color.rgb * sqrt(abs(0.8 - length(vec2(0.5) - f_UV))), o_Color.a);
    if (o_Color.a < 0.8) {
      o_Color = vec4(o_Color.rgb / 2.0, o_Color.a);
    }
  }

  if (u_DisableTopLight == false) {
    o_Color = vec4(o_Color.rgb, min(o_Color.a * v_AlphaMod, o_Color.a));
  }
}
