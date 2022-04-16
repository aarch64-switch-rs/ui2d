use crate::color;
use crate::render;
use crate::scene;
use alloc::vec::Vec;
use nx::gpu;
use nx::input;
use nx::mem;
use nx::result::*;
use nx::service::applet;
use nx::service::hid;

pub struct Gui {
  gpu_surface: gpu::surface::Surface,
  renderer: render::Renderer,
  scenes: Vec<mem::Shared<dyn scene::Scene>>,
  clear_color: color::RGBA8,
  current_scene_idx: usize,
  is_shown: bool,
  input_ctx: input::InputContext,
}

impl Gui {
  pub fn new(
    surface: gpu::surface::Surface,
    aruid: applet::AppletResourceUserId,
    supported_tags: hid::NpadStyleTag,
    controllers: &[hid::ControllerId],
  ) -> Result<mem::Shared<Self>> {
    let renderer = render::Renderer::from(&surface);
    Ok(mem::Shared::new(Self {
      gpu_surface: surface,
      renderer: renderer,
      scenes: Vec::new(),
      clear_color: color::RGBA8::new_rgb(0xFF, 0xFF, 0xFF),
      current_scene_idx: 0,
      is_shown: false,
      input_ctx: input::InputContext::new(aruid, supported_tags, controllers)?,
    }))
  }

  pub fn add_scene<T: scene::Scene + 'static>(&mut self, t: T) {
    self.scenes.push(mem::Shared::new(t));
  }

  pub fn set_clear_color(&mut self, clear_color: color::RGBA8) {
    self.clear_color = clear_color;
  }

  pub fn show(&mut self) -> Result<()> {
    self.is_shown = true;
    while self.is_shown {
      if let Some(scene) = self.scenes.get(self.current_scene_idx) {
        let scene_c = scene.clone();

        let render_ctx = render::RenderContext::new(&mut self.input_ctx)?;
        for object in scene_c.get().get_objects().iter_mut() {
          object.get().on_event_handle(&render_ctx);
        }

        self.renderer.start(&mut self.gpu_surface);
        self.renderer.clear(self.clear_color);

        let scene_c = scene.clone();
        for object in scene_c.get().get_objects().iter_mut() {
          object.get().on_render(&mut self.renderer);
        }

        self.renderer.end(&mut self.gpu_surface);
      } else {
        break;
      }
    }

    Ok(())
  }

  pub fn close(&mut self) {
    self.is_shown = false;
  }
}
