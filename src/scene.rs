extern crate alloc;
use alloc::vec::Vec;

use crate::object;
use crate::gui;
use nx::mem;

pub trait Scene {
    fn get_objects(&mut self) -> &mut Vec<mem::Shared<dyn object::Object>>;
    
    fn add_object(&mut self, object: mem::Shared<dyn object::Object>) {
        self.get_objects().push(object);
    }
}