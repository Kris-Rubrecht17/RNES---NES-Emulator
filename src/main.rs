#![allow(dead_code)]

mod bus;
mod cartridge;
mod cpu;
mod input;
mod ppu;
#[cfg(test)]
mod tests;

fn main() {
    use crate::{
        bus::Bus,
        cartridge::{Cartridge, Mapper},
        cpu::CPU,
        ppu::PPU,
    };
    use sdl2::{event::Event, rect::Rect};

    let cart = Cartridge::from_file("official_roms/Tetris.nes").unwrap();
    let mapper = Mapper::with_cart(cart);

    let mut cpu = CPU::init(Bus::init(mapper));

    let sdl_context = sdl2::init().unwrap();

    let video = sdl_context.video().unwrap();

    let window = video
        .window(
            "RNES TEST",
            PPU::SCREEN_WIDTH as u32,
            PPU::SCREEN_HEIGHT as u32,
        )
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        use sdl2::pixels::Color;

        canvas.set_draw_color(Color::RGBA(0x00, 0, 0, 0xFF));
        canvas.clear();

        let mut controller_state = 0;

        for event in event_pump.poll_iter() {
            use sdl2::keyboard::Keycode;
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::X => {
                        controller_state |= 1;
                    }
                    Keycode::Z => {
                        controller_state |= 1 << 1;
                    }
                    Keycode::LShift | Keycode::RShift => controller_state |= 1 << 2,
                    Keycode::Return => {
                        controller_state |= 1 << 3;
                    }
                    Keycode::Up => {
                        controller_state |= 1 << 4;
                    }
                    Keycode::Down => {
                        controller_state |= 1 << 5;
                    }
                    Keycode::Left => {
                        controller_state |= 1 << 6;
                    }
                    Keycode::Right => {
                        controller_state |= 1 << 7;
                    }
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::X => {
                        controller_state &= !1;
                    }
                    Keycode::Z => {
                        controller_state &= !(1 << 1);
                    }
                    Keycode::LShift | Keycode::RShift => {
                        controller_state &= !(1 << 2);
                    }
                    Keycode::Return => {
                        controller_state &= !(1 << 3);
                    }
                    Keycode::Up => {
                        controller_state &= !(1 << 4);
                    }
                    Keycode::Down => {
                        controller_state &= !(1 << 5);
                    }
                    Keycode::Left => {
                        controller_state &= !(1 << 6);
                    }
                    Keycode::Right => {
                        controller_state &= !(1 << 7);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        cpu.bus.input.borrow_mut().controller_state = controller_state;
        for _ in 0..28781 {
            let cycles = cpu.execute_instruction();
            cpu.bus.tick_ppu(cycles * 3);
        }
        for i in 0..PPU::SCREEN_HEIGHT {
            for j in 0..PPU::SCREEN_WIDTH {
                let idx = i * PPU::SCREEN_WIDTH + j;
                canvas.set_draw_color(cpu.bus.ppu.frame_buffer[idx]);
                let _ = canvas.fill_rect(Rect::new(j as i32, i as i32, 1, 1));
            }
        }
        canvas.present();
    }
}
