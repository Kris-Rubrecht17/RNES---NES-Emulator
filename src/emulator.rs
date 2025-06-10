use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};

use sdl2::pixels::Color;

use crate::{
    cartridge::{Cartridge, Mapper},
    cpu::CPU,
    ppu::{SCREEN_HEIGHT, SCREEN_WIDTH},
    ui::frame_buffer::DoubleBuffer,
};

use crate::ui::UiEvent;

pub struct Emulator {
    cpu: CPU,
    cartridge_loaded: bool,
    event_receive: Receiver<UiEvent>,
    fps_counter: u32,
    fps_multiplier: f64,
    framebuffer: Arc<DoubleBuffer>,
}

impl Emulator {
    pub fn new(event_receive: Receiver<UiEvent>, framebuffer: Arc<DoubleBuffer>) -> Self {
        Emulator {
            cpu: CPU::init(),
            cartridge_loaded: false,

            event_receive,
            fps_counter: 0,
            fps_multiplier: 1.0,
            framebuffer,
        }
    }
    pub fn load_cartridge(&mut self, file_path: String) {
        if let Ok(cartridge) = Cartridge::from_file(file_path) {
            let mapper = Mapper::with_cart(cartridge);
            self.cpu.bus.load_cartridge(mapper);
            self.cpu.reset();
            self.cartridge_loaded = true;
        }
    }
    pub fn run(&mut self) {
        let target_fps = 60.0 * self.fps_multiplier;
        let frame_time = std::time::Duration::from_secs_f64(1.0 / target_fps);
        let mut last_fps_check = std::time::Instant::now();
        let mut last_frame_time = std::time::Instant::now();

        'run: loop {
            let now = std::time::Instant::now();
            let delta = now - last_frame_time;
            if delta < frame_time {
                // We're running too fast — sleep to match target FPS
                std::thread::sleep(frame_time - delta);
                continue;
            }

            last_frame_time = now;

            // Poll all input events quickly
            while let Ok(event) = self.event_receive.try_recv() {
                match event {
                    UiEvent::Quit => break 'run,
                    UiEvent::ControllerInput(inp) => {
                        self.cpu.bus.input.borrow_mut().controller_state = inp;
                    }
                    UiEvent::LoadCart(file_path) => {
                        self.load_cartridge(file_path);
                    }
                }
            }

            if !self.cartridge_loaded {
                continue;
            }

            self.fps_counter += 1;

            // FPS reporting
            if now.duration_since(last_fps_check) >= std::time::Duration::from_secs(1) {
                self.fps_counter = 0;
                last_fps_check = now;
            }

            // Emulate frame
            let mut cycles = 0;
            while cycles < 29781 {
                let new_cycles = self.cpu.execute_instruction();
                self.cpu.bus.tick_ppu(new_cycles * 3);
                cycles += new_cycles + self.cpu.bus.extra_cycles;
                self.cpu.bus.extra_cycles = 0;
            }
            let should_send_framebuffer = self.fps_multiplier <= 1.0
                || self.fps_counter % (self.fps_multiplier.round() as u32) == 0;

            if should_send_framebuffer {
                self.framebuffer.write_back_buffer(|buff| {
                    buff.copy_from_slice(&self.cpu.bus.ppu.frame_buffer[..]);
                });
                self.framebuffer.swap_buffers();
            }
        }
    }
}
