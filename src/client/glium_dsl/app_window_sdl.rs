use sdl2::Sdl;

use super::glium_sdl2::{DisplayBuild, SDL2Facade};

pub enum AppUnits {
    #[allow(dead_code)]
    DeviceIndependent,
    HardwarePixels,
}

impl AppUnits {
    // #[allow(unused_parens)]
    // pub fn hw_to_units(&self, pixel_ratio: f32, value: f32) -> f32 {
    //     match self {
    //         AppUnits::DeviceIndependent => (value / pixel_ratio),
    //         AppUnits::HardwarePixels => value,
    //     }
    // }

    #[allow(unused_parens)]
    pub fn units_to_hw(&self, pixel_ratio: f32, value: f32) -> f32 {
        match self {
            AppUnits::DeviceIndependent => (value * pixel_ratio),
            AppUnits::HardwarePixels => value,
        }
    }
}

pub struct AppWindow {
    pub display: SDL2Facade,
    pub sdl: Sdl,
}

impl Default for AppWindow {
    fn default() -> Self {
        panic!("AppWindow doesn't support default instantination");
    }
}

impl AppWindow {
    pub fn new(title: &str, width: f32, height: f32) -> Self {
        let sdl = sdl2::init().unwrap();
        let video_subsystem = sdl.video().unwrap();

        let mut pixel_ratio = 1.0;
        let dpi = video_subsystem.display_dpi(0).unwrap().0;
        if dpi > 160.0 {
            pixel_ratio = dpi / 160.0;
        }

        let units = AppUnits::HardwarePixels;
        let hw_size =
            (units.units_to_hw(pixel_ratio, width), units.units_to_hw(pixel_ratio, height));

        let display = video_subsystem
            .window(title, hw_size.0 as u32, hw_size.1 as u32)
            .opengl()
            .resizable()
            .build_glium()
            .unwrap();

        Self { display, sdl }
    }
}
