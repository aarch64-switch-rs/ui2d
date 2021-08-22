#[derive(Copy, Clone)]
pub struct RGBA8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RGBA8 {
    pub const fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r: r, g: g, b: b, a: a }
    }

    pub const fn new_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r: r, g: g, b: b, a: 0xFF }
    }

    const fn decode(raw: u32) -> (u8, u8, u8, u8) {
        let a = (raw & 0xFF) as u8;
        let b = ((raw >> 8) & 0xFF) as u8;
        let c = ((raw >> 16) & 0xFF) as u8;
        let d = ((raw >> 24) & 0xFF) as u8;
        (a, b, c, d)
    }

    pub const fn from(raw: u32) -> Self {
        let (r, g, b, a) = Self::decode(raw);
        Self::new_rgba(r, g, b, a)
    }

    const fn encode_impl(a: u8, b: u8, c: u8, d: u8) -> u32 {
        (a as u32 & 0xFF) | ((b as u32 & 0xFF) << 8) | ((c as u32 & 0xFF) << 16) | ((d as u32 & 0xFF) << 24)
    }

    pub const fn encode(&self) -> u32 {
        Self::encode_impl(self.r, self.g, self.b, self.a)
    }

    const fn blend_color_impl(src: u32, dst: u32, alpha: u32) -> u8 {
        let one_minus_a = 0xFF - alpha;
        ((dst * alpha + src * one_minus_a) / 0xFF) as u8
    }

    pub const fn blend_with(&self, other: Self) -> Self {
        let r = Self::blend_color_impl(other.r as u32, self.r as u32, self.a as u32);
        let g = Self::blend_color_impl(other.g as u32, self.g as u32, self.a as u32);
        let b = Self::blend_color_impl(other.b as u32, self.b as u32, self.a as u32);
        Self::new_rgb(r, g, b)
    }
}