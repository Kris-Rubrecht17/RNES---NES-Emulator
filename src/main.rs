#![allow(dead_code)]

use std::sync::mpsc::{Sender,Receiver,channel};

use sdl2::{pixels::Color, EventPump};

mod bus;
mod cartridge;
mod cpu;
mod input;
mod ppu;
#[cfg(test)]
mod tests;






fn handle_input(event_pump : &mut EventPump, sender : &Sender<Option<u8>>,controller_state :&mut u8) {
    use sdl2::event::Event;
    
    for event in event_pump.poll_iter() {
            use sdl2::keyboard::Keycode;
            match event {
                Event::Quit { .. } => {

                    sender.send(None).unwrap();
                    return;
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::X => {
                        *controller_state |= 1;
                    }
                    Keycode::Z => {
                        *controller_state |= 1 << 1;
                    }
                    Keycode::LShift | Keycode::RShift => *controller_state |= 1 << 2,
                    Keycode::Return => {
                        *controller_state |= 1 << 3;
                    }
                    Keycode::Up => {
                        *controller_state |= 1 << 4;
                    }
                    Keycode::Down => {
                        *controller_state |= 1 << 5;
                    }
                    Keycode::Left => {
                        *controller_state |= 1 << 6;
                    }
                    Keycode::Right => {
                        *controller_state |= 1 << 7;
                    }
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::X => {
                        *controller_state &= !1;
                    }
                    Keycode::Z => {
                        *controller_state &= !(1 << 1);
                    }
                    Keycode::LShift | Keycode::RShift => {
                        *controller_state &= !(1 << 2);
                    }
                    Keycode::Return => {
                        *controller_state &= !(1 << 3);
                    }
                    Keycode::Up => {
                        *controller_state &= !(1 << 4);
                    }
                    Keycode::Down => {
                        *controller_state &= !(1 << 5);
                    }
                    Keycode::Left => {
                        *controller_state &= !(1 << 6);
                    }
                    Keycode::Right => {
                        *controller_state &= !(1 << 7);
                    }
                    _ => {}
                },
                _ => {}
            }
            
        }
        sender.send(Some(*controller_state)).unwrap();
}






fn main() {
    use crate::{
        bus::Bus,
        cartridge::{Cartridge, Mapper},
        cpu::CPU,
        ppu::PPU,
    };
    use sdl2::{event::Event, rect::Rect};



    let sdl_context = sdl2::init().unwrap();

    let video = sdl_context.video().unwrap();

    let window = video
        .window(
            "RNES TEST",
            (PPU::SCREEN_WIDTH  * 4 )as u32,
            (PPU::SCREEN_HEIGHT  * 4) as u32,
        )
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    
    let (input_send,input_receive) = channel::<Option<u8>>();
    let (screen_send,screen_receive) = channel::<Option<Vec<Color>>>();
    
    let emu_thread = std::thread::spawn(move || {

        let cart = Cartridge::from_file("test_roms/official.nes").unwrap();
        let mapper = Mapper::with_cart(cart);

        let mut cpu = CPU::init(Bus::init(mapper));
        'run : loop {
        
        if let Some(controller) = input_receive.recv().unwrap() {
            cpu.bus.input.borrow_mut().controller_state = controller;
        }
        else {
            screen_send.send(None).unwrap();
            break 'run;
        }
        for _ in 0..28781 {
            let cycles = cpu.execute_instruction();
            cpu.bus.tick_ppu(cycles * 3);
        }


        screen_send.send(Some(cpu.bus.ppu.frame_buffer.clone())).unwrap();
        }
    });



    let mut controller_state = 0;
    'running: loop {
        use sdl2::pixels::Color;

        canvas.set_draw_color(Color::RGBA(0x00, 0, 0, 0xFF));
        canvas.clear();
        
        handle_input(&mut event_pump,&input_send,& mut controller_state);
        
        if let Some(framebuffer) = screen_receive.recv().unwrap(){
        
        for i in 0..PPU::SCREEN_HEIGHT {
            for j in 0..PPU::SCREEN_WIDTH {
                let idx = i * PPU::SCREEN_WIDTH + j;
                canvas.set_draw_color(framebuffer[idx]);
                let _ = canvas.fill_rect(Rect::new((j * 4) as i32, (i * 4) as i32, 4, 4));
            }
        }
    }
    else {
        break 'running;
    }
        canvas.present();
    }
    let _ = emu_thread.join().unwrap();
}
