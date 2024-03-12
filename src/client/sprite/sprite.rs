use glium::{implement_uniform_block, program};

#[derive(Copy, Clone)]
#[allow(non_snake_case)]
pub struct Vertex {
    pub a_Position: [f32; 2],
    pub a_UV: [f32; 2],
}
glium::implement_vertex!(Vertex, a_Position, a_UV);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LightData {
    pub x: f32,
    pub y: f32,
    pub time: f32,
    pub radius: f32,
}
implement_uniform_block!(LightData, x, y, time, radius);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LightIntersection {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub light_pos: [f32; 2],
    pub light_radius: f32,
    pub tile_id: i32,
}
implement_uniform_block!(LightIntersection, x0, y0, x1, y1, light_pos, light_radius, tile_id);

pub struct SpritePipeline {
    pub vertex_buffer: glium::VertexBuffer<Vertex>,
    pub index_buffer: glium::IndexBuffer<u16>,
    pub program: glium::Program,
    pub time: u32,
    pub light_buffer: glium::uniforms::UniformBuffer<[LightData; 64]>,
    pub light_intersection_buffer: glium::uniforms::UniformBuffer<[LightIntersection; 64]>,
    pub lights: u32,
    pub lights_intersection: u32,
}

impl SpritePipeline {
    pub fn new(
        facade: &impl glium::backend::Facade,
    ) -> Result<SpritePipeline, Box<dyn std::error::Error>> {
        let light_buffer: glium::uniforms::UniformBuffer<[LightData; 64]> =
            glium::uniforms::UniformBuffer::empty(facade).unwrap();

        let light_intersection_buffer: glium::uniforms::UniformBuffer<[LightIntersection; 64]> =
            glium::uniforms::UniformBuffer::empty(facade).unwrap();

        let vertex_buffer = glium::VertexBuffer::new(
            facade,
            &[
                Vertex { a_Position: [0.0, 1.0], a_UV: [0.0, 1.0] },
                Vertex { a_Position: [0.0, 0.0], a_UV: [0.0, 0.0] },
                Vertex { a_Position: [1.0, 0.0], a_UV: [1.0, 0.0] },
                Vertex { a_Position: [1.0, 1.0], a_UV: [1.0, 1.0] },
            ],
        )?;

        let index_buffer = glium::IndexBuffer::new(
            facade,
            glium::index::PrimitiveType::TriangleStrip,
            &[0u16, 1u16, 3u16, 2u16],
        )?;

        let vertex_str = include_str!("shader/sprite.vert");
        let fragment_str = include_str!("shader/sprite.frag");

        let program = program!(
            facade,
            330 => {
                vertex: vertex_str.replace("#version 320 es", "#version 330").leak(),
                fragment: fragment_str.replace("#version 320 es", "#version 330").leak(),
            },
            320 es => {
                vertex: vertex_str,
                fragment: fragment_str,
            },
        )?;

        println!("WORKS 2");

        Ok(SpritePipeline {
            program,
            light_buffer,
            light_intersection_buffer,
            index_buffer,
            vertex_buffer,
            time: 0,
            lights: 0,
            lights_intersection: 0,
        })
    }
}
