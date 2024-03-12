use nanoserde::{DeJson, SerJson};

#[derive(Debug, DeJson, SerJson)]
pub struct Atlas {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub relative_x: f32,
    pub relative_y: f32,
    pub image: String,
}

#[derive(Debug, DeJson, SerJson)]
pub struct Tile {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub atlas_id: u32,
}

#[derive(Debug, DeJson, SerJson)]
pub struct Tilemap {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub atlas: Vec<Atlas>,
    pub tiles: Vec<Tile>,
}
