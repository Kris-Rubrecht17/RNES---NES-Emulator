use std::sync::{atomic::{AtomicU8, Ordering}, mpsc::{Receiver, Sender}, Arc};

use nfd::Response;
use sdl2::{event::Event, keyboard::{Mod, Scancode}, pixels::Color, rect::Rect, render::Canvas, video::Window, EventPump};

use crate::ppu::{SCREEN_WIDTH,SCREEN_HEIGHT};
use super::config::UiConfig;
use super::event::UiEvent;

pub struct RnesUI {
    canvas: Canvas<Window>,
    cfg: UiConfig,
    event_pump: EventPump,
    event_send : Sender<UiEvent>,
    screen_receive: Receiver<Vec<Color>>,
    nes_input_state : u8
}

impl RnesUI {
    //excessive use of unwrap because sdl errors aren't recoverable.
    pub fn new(
        width: u32,
        height: u32,
        event_send: Sender<UiEvent>,
        screen_receive: Receiver<Vec<Color>>,
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

        let canvas = video
            .window("Test RNES", width, height)
            .position_centered()
            .build()
            .unwrap()
            .into_canvas()
            .build()
            .unwrap();
        let cfg = UiConfig::new(width, height);
        let event_pump = sdl_context.event_pump().unwrap();
        RnesUI {
            canvas,
            cfg,
            event_send,
            screen_receive,
            event_pump,
            nes_input_state:0,
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
                    Keycode::LShift | Keycode::RShift => self.nes_input_state |= 1 << 2,
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
                    Keycode::O if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)=>{
                        if let Ok(result)    = nfd::open_dialog(Some("nes"),None,nfd::DialogType::SingleFile) {

                                    match result {
                                        Response::Okay(file_path)=> {
                                            self.event_send.send(UiEvent::LoadCart(file_path)).unwrap();
                                            return true;
                                        }
                                        _=>{return true;}
                                    }


                                }
                    }
                    _ => {return true;}
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
        self.event_send.send(UiEvent::ControllerInput(self.nes_input_state)).unwrap();
        true
        
    }
    fn render_nes_framebuffer(&mut self, framebuffer: Vec<Color>) {
        for i in 0..SCREEN_HEIGHT {
            for j in 0..SCREEN_WIDTH {
                let idx = i * SCREEN_WIDTH + j;
                self.canvas.set_draw_color(framebuffer[idx]);
                self.canvas
                    .fill_rect(Rect::new(
                        self.cfg.offset_x as i32 + j as i32 * self.cfg.scale as i32,
                        self.cfg.offset_y as i32 + i as i32 * self.cfg.scale as i32,
                        self.cfg.scale,
                        self.cfg.scale,
                    ))
                    .unwrap();
            }
        }
    }
    pub fn run(&mut self) {
        'running: loop {
            //A quit event returns false and sends a quit signal to the emulator thread. 
            if !self.handle_input() {
                break 'running;
            }

            
            if let Ok(framebuffer) = self.screen_receive.try_recv() {
                self.render_nes_framebuffer(framebuffer);
                self.canvas.present();
            }   
        }
    }
}

