mod config;
pub mod event;
pub use event::UiEvent;
pub mod frame_buffer;




use crossbeam_channel::Sender;
use std::sync::Arc;

use nfd::Response;
use sdl2::{
    controller::{Button, GameController}, 
    event::{Event, WindowEvent}, 
    keyboard::{Mod,Keycode},
    pixels::{Color, PixelFormatEnum},
    render::{Canvas, Texture, TextureCreator}, 
    video::{Window, WindowContext}, 
    EventPump, 
    GameControllerSubsystem
};

use config::UiConfig;

use crate::{
    ppu::{SCREEN_HEIGHT, SCREEN_WIDTH},
    ui::frame_buffer::DoubleBuffer,
    input::NESButton
};

type KeyMap = std::collections::HashMap<Keycode,NESButton>;

type ControllerMap = std::collections::HashMap<Button,NESButton>;






pub struct RnesUI<'a> {
    canvas: Canvas<Window>,
    cfg: UiConfig,
    event_pump: EventPump,
    event_send: Sender<UiEvent>,
    nes_input_state: u8,
    texture_creator: &'a TextureCreator<WindowContext>,
    texture: Texture<'a>,
    framebuffer: Arc<DoubleBuffer>,
    controller_ctx: GameControllerSubsystem,
    controller: Option<GameController>,
    key_mapping : KeyMap,
    controller_mapping : ControllerMap
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
        let controller_system = sdl_context.game_controller().unwrap();
        let available = controller_system
            .num_joysticks()
            .map_err(|e| format!("Can't enumerate joysticks: {}", e))
            .unwrap_or(0);

        let controller = (0..available).find_map(|id| {
            if !controller_system.is_game_controller(id) {
                return None;
            }

            match controller_system.open(id) {
                Ok(c) => Some(c),
                Err(why) => {
                    println!("Could not open game controller: {}", why);
                    None
                }
            }
        });

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
        let mut key_mapping = KeyMap::new();
        key_mapping.insert(Keycode::Z,NESButton::B);
        key_mapping.insert(Keycode::X,NESButton::A);
        key_mapping.insert(Keycode::LShift,NESButton::Select);
        key_mapping.insert(Keycode::RShift,NESButton::Select);
        key_mapping.insert(Keycode::Return,NESButton::Start);
        key_mapping.insert(Keycode::Up,NESButton::Up);
        key_mapping.insert(Keycode::Down,NESButton::Down);
        key_mapping.insert(Keycode::Left,NESButton::Left);
        key_mapping.insert(Keycode::Right,NESButton::Right);

        let mut controller_mapping = ControllerMap::new();
        controller_mapping.insert(Button::B,NESButton::B);
        controller_mapping.insert(Button::A,NESButton::A);
        controller_mapping.insert(Button::Back,NESButton::Select);
        controller_mapping.insert(Button::Start,NESButton::Start);
        controller_mapping.insert(Button::DPadUp,NESButton::Up);
        controller_mapping.insert(Button::DPadDown,NESButton::Down);
        controller_mapping.insert(Button::DPadLeft,NESButton::Left);
        controller_mapping.insert(Button::DPadRight,NESButton::Right);


        RnesUI {
            canvas,
            cfg,
            event_send,
            event_pump,
            nes_input_state: 0,
            texture_creator,
            texture,
            framebuffer,
            controller_ctx: controller_system,
            controller,
            key_mapping,
            controller_mapping
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
                Event::KeyDown {keycode: Some(keycode), keymod,..} => 
                {
                    if keycode == Keycode::O && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)  {
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
                    if let Some(&button) = self.key_mapping.get(&keycode) {
                        self.nes_input_state |= button;
                    }
                },
                Event::KeyUp {keycode: Some(keycode),..} => {
                    if let Some(&button) = self.key_mapping.get(&keycode) {
                        self.nes_input_state &= !button
                    }
                },
                Event::ControllerButtonDown { button, .. } => {
                    if let Some(&button) = self.controller_mapping.get(&button) {
                        self.nes_input_state |= button;
                    }
                }
                Event::ControllerButtonUp { button, .. } => {
                    if let Some(&button) = self.controller_mapping.get(&button) {
                        self.nes_input_state &= !button;
                    }
                },
                Event::ControllerDeviceAdded { which, .. } => {
                    self.controller = match self.controller_ctx.open(which) {
                        Ok(c) => Some(c),
                        Err(_) => None,
                    };
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    if let Some(c) = &self.controller {
                        if c.instance_id() == which {
                            self.controller = None;
                        }
                    }
                }
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Resized(w, h) => {
                        self.cfg.width = w as u32;
                        self.cfg.height = h as u32;
                        self.cfg.calculate_scale_and_offsets();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        let _ = self
            .event_send
            .send(UiEvent::ControllerInput(self.nes_input_state));
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
            self.canvas.clear();
            let framebuffer = self.framebuffer.clone();
            self.render_nes_framebuffer(framebuffer.read_front_buffer());

            self.canvas
                .copy(&self.texture, None, self.cfg.dst_rect)
                .unwrap();
            self.canvas.present();
        }
    }
}
