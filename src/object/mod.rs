use alloc::boxed::Box;
use crate::render;
use nx::input;

pub struct Callback(Box<dyn Fn()>);

impl Callback {
    pub fn from<F: Fn() + 'static>(f: F) -> Self {
        Self(Box::new(f))
    }

    pub fn run(&self) {
        (self.0)();
    }
}

pub trait Object {
    fn get_x(&self) -> i32;
    fn get_y(&self) -> i32;
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;
    fn on_render(&mut self, renderer: &mut render::Renderer);
    fn get_registered_events(&mut self) -> &mut event::RegisteredEventTable;

    fn get_position(&self) -> (i32, i32) {
        (self.get_x(), self.get_y())
    }
    fn get_size(&self) -> (u32, u32) {
        (self.get_width(), self.get_height())
    }

    fn get_click_bounds(&self) -> (i32, i32, u32, u32) {
        let pos = self.get_position();
        let size = self.get_size();
        (pos.0, pos.1, size.0, size.1)
    }

    fn on_event_handle(&mut self, ctx: &render::RenderContext) {
        let events = self.get_registered_events();
        for event in events.iter() {
            if event.handle(ctx) {
                event.get_callback().run();
            }
        }
    }

    fn register_event(&mut self, event: Box<dyn event::Event>) {
        self.get_registered_events().push(event);
    }

    // Specific events
    fn on_keys_down(&mut self, keys: input::Key, callback: Callback) {
        self.register_event(Box::new(event::KeyEvent::new(keys, event::KeyMode::Down, callback)));
    }

    fn on_keys_up(&mut self, keys: input::Key, callback: Callback) {
        self.register_event(Box::new(event::KeyEvent::new(keys, event::KeyMode::Up, callback)));
    }

    fn on_keys_held(&mut self, keys: input::Key, callback: Callback) {
        self.register_event(Box::new(event::KeyEvent::new(keys, event::KeyMode::Held, callback)));
    }
}

pub mod container;

pub mod button;

pub mod event;
