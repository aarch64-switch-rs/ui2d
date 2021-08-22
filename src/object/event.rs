use super::*;
use crate::render;
use nx::input;
use alloc::vec::Vec;
use alloc::boxed::Box;

pub trait Event {
    fn get_callback(&self) -> &Callback;
    fn handle(&self, ctx: &render::RenderContext) -> bool;
}

pub type RegisteredEvent = Box<dyn Event>;
pub type RegisteredEventTable = Vec<RegisteredEvent>;

// Key event

pub enum KeyMode {
    Down,
    Up,
    Held,
}

pub struct KeyEvent {
    callback: Callback,
    key: input::Key,
    mode: KeyMode
}

impl KeyEvent {
    pub fn new(key: input::Key, mode: KeyMode, callback: Callback) -> Self {
        Self { callback: callback, key: key, mode: mode }
    }
}

impl Event for KeyEvent {
    fn get_callback(&self) -> &Callback {
        &self.callback
    }

    fn handle(&self, ctx: &render::RenderContext) -> bool {
        let keys = match self.mode {
            KeyMode::Down => ctx.keys_down,
            KeyMode::Up => ctx.keys_up,
            KeyMode::Held => ctx.keys_held,
        };
        self.key.contains(keys)
    }
}