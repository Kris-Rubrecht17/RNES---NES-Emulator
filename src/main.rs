#![allow(dead_code)]

mod bus;
mod cartridge;
mod cpu;
mod emulator;
mod input;
mod ppu;
mod ui;

use std::sync::Arc;

use crossbeam_channel::unbounded;
use ui::{RnesUI, UiEvent};

use crate::ui::frame_buffer::DoubleBuffer;

#[cfg(test)]
mod tests;

fn main() {
    let buf = Arc::new(DoubleBuffer::new());
    let buf2 = Arc::clone(&buf);
    let (sx2, rx2) = unbounded::<UiEvent>();

    let emu_thread = std::thread::spawn(move || {
        use crate::emulator::Emulator;

        let mut emu = Emulator::new(rx2, buf);

        emu.run();
    });

    let sdl2 = sdl2::init().unwrap();
    let video = sdl2.video().unwrap();
    let canvas = video
        .window("RNES", 1280, 720)
        .build()
        .unwrap()
        .into_canvas()
        .build()
        .unwrap();

    let texture_creator = canvas.texture_creator();

    let mut ui = RnesUI::new(1280, 720, sx2, canvas, &texture_creator, buf2);

    ui.run();
    emu_thread.join().unwrap();
}
