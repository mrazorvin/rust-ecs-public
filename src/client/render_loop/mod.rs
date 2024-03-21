use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Instant,
};

use glium::Surface;

use crate::{
    client::glium_dsl::app_window_sdl::AppWindow,
    ecs::world::{self, World},
};

use super::{
    glium_dsl::glium_sdl2::SDL2Facade,
    imgui_glium_sdl::{glium_imgui_renderer, sld2_imgui_support},
};

pub fn render_loop(
    mut world: crate::ecs::world::World,
    cb: &dyn Fn(&mut World) -> (),
    cb_frame_end: &dyn Fn(&mut World) -> (),
) -> Result<(), Box<dyn std::error::Error>> {
    let app = AppWindow::new("Game", 1920.0, 1080.0);

    // #region ### imgui setup

    let mut imgui = ::imgui::Context::create();
    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);
    imgui.fonts().add_font(&[::imgui::FontSource::DefaultFontData { config: None }]);
    let mut imgui_platform = sld2_imgui_support::SdlPlatform::init(&mut imgui);
    let mut imgui_renderer = glium_imgui_renderer::Renderer::init(&mut imgui, &app.display)?;
    let imgui_renderer_ptr = MutPtrLifetimeGuard::new(&mut imgui_renderer);
    // #endregion

    // it's ok to have init this value here  because no-one can access it
    // until first execution call
    let (opengl, _) = world.set_unique(OpenGL {
        frame: None,
        display: std::ptr::null(),
        textures: HashMap::new(),
    });

    let (ui, _) = world.set_unique(Ui {
        ui: std::ptr::null_mut(),
        renderer: imgui_renderer_ptr.ptr,
        textures: HashMap::new(),
    });

    let loop_res = world.add_unique(Loop { time: 0, second: 0.0 })?;

    let mut event_pump = app.sdl.event_pump()?;
    let mut second = Instant::now();

    loop {
        let mut frame = app.display.draw();
        frame.clear_all((0.05, 0.05, 0.05, 0.0), 1.0, 1);
        imgui_platform.prepare_frame(&mut imgui, app.display.window(), &event_pump);
        let frame_duration = Instant::now();

        // raw pointer prevents `&mut` aliasing problem, when UI injected as resource
        //
        // safety: it's ok to use this resource because:
        //         1. it's will over-live `execute()` call
        //         2. it's impl `!Send`` i.e it will be used only on main thread
        let ui_ptr = MutPtrLifetimeGuard::new(imgui.new_frame());
        let display_ptr = SharedPtrLifetimeGuard::new(&app.display);
        unsafe { &mut *ui }.ui = ui_ptr.ptr;
        unsafe { &mut *opengl }.frame = Some(frame);
        unsafe { &mut *opengl }.display = display_ptr.ptr;

        cb(&mut world);

        world.execute();

        drop(ui_ptr);
        drop(display_ptr);

        let draw_data = imgui.render();
        let passed = second.elapsed().as_secs_f32();
        if passed >= 1f32 {
            second = Instant::now();
        }

        unsafe { &mut *loop_res }.second = passed;

        #[cfg(not(target_os = "android"))]
        {
            unsafe { &mut *loop_res }.time = frame_duration.elapsed().as_micros();
        }

        let render_result = if draw_data.total_vtx_count != 0 {
            unsafe { &mut *imgui_renderer_ptr.ptr }.render(unsafe { &mut *opengl }, draw_data)
        } else {
            Ok(())
        };
        // buffer swap, on android may take ~2ms
        unsafe { &mut *opengl }.set_finish()?;
        // all results must be unwrapped after `frame.set_finish()` call, otherwise
        // instead of real error programm will panic! with `frame not finished after dropping` error
        render_result?;

        #[cfg(target_os = "android")]
        {
            unsafe { &mut *loop_res }.time = frame_duration.elapsed().as_micros();
        }

        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => return Ok(()),
                ref event => imgui_platform.handle_event(&mut imgui, &event),
            };
        }

        cb_frame_end(&mut world);
    }
}

// #region ### lifetime guard & pinning for pointer value

struct MutPtrLifetimeGuard<'a, T> {
    ptr: *mut T,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> MutPtrLifetimeGuard<'a, T> {
    fn new(value: &'a mut T) -> MutPtrLifetimeGuard<'a, T> {
        MutPtrLifetimeGuard { ptr: value as *mut T, _marker: PhantomData }
    }
}

struct SharedPtrLifetimeGuard<'a, T> {
    ptr: *const T,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> SharedPtrLifetimeGuard<'a, T> {
    fn new(value: &'a T) -> SharedPtrLifetimeGuard<'a, T> {
        SharedPtrLifetimeGuard { ptr: value as *const T, _marker: PhantomData }
    }
}
// #endregion

// #region ### loop resource
#[derive(Default)]
pub struct Loop {
    pub time: u128,
    pub second: f32,
}

impl world::UniqueResource for Loop {}

// ### UI resource
pub struct Ui {
    ui: *mut imgui::Ui,
    renderer: *mut glium_imgui_renderer::Renderer,
    pub textures: HashMap<String, imgui::TextureId>,
}

impl Ui {
    pub fn renderer(&mut self) -> &mut glium_imgui_renderer::Renderer {
        unsafe { &mut *self.renderer }
    }
}

impl Deref for Ui {
    type Target = imgui::Ui;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ui }
    }
}

impl DerefMut for Ui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ui }
    }
}

impl world::UniqueResource for Ui {}

// ### Frame resource
pub struct OpenGL {
    pub frame: Option<glium::Frame>,
    display: *const SDL2Facade,
    pub textures: HashMap<String, std::rc::Rc<glium::texture::CompressedTexture2d>>,
}

impl OpenGL {
    pub fn display(&self) -> &SDL2Facade {
        unsafe { &*self.display }
    }
}

impl Deref for OpenGL {
    type Target = glium::Frame;

    fn deref(&self) -> &Self::Target {
        self.frame.as_ref().unwrap()
    }
}

impl DerefMut for OpenGL {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.frame.as_mut().unwrap()
    }
}

impl world::UniqueResource for OpenGL {}
