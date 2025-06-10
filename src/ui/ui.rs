use crossbeam_channel::Sender;
use std::sync::Arc;

use nfd::Response;
use sdl2::{
    EventPump,
    event::Event,
    keyboard::Mod,
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    video::{Window, WindowContext},
};

use super::config::UiConfig;
use super::event::UiEvent;
use crate::{
    ppu::{SCREEN_HEIGHT, SCREEN_WIDTH},
    ui::frame_buffer::DoubleBuffer,
};

pub struct RnesUI<'a> {
    canvas: Canvas<Window>,
    cfg: UiConfig,
    event_pump: EventPump,
    event_send: Sender<UiEvent>,
    nes_input_state: u8,
    texture_creator: &'a TextureCreator<WindowContext>,
    texture: Texture<'a>,
    framebuffer: Arc<DoubleBuffer>,
}

impl<'a> RnesUI<'a> {
    //excessive use of unwrap because sdl errors aren't recoverable.

    pub fn new(
        width: u32,
        height: u32,
        event_send: Sender<UiEvent>,
        canvas: Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        framebuffer: Arc<DoubleBuffer>,
    ) -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video = sdl_context.video().unwrap();

        //clamp to monitor size just in case
        let video_mode = video.current_display_mode(0).unwrap();
        let width = if width > video_mode.w as u32 {
            video_mode.w as u32
        } else {
            width
        };
        let height = if height > video_mode.h as u32 {
            video_mode.h as u32
        } else {
            height
        };

        let cfg = UiConfig::new(width, height);
        let event_pump = sdl_context.event_pump().unwrap();
        let texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGBA32,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
            )
            .unwrap();
        RnesUI {
            canvas,
            cfg,
            event_send,
            event_pump,
            nes_input_state: 0,
            texture_creator,
            texture,
            framebuffer,
        }
    }
    fn handle_input(&mut self) -> bool {
        for event in self.event_pump.poll_iter() {
            use sdl2::keyboard::Keycode;
            match event {
                Event::Quit { .. } => {
                    self.event_send.send(UiEvent::Quit).unwrap();
                    return false;
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    keymod,
                    ..
                } => match keycode {
                    Keycode::X => {
                        self.nes_input_state |= 1;
                    }
                    Keycode::Z => {
                        self.nes_input_state |= 1 << 1;
                    }
                    Keycode::LSHIFT | Keycode::RSHIFT => {
                        self.nes_input_state |= 1 << 2;
                    }
                    Keycode::Return => {
                        self.nes_input_state |= 1 << 3;
                    }
                    Keycode::Up => {
                        self.nes_input_state |= 1 << 4;
                    }
                    Keycode::Down => {
                        self.nes_input_state |= 1 << 5;
                    }
                    Keycode::Left => {
                        self.nes_input_state |= 1 << 6;
                    }
                    Keycode::Right => {
                        self.nes_input_state |= 1 << 7;
                    }
                    Keycode::O if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                        if let Ok(result) =
                            nfd::open_dialog(Some("nes"), None, nfd::DialogType::SingleFile)
                        {
                            match result {
                                Response::Okay(file_path) => {
                                    self.event_send.send(UiEvent::LoadCart(file_path)).unwrap();
                                    return true;
                                }
                                _ => {
                                    return true;
                                }
                            }
                        }
                    }
                    _ => {
                        return true;
                    }
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::X => {
                        self.nes_input_state &= !1;
                    }
                    Keycode::Z => {
                        self.nes_input_state &= !(1 << 1);
                    }
                    Keycode::LShift | Keycode::RShift => {
                        self.nes_input_state &= !(1 << 2);
                    }
                    Keycode::Return => {
                        self.nes_input_state &= !(1 << 3);
                    }
                    Keycode::Up => {
                        self.nes_input_state &= !(1 << 4);
                    }
                    Keycode::Down => {
                        self.nes_input_state &= !(1 << 5);
                    }
                    Keycode::Left => {
                        self.nes_input_state &= !(1 << 6);
                    }
                    Keycode::Right => {
                        self.nes_input_state &= !(1 << 7);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        self.event_send
            .send(UiEvent::ControllerInput(self.nes_input_state))
            .unwrap();
        true
    }
    fn render_nes_framebuffer(&mut self, framebuffer: &[Color]) {
        self.texture
            .with_lock(None, |buffer, pitch| {
                for y in 0..SCREEN_HEIGHT {
                    let offset_tex = y * pitch;
                    let offset_src = y * SCREEN_WIDTH;
                    for x in 0..SCREEN_WIDTH {
                        let color = framebuffer[offset_src + x];
                        let pixel_offset = offset_tex + x * 4;

                        buffer[pixel_offset..pixel_offset + 4]
                            .copy_from_slice(&[color.r, color.g, color.b, color.a]);
                    }
                }
            })
            .unwrap();
    }
    pub fn run(&mut self) {
        'running: loop {
            //A quit event returns false and sends a quit signal to the emulator thread.
            if !self.handle_input() {
                break 'running;
            }
            let framebuffer = self.framebuffer.clone();
            self.render_nes_framebuffer(framebuffer.read_front_buffer());

            self.canvas
                .copy(&self.texture, None, self.cfg.dst_rect)
                .unwrap();
            self.canvas.present();
        }
    }
}
