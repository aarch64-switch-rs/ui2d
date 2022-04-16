use crate::color;
use alloc::string::String;
use alloc::vec::Vec;
use nx::result::*;
use nx::arm;
use nx::gpu;
use nx::service::hid;
use nx::input;
use core::ptr;
use core::mem as cmem;

pub struct RegisteredFont {
    pub name: &'static str,
    pub font: &'static rusttype::Font<'static>
}

impl RegisteredFont {
    pub fn new(name: &'static str, font: &'static rusttype::Font<'static>) -> Self {
        Self { name: name, font: font }
    }
}

pub struct RenderContext {
    pub keys_down: input::Key,
    pub keys_up: input::Key,
    pub keys_held: input::Key,
    pub touch_data:  Option<input::TouchData>
}

impl RenderContext {
    pub fn new(input_ctx: &mut input::InputContext) -> Result<Self> {
        let mut input_player = match input_ctx.is_controller_connected(hid::ControllerId::Player1) {
            true => input_ctx.get_player(hid::ControllerId::Player1),
            false => input_ctx.get_player(hid::ControllerId::Handheld)
        }?;
        let keys_down = input_player.get_button_state_down();
        let keys_up = input_player.get_button_state_up();
        let keys_held = input_player.get_button_state_held();
        let touch_data = match keys_held.contains(input::Key::Touch()) {
            true => Some(input_ctx.get_touch_data(0)?),
            false => None
        };
        Ok(Self { keys_down: keys_down, keys_up: keys_up, keys_held: keys_held, touch_data: touch_data })
    }
}

pub struct Renderer {
    gpu_buf: *mut u32,
    gpu_buf_size: usize,
    linear_buf: *mut u32,
    linear_buf_size: usize,
    stride: u32,
    width: u32,
    height: u32,
    color_format: gpu::ColorFormat,
    slot: i32,
    fences: gpu::MultiFence,
    loaded_fonts: Vec<RegisteredFont>
}

impl Renderer {
    pub fn from(surface: &gpu::surface::Surface) -> Self {
        let stride = surface.compute_stride();
        let width = surface.get_width();
        let height = surface.get_height();
        let aligned_width = stride as usize;
        let aligned_height = ((height + 7) & !7) as usize;
        let linear_buf_size = aligned_width * aligned_height;
        unsafe {
            let linear_buf_layout = alloc::alloc::Layout::from_size_align_unchecked(linear_buf_size, 8);
            let linear_buf = alloc::alloc::alloc_zeroed(linear_buf_layout);
            Self { gpu_buf: ptr::null_mut(), gpu_buf_size: 0, linear_buf: linear_buf as *mut u32, linear_buf_size: linear_buf_size, stride: stride, width: width, height: height, color_format: surface.get_color_format(), slot: 0, fences: cmem::zeroed(), loaded_fonts: Vec::new() }
        }
    }

    pub fn start(&mut self, surface: &mut gpu::surface::Surface) -> Result<()> {
        let (buf, buf_size, slot, _has_fences, fences) = surface.dequeue_buffer(true)?;
        self.gpu_buf = buf as *mut u32;
        self.gpu_buf_size = buf_size;
        self.slot = slot;
        self.fences = fences;
        surface.wait_fences(fences, -1)
    }

    pub fn load_font(&mut self, font: &'static rusttype::Font<'static>, name: &'static str) {
        self.loaded_fonts.push(RegisteredFont::new(name, font));
    }

    pub fn find_font(&self, name: &'static str) -> Option<&rusttype::Font> {
        for loaded_font in self.loaded_fonts.iter() {
            if loaded_font.name == name {
                return Some(&loaded_font.font);
            }
        }
        None
    }

    fn convert_buffers_gob_impl(out_gob_buf: *mut u8, in_gob_buf: *mut u8, stride: u32) {
        unsafe {
            let mut tmp_out_gob_buf_128 = out_gob_buf as *mut u128;
            for i in 0..32 {
                let y = ((i >> 1) & 0x6) | (i & 0x1);
                let x = ((i << 3) & 0x10) | ((i << 1) & 0x20);
                let out_gob_buf_128 = tmp_out_gob_buf_128 as *mut u128;
                let in_gob_buf_128 = in_gob_buf.offset((y * stride + x) as isize) as *mut u128;
                *out_gob_buf_128 = *in_gob_buf_128;
                tmp_out_gob_buf_128 = tmp_out_gob_buf_128.offset(1);
            }
        }
    }

    fn convert_buffers_impl(out_buf: *mut u8, in_buf: *mut u8, stride: u32, height: u32) {
        let block_height_gobs = 1 << gpu::BLOCK_HEIGHT_LOG2;
        let block_height_px = 8 << gpu::BLOCK_HEIGHT_LOG2;

        let width_blocks = stride >> 6;
        let height_blocks = (height + block_height_px - 1) >> (3 + gpu::BLOCK_HEIGHT_LOG2);
        let mut tmp_out_buf = out_buf;

        for block_y in 0..height_blocks {
            for block_x in 0..width_blocks {
                for gob_y in 0..block_height_gobs {
                    unsafe {
                        let x = block_x * 64;
                        let y = block_y * block_height_px + gob_y * 8;
                        if y < height {
                            let in_gob_buf = in_buf.offset((y * stride + x) as isize);
                            Self::convert_buffers_gob_impl(tmp_out_buf, in_gob_buf, stride);
                        }
                        tmp_out_buf = tmp_out_buf.offset(512);
                    }
                }
            }
        }
    }

    pub fn end(&mut self, surface: &mut gpu::surface::Surface) -> Result<()> {
        Self::convert_buffers_impl(self.gpu_buf as *mut u8, self.linear_buf as *mut u8, self.stride, self.height);
        arm::cache_flush(self.gpu_buf as *mut u8, self.gpu_buf_size);
        surface.queue_buffer(self.slot, self.fences)?;
        surface.wait_vsync_event(-1)
    }

    pub fn clear(&mut self, color: color::RGBA8) {
        unsafe {
            let buf_size_32 = self.linear_buf_size / cmem::size_of::<u32>();
            for i in 0..buf_size_32 {
                let cur = self.linear_buf.offset(i as isize);
                *cur = color.encode();
            }
        }
    }

    pub fn draw_single(&mut self, x: i32, y: i32, color: color::RGBA8) {
        unsafe {
            let offset = ((self.stride / cmem::size_of::<u32>() as u32) as i32 * y + x) as isize;
            let cur = self.linear_buf.offset(offset);
            let old_color = color::RGBA8::from(*cur);
            let new_color = color.blend_with(old_color);
            *cur = new_color.encode();
        }
    }

    fn clamp(max: i32, value: i32) -> i32 {
        if value < 0 {
            return 0;
        }
        if value > max {
            return max;
        }
        value
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn get_color_format(&self) -> gpu::ColorFormat {
        self.color_format
    }

    pub fn draw(&mut self, x: i32, y: i32, width: i32, height: i32, color: color::RGBA8) {
        let s_width = self.width as i32;
        let s_height = self.height as i32;
        let x0 = Self::clamp(s_width, x);
        let x1 = Self::clamp(s_width, x + width);
        let y0 = Self::clamp(s_height, y);
        let y1 = Self::clamp(s_height, y + height);
        for y in y0..y1 {
            for x in x0..x1 {
                self.draw_single(x, y, color);
            }
        }
    }

    fn draw_font_text_impl(&mut self, font: &rusttype::Font, text: &str, color: color::RGBA8, scale: rusttype::Scale, v_metrics: rusttype::VMetrics, x: i32, y: i32) {
        let glyphs: Vec<_> = font.layout(&text[..], scale, rusttype::point(0.0, v_metrics.ascent)).collect();
        for glyph in &glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                // Draw the glyph into the image per-pixel by using the draw closure
                glyph.draw(|g_x, g_y, g_v| {
                    let mut pix_color = color;
                    // Different alpha depending on the pixel
                    pix_color.a = (g_v * 255.0) as u8;
                    self.draw_single(x + g_x as i32 + bounding_box.min.x as i32, y + g_y as i32 + bounding_box.min.y as i32, pix_color);
                });
            }
        }
    }

    pub fn draw_font_text(&mut self, font: &rusttype::Font, text: String, color: color::RGBA8, size: f32, x: i32, y: i32) {
        let scale = rusttype::Scale::uniform(size);
        let v_metrics = font.v_metrics(scale);

        let mut tmp_y = y;
        for semi_text in text.lines() {
            self.draw_font_text_impl(font, semi_text, color, scale, v_metrics, x, tmp_y);
            tmp_y += v_metrics.ascent as i32;
        }
    }

    /*
    pub fn draw_bitmap_text(&mut self, text: String, color: RGBA8, scale: i32, x: i32, y: i32) {
        let mut tmp_x = x;
        let mut tmp_y = y;
        for c in text.chars() {
            match c {
                '\n' | '\r' => {
                    tmp_y += 8 * scale;
                    tmp_x = x;
                },
                _ => {
                    if let Some(glyph) = font8x8::BASIC_FONTS.get(c) {
                        let char_tmp_x = tmp_x;
                        let char_tmp_y = tmp_y;
                        for gx in &glyph {
                            for bit in 0..8 {
                                match *gx & 1 << bit {
                                    0 => {},
                                    _ => {
                                        self.draw(tmp_x, tmp_y, scale, scale, color);
                                    },
                                }
                                tmp_x += scale;
                            }
                            tmp_y += scale;
                            tmp_x = char_tmp_x;
                        }
                        tmp_x += 8 * scale;
                        tmp_y = char_tmp_y;
                    }
                }
            }
        }
    }
    */
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let linear_buf_layout = alloc::alloc::Layout::from_size_align_unchecked(self.linear_buf_size, 8);
            alloc::alloc::dealloc(self.linear_buf as *mut u8, linear_buf_layout);
        }
    }
}

// Needed by rusttype

pub trait FloatExt {
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn fract(self) -> Self;
    fn trunc(self) -> Self;
    fn round(self) -> Self;
    fn abs(self) -> Self;
}

impl FloatExt for f32 {
    #[inline]
    fn floor(self) -> Self {
        libm::floorf(self)
    }

    #[inline]
    fn ceil(self) -> Self {
        libm::ceilf(self)
    }

    #[inline]
    fn fract(self) -> Self {
        self - self.trunc()
    }

    #[inline]
    fn trunc(self) -> Self {
        libm::truncf(self)
    }

    #[inline]
    fn round(self) -> Self {
        libm::roundf(self)
    }

    #[inline]
    fn abs(self) -> Self {
        libm::fabsf(self)
    }
}
