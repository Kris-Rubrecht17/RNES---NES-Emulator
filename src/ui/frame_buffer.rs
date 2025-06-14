use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use sdl2::pixels::Color;

use crate::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub type Framebuffer = Box<[Color; SCREEN_HEIGHT * SCREEN_WIDTH]>;

pub struct DoubleBuffer {
    buffers: [SyncUnsafeCell; 2],
    current_idx: AtomicUsize,
}

impl DoubleBuffer {
    pub fn new() -> Self {
        let front = SyncUnsafeCell(UnsafeCell::new(Box::new(
            [Color::BLACK; SCREEN_HEIGHT * SCREEN_WIDTH],
        )));
        let back = SyncUnsafeCell(UnsafeCell::new(Box::new(
            [Color::BLACK; SCREEN_HEIGHT * SCREEN_WIDTH],
        )));

        DoubleBuffer {
            buffers: [front, back],
            current_idx: AtomicUsize::new(0),
        }
    }
    pub fn write_back_buffer<F: FnOnce(&mut [Color])>(&self, write_fn: F) {
        write_fn(unsafe {
            let idx = 1 - self.current_idx.load(Ordering::Acquire);
            &mut **self.buffers[idx].0.get()
        });
    }
    pub fn swap_buffers(&self) {
        let old_idx = self.current_idx.load(Ordering::Acquire);
        let new_idx = 1 - old_idx;
        self.current_idx.store(new_idx, Ordering::Release);
    }
    pub fn read_front_buffer(&self) -> &[Color] {
        let idx = self.current_idx.load(Ordering::Acquire);
        unsafe { &**self.buffers[idx].0.get() }
    }
}

pub struct SyncUnsafeCell(pub UnsafeCell<Framebuffer>);

unsafe impl Sync for SyncUnsafeCell {}
unsafe impl Send for SyncUnsafeCell {}
