use sdl2::rect::Rect;

use crate::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct UiConfig {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) scale: u32,
    pub(super) offset_x: u32,
    pub(super) offset_y: u32,
    pub(super) dst_rect: Option<Rect>,
}
impl UiConfig {
    pub fn new(width: u32, height: u32) -> Self {
        let mut cfg = UiConfig {
            width,
            height,
            scale: 0,
            offset_x: 0,
            offset_y: 0,
            dst_rect: None,
        };
        cfg.calculate_scale_and_offsets();
        cfg
    }
    pub fn calculate_scale_and_offsets(&mut self) {
        let (w, h) = (self.width, self.height);
        let screen_w = SCREEN_WIDTH as u32;
        let screen_h = SCREEN_HEIGHT as u32;

        self.scale = (w / screen_w).min(h / screen_h);
        assert!(self.scale >= 1, "Window must be at least 256x240px");

        self.offset_x = w - self.scale * screen_w;
        self.offset_x >>= 1;

        self.offset_y = h - self.scale * screen_h;
        self.offset_y >>= 1;
        self.dst_rect = Some(Rect::new(
            self.offset_x as i32,
            self.offset_y as i32,
            SCREEN_WIDTH as u32 * self.scale,
            SCREEN_HEIGHT as u32 * self.scale,
        ))
    }
}
