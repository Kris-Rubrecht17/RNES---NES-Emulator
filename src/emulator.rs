use std::sync::mpsc::{Sender,Receiver};

use sdl2::pixels::Color;

use crate::{cartridge::{Cartridge,Mapper}, cpu::CPU};

use crate::ui::UiEvent;





pub struct Emulator {
    cpu : CPU,
    cartridge_loaded : bool,
    screen_send : Sender<Vec<Color>>,
    event_receive : Receiver<UiEvent>,
    fps_counter : u32
}


impl Emulator {
    pub fn new(screen_send : Sender<Vec<Color>>, event_receive : Receiver<UiEvent>)->Self {
        Emulator { cpu: CPU::init(), cartridge_loaded: false,screen_send,event_receive,fps_counter:0}
    }
    pub fn load_cartridge(&mut self, file_path : String) {
        if let Ok(cartridge) = Cartridge::from_file(file_path) {
            let mapper = Mapper::with_cart(cartridge);
            self.cpu.bus.load_cartridge(mapper);
            self.cpu.reset();
            self.cartridge_loaded = true;
        }
    }
    pub fn run(&mut self) {

        let frame_time = std::time::Duration::from_secs_f64(1.0/60.0);
        let mut last_fps_check = std::time::Instant::now();
        let mut previous_frame = std::time::Instant::now();
        let mut accumulator = std::time::Duration::ZERO;

        'run : loop {
            let now = std::time::Instant::now();
            let delta = now - previous_frame;
            accumulator += delta;
            previous_frame = now;

            if let Ok(event) = self.event_receive.try_recv() {
                    match event {
                        UiEvent::Quit=>{break 'run;}
                        UiEvent::ControllerInput(inp)=>{
                            self.cpu.bus.input.borrow_mut().controller_state = inp;
                        }
                        UiEvent::LoadCart(file_path)=>{
                            self.load_cartridge(file_path);
                        }
                    }
                }
            if !self.cartridge_loaded {
                    continue;
                } 

            while accumulator >= frame_time {
                self.fps_counter += 1;
                if now.duration_since(last_fps_check) >= std::time::Duration::from_secs(1) {
                    self.fps_counter = 0;
                    last_fps_check = now;
                }
                let mut cycles = 0;
                while cycles < 29781 {

                    let new_cycles = self.cpu.execute_instruction();
                    self.cpu.bus.tick_ppu(new_cycles * 3);
                    cycles += new_cycles + self.cpu.bus.extra_cycles;
                    self.cpu.bus.extra_cycles = 0;
                    
                }
                self.screen_send.send(self.cpu.bus.ppu.frame_buffer.clone()).unwrap();
                accumulator -= frame_time;
            }
        }

    }
}




