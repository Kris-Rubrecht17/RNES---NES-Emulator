#![allow(dead_code)]

mod bus;
mod cartridge;
mod cpu;
mod emulator;
mod input;
mod ppu;
mod ui;

use sdl2::pixels::Color;
use std::sync::mpsc::channel;
use ui::{RnesUI, UiEvent};

#[cfg(test)]
mod tests;

fn main() {
    let (sx1, rx1) = channel::<Vec<Color>>();
    let (sx2, rx2) = channel::<UiEvent>();

    let emu_thread = std::thread::spawn(move || {
        use crate::emulator::Emulator;

        let mut emu = Emulator::new(sx1, rx2);

        emu.run();
    });

    let mut ui = RnesUI::new(1280, 720, sx2, rx1);
    ui.run();

    emu_thread.join().unwrap();
}
