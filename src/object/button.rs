use super::*;
use crate::render;
use crate::color;
use crate::object::event;
use alloc::vec::Vec;
use alloc::string::String;

pub struct Button {
    events: event::RegisteredEventTable,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    text: String
}

impl Button {
    pub fn new(x: i32, y: i32, width: u32, height: u32, text: String) -> Self {
        Self { events: Vec::new(), x: x, y: y, width: width, height: height, text: text }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn set_x(&mut self, x: i32) {
        self.x = x;
    }

    pub fn set_y(&mut self, y: i32) {
        self.y = y;
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = width;
    }

    pub fn set_height(&mut self, height: u32) {
        self.height = height;
    }
}

impl Object for Button {
    fn get_x(&self) -> i32 {
        self.x
    }

    fn get_y(&self) -> i32 {
        self.y
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn get_height(&self) -> u32 {
        self.height
    }

    fn on_render(&mut self, renderer: &mut render::Renderer) {
        renderer.draw(self.x, self.y, self.width as i32, self.height as i32, color::RGBA8::new_rgb(0xFF, 0x0, 0xFF))
    }

    fn get_registered_events(&mut self) -> &mut event::RegisteredEventTable {
        &mut self.events
    }
}