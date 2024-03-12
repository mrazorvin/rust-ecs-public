use std::error::Error;
use std::io::Read;
use std::rc::Rc;

use basis_universal::{TranscodeParameters, Transcoder, TranscoderTextureFormat};
use glium::texture::CompressedTexture2d;
use hashlink::LinkedHashMap;
use nanoserde::DeJson;

use crate::client::render_loop::OpenGL;
use crate::client::sprite::sprite::SpritePipeline;
use crate::client::tilemap::tilemap::Tilemap;
use crate::ecs::prelude::*;

pub struct Assets {
    pub sprites: LinkedHashMap<String, jzon::JsonValue>,
    pub maps: LinkedHashMap<String, Tilemap>,
    pub pipeline: Option<SpritePipeline>,
}

impl Assets {
    pub fn read_json(path: &str) -> Result<String, Box<dyn Error>> {
        let mut file = String::new();
        sdl2::rwops::RWops::from_file(path, "r+")?.read_to_string(&mut file)?;
        Ok(file)
    }

    pub fn load_texture<'a, 'b, 'c>(
        &mut self,
        basis_path: &'a str,
        key: &'c str,
        opengl: &'b mut OpenGL,
    ) -> Result<Rc<CompressedTexture2d>, Box<dyn Error>> {
        let mut transcoder = Transcoder::new();
        let mut basis_file = Vec::new();
        sdl2::rwops::RWops::from_file(format!("{basis_path}"), "r+")?.read_to_end(&mut basis_file)?;
        transcoder.prepare_transcoding(&basis_file).unwrap();
        let image = transcoder
            .transcode_image_level(
                &basis_file,
                #[cfg(target_os = "android")]
                TranscoderTextureFormat::ASTC_4x4_RGBA,
                #[cfg(not(target_os = "android"))]
                TranscoderTextureFormat::BC3_RGBA,
                TranscodeParameters {
                    image_index: 0,
                    level_index: 0,
                    ..Default::default()
                },
            )
            .unwrap();

        transcoder.end_transcoding();

        let description = transcoder.image_level_description(&basis_file, 0, 0).unwrap();

        let texture = std::rc::Rc::new(glium::texture::CompressedTexture2d::with_compressed_data(
            opengl.display(),
            &image,
            description.original_width,
            description.original_height,
            #[cfg(not(target_os = "android"))]
            glium::texture::CompressedFormat::S3tcDxt5Alpha,
            #[cfg(target_os = "android")]
            glium::texture::CompressedFormat::Astc4x4,
            glium::texture::CompressedMipmapsOption::NoMipmap,
        )?);

        opengl.textures.insert(String::from(key), texture.clone());

        Ok(texture)
    }

    pub fn load_sprite(
        &mut self,
        key: &str,
        opengl: &mut OpenGL,
    ) -> Result<Rc<CompressedTexture2d>, Box<dyn Error>> {
        let texture = self.load_texture(&format!("assets/{key}.basis"), key, opengl)?;

        self.sprites.insert(
            String::from(key),
            // TODO replace assets with something more generic
            jzon::parse(&Assets::read_json(&format!("assets/{key}.json"))?)?,
        );

        Ok(texture)
    }

    pub fn load_tilemap(&mut self, key: &str, opengl: &mut OpenGL) -> Result<(), Box<dyn Error>> {
        let map = Tilemap::deserialize_json(&Assets::read_json(&format!("assets/{key}/info.json"))?)?;

        self.load_texture(&format!("assets/{key}/packed_map.basis"), key, opengl)?;
        self.load_texture(
            &format!("assets/{key}/background.basis"),
            &format!("{key}_background"),
            opengl,
        )?;

        self.maps.insert(String::from(key), map);

        Ok(())
    }

    pub fn load_pipeline(&mut self, opengl: &mut OpenGL) -> Result<(), Box<dyn Error>> {
        self.pipeline = Some(SpritePipeline::new(opengl.display())?);
        Ok(())
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self {
            sprites: Default::default(),
            maps: Default::default(),
            pipeline: None,
        }
    }
}

impl world::Resource for Assets {
    type Target = Assets;
}
