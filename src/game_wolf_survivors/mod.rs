use std::{
    io::Read,
    time::{self, Instant},
};

use glam::Vec2;
use glium::{uniform, DrawParameters, Surface};
use quad_snd::{AudioContext, Sound};

use crate::{
    client::{
        assets::assets::Assets,
        render_loop::{self, Loop, OpenGL, Ui},
        sprite::sprite::LightData,
        tilemap::tilemap::Tile,
    },
    ecs::{prelude::*, query::query_ctx, world::World},
};

pub fn start_game() -> Result<(), Box<dyn std::error::Error>> {
    let world = world::World::new();
    render_loop::render_loop(world, &|world| world.add_system(schedule, Schedule::Update))
}

#[derive(Default)]
struct Sprite {
    id: &'static str,
    kind: &'static str,
    direction: &'static str,
    pos: Vec2,
}

impl world::Resource for Sprite {
    type Target = Components<Sprite>;
}

#[derive(Default)]
struct Enemy {
    id: &'static str,
    kind: &'static str,
    direction: &'static str,
    pos: Vec2,
    dead: Option<Instant>,
    push: Option<Vec2>,
}

impl world::Resource for Enemy {
    type Target = Components<Enemy>;
}

#[derive(Default)]
struct Light {
    pos: [f32; 2],
}

impl world::Resource for Light {
    type Target = Light;
}

#[derive(Default)]
struct Effect {
    id: &'static str,
    kind: &'static str,
    direction: &'static str,
    time: Option<Instant>,
    size: f32,
    pos: Vec2,
}

struct Audio {
    ctx: AudioContext,
}

impl Default for Audio {
    fn default() -> Self {
        Audio { ctx: AudioContext::new() }
    }
}

impl world::Resource for Audio {
    type Target = Audio;
}

impl world::Resource for Effect {
    type Target = Components<Effect>;
}
//
fn schedule(sys: &mut world::System) -> system::Return {
    system::define!(sys, read![Sprite], {
        if Sprite.len() == 0 {
            sys.add_system(init, Schedule::Update);
        }
        sys.add_system(set_buffers, Schedule::Update);
        sys.add_system(increase_time, Schedule::Update);
        sys.add_system(render_background, Schedule::Update);
        sys.add_system(render_effect_back, Schedule::Update);
        sys.add_system(render_enemy, Schedule::Update);
        sys.add_system(render_sprite, Schedule::Update);
        sys.add_system(render_effect_front, Schedule::Update);
        sys.add_system(render_ui, Schedule::Update);
    });

    system::OK
}

fn init(_sys: &mut world::System) -> system::Return {
    system::define!(_sys, write![OpenGL], write![Sprite], write![Assets], read![Audio]);

    Assets.load_tilemap("forest_map", &mut OpenGL)?;
    Assets.load_sprite("Baenor_Mastersheet_Packed", &mut OpenGL)?;
    Assets.load_sprite("ExplosionEFXFront_Packed", &mut OpenGL)?;
    Assets.load_sprite("ExplosionEFXBack_Packed", &mut OpenGL)?;
    Assets.load_sprite("DireWolf_Mastersheet_Packed", &mut OpenGL)?;
    Assets.load_sprite("BloodExplosionVFX_Packed", &mut OpenGL)?;

    Assets.load_pipeline(&mut OpenGL)?;

    // let mut wav1 = Vec::new();
    // sdl2::rwops::RWops::from_file("assets/BRPG_Assault_Adventure_Stinger.wav", "r+")?
    //     .read_to_end(&mut wav1)?;
    // let sound1 = Sound::load(&Audio.ctx, &wav1);
    // sound1.play(&Audio.ctx, Default::default());

    let mut wav2 = Vec::new();
    sdl2::rwops::RWops::from_file("assets/Eclipsed Desolation.ogg", "r+")?
        .read_to_end(&mut wav2)?;
    let sound2 = Sound::load(&Audio.ctx, &wav2);
    sound2.play(&Audio.ctx, quad_snd::PlaySoundParams { looped: true, volume: 0.3 });

    Sprite.set(
        1,
        1,
        Sprite {
            id: "Baenor_Mastersheet_Packed",
            kind: "idle",
            direction: "forward",
            pos: Vec2 { x: 1920f32 / 2f32, y: 1020f32 / 2f32 },
        },
    );

    return system::OK;
}

fn increase_time(sys: &mut world::System) -> system::Return {
    system::define!(sys, write![Assets], write![Loop]);
    Assets.pipeline.as_mut().map(|pipeline| pipeline.time += Loop.time as u32 / 100);

    return system::OK;
}

fn to_cartesian(pos: Vec2, origin: Vec2) -> Vec2 {
    let x = pos.x - origin.x;
    let y = pos.y - origin.y;

    return Vec2 {
        x: x * 45f32.to_radians().cos() - y * 45f32.to_radians().sin() + origin.x,
        y: x * 45f32.to_radians().sin() + y * 45f32.to_radians().cos() + origin.y,
    };
}

fn set_buffers(sys: &mut world::System) -> system::Return {
    system::define!(sys, write![Assets], read![Effect], read![Loop]);

    let pipeline = Assets.pipeline.as_mut().ok_or("Pipeline must existed")?;
    let mut count = 0;
    let mut total_effects = 0;
    {
        let mut mapping = pipeline.light_buffer.map();
        query!(Effect[&effect], |_| {
            if effect.time.unwrap().elapsed().as_secs_f32() < 20.0 {
                total_effects += 1;
            }
            if effect.id.starts_with("Explosion")
                && effect.time.unwrap().elapsed().as_secs_f32() < 3f32
            {
                mapping[count] = LightData {
                    x: effect.pos.x + 128f32,
                    y: effect.pos.y + 128f32,
                    time: (1f32 - effect.time.unwrap().elapsed().as_secs_f32() / 3.0).max(0.0),
                    radius: 0.4f32,
                };

                count += 1;
            }
        });
    }

    if Loop.second < 0.9 && Loop.second > 0.89 {
        println!("Total effects {}", total_effects);
    }

    pipeline.lights = count as u32;

    return system::OK;
}

fn render_background(sys: &mut world::System) -> system::Return {
    system::define!(sys, write![OpenGL], write![Assets], read![Light]);

    let (width, height) = OpenGL.display().window().size();
    let world_matrix = glam::Mat3::from_scale_angle_translation(
        Vec2 { x: 2f32 / width as f32, y: -2f32 / height as f32 },
        0f32,
        Vec2 { x: -1f32, y: 1f32 },
    );
    let transform =
        world_matrix.mul_mat3(&glam::Mat3::from_scale(Vec2 { x: width as f32, y: height as f32 }));

    let pipeline = unsafe { Assets.pipeline.as_ref().unwrap_unchecked() };
    let texture = OpenGL.textures.get("forest_map_background").unwrap().clone();
    let texture = texture
        .sampled()
        .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
        .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);
    let mat4: [[f32; 3]; 3] = unsafe { std::mem::transmute(transform) };

    OpenGL.draw(
        &pipeline.vertex_buffer,
        &pipeline.index_buffer,
        &pipeline.program,
        &uniform! {
                u_Height: false,
        u_Origin: [1.0,1.0f32],
                         u_Shading: false,
                          u_Position: [0f32, 0f32],
                          u_Size: [width as f32, height as f32],
                          u_Offset: [0f32, 0f32],
                          u_LightPos: Light.pos,
                          u_WorldMatrix: unsafe { std::mem::transmute::<_, [[f32; 3]; 3]>(world_matrix) },
                          u_DisableTopLight: true,
                          u_Lights: &(pipeline.light_buffer),
                          u_LightsLen: pipeline.lights as i32,
                          u_LightsIntersectionLen: 0,


                          u_ImageWidth: 0f32,
                          u_TileWidth: 1920f32,
                          u_TileHeight: 1080f32,
                          u_Transform: mat4,
                          u_Image: texture
                      },
        &DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        },
    )?;

    return system::OK;
}

fn render_effect_back(sys: &mut world::System) -> system::Return {
    system::define!(sys, write![OpenGL], write![Assets], write![Effect], read![Light]);

    render_specific_effect(
        "ExplosionEFXBack_Packed",
        None,
        Light.pos,
        &mut OpenGL,
        &Assets,
        &mut Effect,
    );

    return system::OK;
}

fn render_effect_front(sys: &mut world::System) -> system::Return {
    system::define!(
        sys,
        write![OpenGL],
        write![Assets],
        write![Effect],
        read![Audio],
        read![Ui],
        read![Light]
    );
    let (ref mut next_id, ref mut time, sound) = sys.state(&|| {
        let mut wav2 = Vec::new();
        sdl2::rwops::RWops::from_file("assets/Fire-4.ogg", "r+")
            .unwrap()
            .read_to_end(&mut wav2)
            .unwrap();
        (2 as u16, Instant::now(), Sound::load(&Audio.ctx, &wav2))
    })?;

    let pos = Ui.io().mouse_pos;
    if Ui.io().mouse_down[1] && time.elapsed().as_millis() > 200 {
        let origin = Vec2::from([128f32, 128f32]);
        *time = Instant::now();
        sound.play(&Audio.ctx, quad_snd::PlaySoundParams { looped: false, volume: 0.3 });

        Effect.set(
            1,
            *next_id as usize,
            Effect {
                id: "ExplosionEFXFront_Packed",
                kind: "effect",
                time: Some(Instant::now()),
                direction: "effect",
                size: 256f32,
                pos: Vec2 { x: pos[0] - origin[0], y: pos[1] - origin[1] },
            },
        );

        Effect.set(
            2,
            *next_id as usize,
            Effect {
                id: "ExplosionEFXBack_Packed",
                kind: "effect",
                time: Some(Instant::now()),
                direction: "effect",
                size: 256f32,
                pos: Vec2 { x: pos[0] - origin[0], y: pos[1] - origin[1] },
            },
        );

        *next_id += 1;
    }

    struct Circle {
        center: Vec2,
        radius: f32,
    }

    fn circle_intersection(circle_a: Circle, circle_b: Circle) -> Option<[Vec2; 2]> {
        let center_a = circle_a.center;
        let center_b = circle_b.center;
        let r_a = circle_a.radius;
        let r_b = circle_b.radius;

        let center_dx = center_b.x - center_a.x;
        let center_dy = center_b.y - center_a.y;
        let center_dist = center_dx.hypot(center_dy);

        if !(center_dist <= r_a + r_b && center_dist >= r_a - r_b) {
            return None;
        }

        let r_2 = center_dist * center_dist;
        let r_4 = r_2 * r_2;
        let a = (r_a * r_a - r_b * r_b) / (2.0 * r_2);
        let r_2_r_2 = r_a * r_a - r_b * r_b;
        let c = (2.0 * (r_a * r_a + r_b * r_b) / r_2 - r_2_r_2 * r_2_r_2 / r_4 - 1.0).sqrt();

        let fx = (center_a.x + center_b.x) / 2.0 + a * (center_b.x - center_a.x);
        let gx = c * (center_b.y - center_a.y) / 2.0;
        let ix1 = fx + gx;
        let ix2 = fx - gx;

        let fy = (center_a.y + center_b.y) / 2.0 + a * (center_b.y - center_a.y);
        let gy = c * (center_a.x - center_b.x) / 2.0;
        let iy1 = fy + gy;
        let iy2 = fy - gy;

        Some([Vec2 { x: ix1, y: iy1 }, Vec2 { x: ix2, y: iy2 }])
    }

    let Assets { ref mut pipeline, ref maps, .. } = *Assets;
// 
    // let pipeline = pipeline.as_mut().ok_or("Pipeline must existed")?;
    // let mut mapping = pipeline.light_intersection_buffer.map();
    // let map = maps.get("forest_map").ok_or("Unknown map")?;
    // let invalid_id = map.atlas.iter().position(|image| image.image == "Grid").unwrap() as u32;
    // let mut idx = 0;
    // let mut tile_id = 0;
    // query!(Effect[&effect], |_| {
    //     let duration = effect.time.unwrap().elapsed().as_secs_f32();
    //     if effect.id.starts_with("Explosion") && duration < 3.0 {
    //         for tile in map.tiles.iter() {
    //             if tile.atlas_id == invalid_id {
    //                 continue;
    //             }
    //             let tile_circle = Circle {
    //                 radius: tile.width / 2f32,
    //                 center: Vec2 {
    //                     x: tile.x + tile.width / 2f32,
    //                     y: tile.y + tile.height - tile.width / 2f32,
    //                 },
    //             };
    //             let light_circle = Circle {
    //                 radius: effect.size / 2f32,
    //                 center: Vec2 {
    //                     x: effect.pos.x + effect.size / 2f32,
    //                     y: effect.pos.y + effect.size / 2f32,
    //                 },
    //             };
    //             if let Some(intersection) = circle_intersection(tile_circle, light_circle) {
    //                 if intersection[0] != intersection[1] {
    //                     mapping[idx].x0 = intersection[0].x;
    //                     mapping[idx].y0 = intersection[0].y;
    //                     mapping[idx].x1 = intersection[1].x;
    //                     mapping[idx].y1 = intersection[1].y;
    //                     mapping[idx].light_pos =
    //                         [effect.pos.x + effect.size / 2f32, effect.pos.y + effect.size / 2f32];
    //                     mapping[idx].tile_id = tile_id;
    //                     mapping[idx].light_radius =
    //                         (1f32 - effect.time.unwrap().elapsed().as_secs_f32() / 3.0).max(0.0);
    //                     idx = (idx + 1) % 64;
    //                 }
    //             }
    //             tile_id += 1;
    //         }
    //     }
    // });
    // drop(mapping);
    // pipeline.lights_intersection = idx as u32;

    render_specific_effect(
        "ExplosionEFXFront_Packed",
        None,
        Light.pos,
        &mut OpenGL,
        &Assets,
        &mut Effect,
    );

    return system::OK;
}
//
fn render_specific_effect(
    prefix: &'static str,
    max_frame: Option<u8>,
    light_pos: [f32; 2],
    OpenGL: &mut OpenGL,
    Assets: &Assets,
    Effect: &mut View<Components<Effect>, write>,
) {
    let (width, height) = OpenGL.display().window().size();

    let world_matrix = glam::Mat3::from_scale_angle_translation(
        Vec2 { x: 2f32 / width as f32, y: -2f32 / height as f32 },
        0f32,
        Vec2 { x: -1f32, y: 1f32 },
    );

    let params = ();
    let _: Option<_> = try {
        query_ctx!(params, Effect[&mut effect], |effect_id| {
            if !effect.id.starts_with(prefix) {
                continue;
            }

            let time = effect.time.unwrap_or_else(|| {
                panic!("{effect_id}");
            });
            let modifier = if effect.id.starts_with("Blood") { 2.0 } else { 1.0 };

            if time.elapsed().as_secs_f32() * modifier >= 1.0 {
                if max_frame.is_none() || time.elapsed().as_secs_f32() > 20.0 {
                    continue;
                }
            }

            let db = Assets.sprites.get(effect.id)?;
            let texture = OpenGL.textures.get(effect.id)?;

            let animations = db["tiles"]["effect"]["effect"].as_array()?;
            let mut frame_idx =
                time.elapsed().as_secs_f32() / (1f32 / modifier / animations.len() as f32);
            if let Some(max_frame) = max_frame {
                if time.elapsed().as_secs_f32() < 1.0f32 {
                    frame_idx = frame_idx.min(max_frame as f32);
                } else {
                    frame_idx = max_frame as f32;
                }
            }

            let current_frame = animations.get(frame_idx as usize)?;

            let transform = world_matrix.mul_mat3(&glam::Mat3::from_scale_angle_translation(
                Vec2 { x: effect.size, y: effect.size },
                0f32,
                Vec2 { x: effect.pos.x, y: effect.pos.y },
            ));

            let mat4: [[f32; 3]; 3] = unsafe { std::mem::transmute(transform) };
            let texture = texture.clone();
            let texture = texture
                .sampled()
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);

            let pipeline = unsafe { Assets.pipeline.as_ref().unwrap_unchecked() };

            OpenGL
            .draw(
                &pipeline.vertex_buffer,
                &pipeline.index_buffer,
                &pipeline.program,
                &uniform! {
  u_Height: false,
u_Origin: [1.0,1.0f32],
                    u_Shading: true,
                    u_Position: [current_frame["x"].as_f32()?, current_frame["y"].as_f32()?],
                    u_Size: [current_frame["width"].as_f32()?, current_frame["height"].as_f32()?],
                    u_Offset: [current_frame["relative_x"].as_f32()?, current_frame["relative_y"].as_f32()?],
                    u_LightPos: light_pos,
                    u_WorldMatrix: unsafe { std::mem::transmute::<_, [[f32; 3]; 3]>(world_matrix) },
                    u_DisableTopLight: true,
                    u_Light: &(pipeline.light_buffer),
                    u_LightsLen: pipeline.lights as i32,
                    u_LightsIntersectionLen: 0,



                    u_ImageWidth: db["size"].as_f32()?,
                    u_TileWidth: db["tile_width"].as_f32()?,
                    u_TileHeight: db["tile_width"].as_f32()?,
                    u_Transform: mat4,
                    u_Image: texture
                },
                &DrawParameters {
                    blend: glium::Blend::alpha_blending(),
                    ..Default::default()
                },
            )
            .ok()?;
        });
    };
}

fn render_sprite(sys: &mut world::System) -> system::Return {
    system::define!(
        sys,
        write![OpenGL],
        write![Loop],
        write![Sprite],
        read![Assets],
        read![Ui],
        write![Light]
    );

    let mut animation_transition = sys.state(&|| Instant::now())?;

    let (width, height) = OpenGL.display().window().size();

    let world_matrix = glam::Mat3::from_scale_angle_translation(
        Vec2 { x: 2f32 / width as f32, y: -2f32 / height as f32 },
        0f32,
        Vec2 { x: -1f32, y: 1f32 },
    );

    let origin = Vec2::from([64f32, 96f32]);
    let position = Vec2::from(Ui.io().mouse_pos);

    // if Ui.io().mouse_down[1] {
    //     Sprite.set(id, data)
    // }

    fn intersects(character_pos: &Vec2, tile: &Tile) -> bool {
        let new_size = 48f32;
        let circle_radius = 10f32;
        let circle_pos =
            to_cartesian(*character_pos, Vec2 { x: 1920f32 / 2f32, y: 1080f32 / 2f32 });
        let rect_pos = to_cartesian(
            Vec2 { x: (tile.x - tile.width / 2f32), y: (tile.y - tile.height) },
            Vec2 { x: 1920f32 / 2f32, y: 1080f32 / 2f32 },
        );

        let circle_distance =
            Vec2 { x: (circle_pos.x - rect_pos.x).abs(), y: (circle_pos.y - rect_pos.y).abs() };

        if circle_distance.x > (new_size / 2f32 + circle_radius) {
            return false;
        }

        if circle_distance.y > (new_size / 2f32 + circle_radius) {
            return false;
        }

        if circle_distance.x <= (new_size / 2f32) {
            return true;
        }

        if circle_distance.y <= (new_size / 2f32) {
            return true;
        }

        let corner_distance = (circle_distance.x - new_size / 2f32).powi(2)
            + (circle_distance.y - new_size / 2f32).powi(2);

        return corner_distance < circle_radius.powi(2);
    }

    let texture = OpenGL.textures.get("forest_map").unwrap().clone();
    let map = Assets.maps.get("forest_map").ok_or("Unknown map")?;
    let invalid_id = map.atlas.iter().position(|image| image.image == "Grid").unwrap() as u32;
    let flat_id = map.atlas.iter().position(|image| image.image == "Column_1").unwrap();

    query_try!(Sprite[&mut sprite], |_| {
        let db = Assets.sprites.get(sprite.id)?;
        let texture = OpenGL.textures.get(sprite.id)?;

        if Ui.io().mouse_down[0] {
            let target = position - (sprite.pos + origin);
            let direction = target.normalize();

            let mut cant_walk_in_wall: i32 = -1;

            for (pos, tile) in map.tiles.iter().enumerate() {
                if tile.atlas_id != invalid_id {
                    continue;
                }

                if intersects(&(sprite.pos + direction), tile) {
                    cant_walk_in_wall = pos as i32;
                    break;
                }
            }

            if target.length() >= 1f32 {
                if target.length() >= 60f32
                    && cant_walk_in_wall < 0
                    && animation_transition.elapsed().as_millis() > 100
                {
                    sprite.kind = "run";
                    sprite.pos += direction;
                } else {
                    sprite.kind = "walk";
                    if cant_walk_in_wall < 0 {
                        sprite.pos += direction;
                    } else {
                        *animation_transition = Instant::now();
                        let tile = unsafe { map.tiles.get_unchecked(cant_walk_in_wall as usize) };
                        let y_mv = Vec2 { x: 0f32, y: direction.y };
                        let x_mv = Vec2 { x: direction.x, y: 0f32 };

                        if !intersects(&(sprite.pos + y_mv), tile) {
                            sprite.pos += y_mv;
                        }

                        if !intersects(&(sprite.pos + x_mv), tile) {
                            sprite.pos += x_mv;
                        }
                    }
                }
            } else {
                sprite.kind = "idle";
            }

            Light.pos = [sprite.pos.x + origin.x, sprite.pos.y + origin.y];

            let angle = (direction).angle_between(Vec2 { x: 1f32, y: 0f32 }).to_degrees();
            let angle = if angle > 0f32 { angle } else { 360f32 - angle.abs() };
            sprite.direction = match angle as i32 {
                0..=22 => "right",
                23..=67 => "backward_right",
                68..=112 => "backward",
                113..=157 => "backward_left",
                158..=202 => "left",
                203..=247 => "forward_left",
                248..=292 => "forward",
                293..=337 => "forward_right",
                338..=360 => "right",
                _ => unimplemented!("Unknow direction"),
            };
        } else {
            sprite.kind = "idle";
        };

        let animations = db["tiles"][sprite.kind][sprite.direction].as_array()?;

        let frame_idx = Loop.second / (1f32 / 1.3 / animations.len() as f32);

        let current_frame = animations.get(frame_idx as usize % animations.len())?;

        let transform = world_matrix.mul_mat3(&glam::Mat3::from_scale_angle_translation(
            Vec2 { x: 128f32, y: 128f32 },
            0f32,
            Vec2 { x: sprite.pos.x, y: sprite.pos.y },
        ));

        let mat4: [[f32; 3]; 3] = unsafe { std::mem::transmute(transform) };
        let texture = texture.clone();
        let texture = texture
            .sampled()
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);

        let pipeline = unsafe { Assets.pipeline.as_ref().unwrap_unchecked() };

        OpenGL
            .draw(
                &pipeline.vertex_buffer,
                &pipeline.index_buffer,
                &pipeline.program,
                &uniform! {
  u_Height: false,
u_Origin: [1.0,1.0f32],
                    u_Shading: false,
                    u_Position: [current_frame["x"].as_f32()?, current_frame["y"].as_f32()?],
                    u_Size: [current_frame["width"].as_f32()?, current_frame["height"].as_f32()?],
                    u_Offset: [current_frame["relative_x"].as_f32()?, current_frame["relative_y"].as_f32()?],
                    u_LightPos: Light.pos,
                    u_WorldMatrix: unsafe { std::mem::transmute::<_, [[f32; 3]; 3]>(world_matrix) },
                    u_DisableTopLight: true,
                    u_Lights: &(pipeline.light_buffer),
                    u_LightsLen: pipeline.lights as i32,
                    u_LightsIntersectionLen: 0,


                    u_ImageWidth: db["size"].as_f32()?,
                    u_TileWidth: db["tile_width"].as_f32()?,
                    u_TileHeight: db["tile_width"].as_f32()?,
                    u_Transform: mat4,
                    u_Image: texture
                },
                &DrawParameters {
                    blend: glium::Blend::alpha_blending(),
                    ..Default::default()
                },
            )
            .ok()?;
    });

    let mut tile_id = 0;
    for tile in map.tiles.iter() {
        if tile.atlas_id == invalid_id {
            continue;
        }
        let atlas = unsafe { map.atlas.get_unchecked(tile.atlas_id as usize) };

        let transform = world_matrix.mul_mat3(&glam::Mat3::from_scale_angle_translation(
            Vec2 { x: tile.width + 1f32, y: tile.height + 1f32 },
            0.0,
            Vec2 { x: tile.x - 1f32, y: tile.y - 1f32 },
        ));

        let mat4: [[f32; 3]; 3] = unsafe { std::mem::transmute(transform) };
        let image = texture
            .sampled()
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);
        let pipeline = unsafe { Assets.pipeline.as_ref().unwrap_unchecked() };

        let uniforms: Option<_> = try {
            uniform! {
                u_Height: tile.atlas_id == flat_id as u32,
                u_Origin: [tile.x + tile.width / 2f32, tile.y + tile.height * 0.8],
                u_Position: [atlas.x, atlas.y],
                u_Size: [atlas.width, atlas.height],
                u_Offset: [atlas.relative_x, atlas.relative_y],
                u_LightPos: Light.pos,
                u_WorldMatrix: unsafe { std::mem::transmute::<_, [[f32; 3]; 3]>(world_matrix) },
                u_DisableTopLight: false,
                u_Lights: &(pipeline.light_buffer),
                u_LightsLen: pipeline.lights as i32,

                u_LightsIntersection: &(pipeline.light_intersection_buffer),
                u_LightsIntersectionLen: pipeline.lights_intersection as i32,
                u_TileId: tile_id,

                u_ImageWidth: 2048f32,
                u_TileWidth: tile.width,
                u_TileHeight: tile.height,

                u_Transform: mat4,
                u_Image: image
            }
        };

        tile_id += 1;

        OpenGL.draw(
            &pipeline.vertex_buffer,
            &pipeline.index_buffer,
            &pipeline.program,
            &uniforms.ok_or("Can't create uniforms from JSON")?,
            &DrawParameters { blend: glium::Blend::alpha_blending(), ..Default::default() },
        )?;
    }

    return system::OK;
}

fn render_enemy(sys: &mut world::System) -> system::Return {
    system::define!(
        sys,
        write![OpenGL],
        write![Enemy],
        read![Sprite],
        write![Effect],
        read![Assets],
        read![Loop],
        read![Audio],
        read![Light]
    );
    use nanorand::{Rng, WyRand};

    let (ref mut random, sound) = sys.state(&|| {
        let mut wav2 = Vec::new();
        sdl2::rwops::RWops::from_file("assets/Warg-Death-2.ogg", "r+")
            .unwrap()
            .read_to_end(&mut wav2)
            .unwrap();
        (Instant::now(), Sound::load(&Audio.ctx, &wav2))
    })?;

    let origin = Vec2::from([64f32, 96f32]);

    if random.elapsed().as_millis() > 500 {
        let mut rng = WyRand::new();
        let x = rng.generate_range(0_u64..=1920);
        let y = rng.generate_range(0_u64..=1024);

        Enemy.set(
            2,
            rng.generate_range(2..=4000),
            Enemy {
                id: "DireWolf_Mastersheet_Packed",
                kind: "run",
                direction: "forward",
                pos: Vec2 { x: x as f32 - origin[0], y: y as f32 - origin[1] },
                dead: None,
                push: None,
            },
        );

        *random = Instant::now();
    }

    let mut position: Vec2 = Vec2::ZERO;
    query!(Sprite[f! {pos}], |_| { position = pos + origin });
    let (width, height) = OpenGL.display().window().size();
    let world_matrix = glam::Mat3::from_scale_angle_translation(
        Vec2 { x: 2f32 / width as f32, y: -2f32 / height as f32 },
        0f32,
        Vec2 { x: -1f32, y: 1f32 },
    );

    render_specific_effect("Blood", Some(14), Light.pos, &mut OpenGL, &Assets, &mut Effect);

    let mut total_enemies = 0;
    
        query_try!(Enemy[&mut sprite], |id| {
            let db = Assets.sprites.get(sprite.id)?;
            let texture = OpenGL.textures.get(sprite.id)?;
    
            if sprite.dead.is_some()
                && unsafe { sprite.dead.unwrap_unchecked() }.elapsed().as_secs_f32() > 20.0
            {
                continue;
            }
    
            if sprite.dead.is_none() {
                let target = position - (sprite.pos + origin);
                let direction = target.normalize() * 1.25;
    
                sprite.kind = "run";
                sprite.pos += direction;
    
                let angle = (direction).angle_between(Vec2 { x: 1f32, y: 0f32 }).to_degrees();
                let angle = if angle > 0f32 { angle } else { 360f32 - angle.abs() };
                sprite.direction = match angle as i32 {
                    0..=22 => "right",
                    23..=67 => "backward_right",
                    68..=112 => "backward",
                    113..=157 => "backward_left",
                    158..=202 => "left",
                    203..=247 => "forward_left",
                    248..=292 => "forward",
                    293..=337 => "forward_right",
                    338..=360 => "right",
                    _ => unimplemented!("Unknow direction"),
                };

                query!(Sprite[&hero], |_| {
                    let unit_radius = 20f32;
                    let hero_pos =
                        to_cartesian(hero.pos, Vec2 { x: 1920f32 / 2f32, y: 1080f32 / 2f32 });
                    let character_pos =
                        to_cartesian(sprite.pos, Vec2 { x: 1920f32 / 2f32, y: 1080f32 / 2f32 });
                    let intersect_with_hero = ((hero_pos.x - character_pos.x).powi(2)
                        + (hero_pos.y - character_pos.y).powi(2))
                        <= (unit_radius + unit_radius).powi(2);
    
                    if intersect_with_hero {
                        sprite.push = Some(target.normalize() * -10f32);
                        sprite.dead = Some(Instant::now());
                        sprite.kind = "death";
                        sound
                            .play(&Audio.ctx, quad_snd::PlaySoundParams { looped: false, volume: 0.3 });
                    }

                    query!(Effect[&effect], |_| {
                        let elapsed = unsafe { effect.time.unwrap_unchecked() }.elapsed().as_secs_f32();
                        if elapsed > 0.4 && elapsed < 0.8f32 && !effect.id.starts_with("Blood") {
                            let effect_radius = 50f32;
                            let effect_pos = to_cartesian(
                                Vec2 { x: effect.pos.x + 64f32, y: effect.pos.y + 64f32 },
                                Vec2 { x: 1920f32 / 2f32, y: 1080f32 / 2f32 },
                            );
    
                            let intersect_with_effect = ((effect_pos.x - character_pos.x).powi(2)
                                + (effect_pos.y - character_pos.y).powi(2))
                                <= (effect_radius + unit_radius).powi(2);
    
                            if intersect_with_effect {
                                let mut rng = WyRand::new();
                                let x = rng.generate_range(0u8..=20);
                                let y = rng.generate_range(0u8..=20);
                                sprite.dead = Some(Instant::now());
                                sprite.kind = "death";
                                sound.play(
                                    &Audio.ctx,
                                    quad_snd::PlaySoundParams { looped: false, volume: 0.3 },
                                );
                                unsafe {
                                    Effect.try_set(
                                        5,
                                        id,
                                        Effect {
                                            id: "BloodExplosionVFX_Packed",
                                            kind: "effect",
                                            time: Some(Instant::now()),
                                            direction: "effect",
                                            pos: Vec2 {
                                                x: sprite.pos.x as f32 + x as f32,
                                                y: sprite.pos.y as f32 + y as f32,
                                            },
                                            size: 128f32,
                                        },
                                    )
                                };
                            }
                        }
                    });
                });
            }

            if let Some(push_strength) = sprite.push {
                if push_strength.length() > 1.05 {
                    sprite.pos += push_strength;
                    let next_push = push_strength * 0.85;
                    sprite.push = Some(next_push);
                    if next_push.length() > 1.05 {
                        Effect.set(
                            5,
                            id,
                            Effect {
                                id: "BloodExplosionVFX_Packed",
                                kind: "effect",
                                time: Some(Instant::now()),
                                direction: "effect",
                                pos: Vec2 { x: sprite.pos.x as f32, y: sprite.pos.y as f32 },
                                size: 128f32,
                            },
                        );
                    }
                }
            }
    
            if sprite.dead.is_some() && sprite.push.is_none() {
                continue;
            }
            total_enemies += 1;
    
            let animations = db["tiles"][sprite.kind][sprite.direction].as_array()?;
    
            let frame_idx = match sprite.dead {
                Some(time) if (time.elapsed().as_secs_f32()) < 0.64f32 => time.elapsed().as_secs_f32(),
                Some(_) => 0.63f32,
                _ => Loop.second,
            } / (0.64f32 / animations.len() as f32);
    
            let current_frame = animations.get(frame_idx as usize % animations.len())?;
    
            let transform = world_matrix.mul_mat3(&glam::Mat3::from_scale_angle_translation(
                Vec2 { x: 128f32, y: 128f32 },
                0f32,
                Vec2 { x: sprite.pos.x, y: sprite.pos.y },
            ));
    
            let mat4: [[f32; 3]; 3] = unsafe { std::mem::transmute(transform) };
            let texture = texture.clone();
            let texture = texture
                .sampled()
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);
    
            let pipeline = unsafe { Assets.pipeline.as_ref().unwrap_unchecked() };
    
            OpenGL
                .draw(
                    &pipeline.vertex_buffer,
                    &pipeline.index_buffer,
                    &pipeline.program,
                    &uniform! {
                        u_Height: false,
                        u_Origin: [1.0,1.0f32],
                        u_Shading: false,
                        u_Position: [current_frame["x"].as_f32()?, current_frame["y"].as_f32()?],
                        u_Size: [current_frame["width"].as_f32()?, current_frame["height"].as_f32()?],
                        u_Offset: [current_frame["relative_x"].as_f32()?, current_frame["relative_y"].as_f32()?],
                        u_LightPos: Light.pos,
                        u_WorldMatrix: unsafe { std::mem::transmute::<_, [[f32; 3]; 3]>(world_matrix) },
                        u_DisableTopLight: true,
                        u_Lights: &(pipeline.light_buffer),
                        u_LightsLen: pipeline.lights as i32,
                        u_LightsIntersectionLen: 0,
    
    
                        u_ImageWidth: db["size"].as_f32()?,
                        u_TileWidth: db["tile_width"].as_f32()?,
                        u_TileHeight: db["tile_width"].as_f32()?,
                        u_Transform: mat4,
                        u_Image: texture
                    },
                    &DrawParameters {
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .ok()?;
        });
    
        if Loop.second < 0.9 && Loop.second > 0.89 {
            println!("Total enemies sprites: {}", total_enemies);
        }

    return system::OK;
}

fn render_ui(sys: &mut world::System) -> system::Return {
    system::define!(sys, write![Ui], read![Loop]);

    let fps = sys.state(&|| (String::new(), 0u8, time::Instant::now()))?;

    if fps.2.elapsed().as_millis() >= 1000 {
        use std::fmt::Write;

        fps.0.truncate(0);
        writeln!(&mut fps.0, "Current FPS: {}", fps.1)?;
        writeln!(&mut fps.0, "Current Time: {}micro", Loop.time)?;
        writeln!(&mut fps.0, "Current Pos: {:?}", Ui.io().mouse_pos)?;

        println!("{}", &mut fps.0);

        fps.1 = 0;
        fps.2 = time::Instant::now();
    }

    Ui.window("FPS").size([200.0, 100.0], ::imgui::Condition::FirstUseEver).build(|| {
        let [width, _] = Ui.window_size();
        let [text_x, _] = Ui.calc_text_size(&fps.0);
        let [_, cursor_y] = Ui.cursor_pos();

        Ui.set_cursor_pos([(width - text_x) * 0.5, cursor_y]);
        Ui.text(&fps.0);

        let [_, cursor_y] = Ui.cursor_pos();
        Ui.set_cursor_pos([(width - text_x / 2.0) * 0.5, cursor_y]);
        Ui.button("Exit");
        if Ui.is_item_clicked() {
            panic!("exit clicked");
        }
    });

    fps.1 += 1;

    return system::OK;
}
