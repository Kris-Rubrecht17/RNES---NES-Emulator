use std::{cell::RefCell, rc::Rc};

use sdl2::pixels::Color;

use crate::cartridge::{Mapper, MirrorMode};

pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 240;

pub struct PPURegisters {
    pub control: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,
    pub oam_data: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub ppu_addr: u16,
    pub ppu_data: u8,
    pub address_latch: bool,
    pub fine_x: u8,
    pub vram_addr: u16,
    pub tmp_vram_addr: u16,
    pub data_buffer: u8,
}

impl PPURegisters {
    pub fn new() -> Self {
        PPURegisters {
            control: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            oam_data: 0,
            ppu_addr: 0,
            ppu_data: 0,
            scroll_x: 0,
            scroll_y: 0,
            address_latch: false,
            fine_x: 0,
            vram_addr: 0,
            tmp_vram_addr: 0,
            data_buffer: 0,
        }
    }
    pub fn reset(&mut self) {
        self.control = 0;
        self.mask = 0;
        self.status = 0;
        self.oam_addr = 0;
        self.oam_data = 0;
        self.ppu_addr = 0;
        self.ppu_data = 0;
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.address_latch =false;
        self.fine_x = 0;
        self.vram_addr = 0;
        self.tmp_vram_addr = 0;
        self.data_buffer = 0;
    }
}

pub struct PPU {
    //need interior mutability since a read from the registers might cause other registers to change.
    pub registers: Rc<RefCell<PPURegisters>>,
    vram: Vec<u8>,
    palette_ram: [u8; 32],
    pub oam_ram: [u8; 256],
    pub frame_buffer: Box<[Color; SCREEN_HEIGHT * SCREEN_WIDTH]>,
    background_priority: Box<[bool; SCREEN_HEIGHT * SCREEN_WIDTH]>,
    scanline: u32,
    scanline_cycle: u32,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            registers: Rc::new(RefCell::new(PPURegisters::new())),
            vram: vec![0; 2048],
            palette_ram: [0; 32],
            oam_ram: [0; 256],
            frame_buffer: Box::new([Color::BLACK; SCREEN_HEIGHT * SCREEN_WIDTH]),
            background_priority: Box::new([false; SCREEN_HEIGHT * SCREEN_WIDTH]),
            scanline: 0,
            scanline_cycle: 0,
        }
    }
    pub fn reset(&mut self) {
        self.registers.borrow_mut().reset();
        self.vram.fill(0);
        self.oam_ram.fill(0);
        self.frame_buffer.fill(Color::BLACK);
        self.background_priority.fill(false);
        self.scanline = 0;
        self.scanline_cycle = 0;
    }
    pub fn step(
        &mut self,
        mapper: &mut Mapper,
        nmi: &mut bool,
        _irq: &mut bool,
        elapsed_cycles: i32,
    ) {
        for _ in 0..elapsed_cycles {
            let mut registers = self.registers.borrow_mut();

            // PPU Clocking and VRAM address updates
            let rendering_enabled = (registers.mask & 0x18) != 0;

            // VBLANK Start
            if self.scanline == 241 && self.scanline_cycle == 1 {
                registers.status |= 0x80; // Set VBLANK flag
                if (registers.control & 0x80) != 0 {
                    *nmi = true;
                }
                println!("PPU ENTERING VBLANK");
            }

            // VBLANK End, clear flags
            if self.scanline == 261 && self.scanline_cycle == 1 {
                registers.status &= !0x80; // Clear VBLANK flag
                registers.status &= !0x40; // Clear sprite zero hit
                registers.status &= !0x20; // Clear sprite overflow
            }

            // Rendering period (scanlines 0-239 and pre-render scanline 261)
            if rendering_enabled && (self.scanline < 240 || self.scanline == 261) {
                // No longer render visible scanlines at cycle 1

                // Sprite evaluation (cycles 1-64)
                if self.scanline_cycle >= 1 && self.scanline_cycle <= 64 {
                    let next_scanline = if self.scanline == 261 { 0 } else { self.scanline + 1 };
                    let mut sprite_count = 0;
                    
                    // Only evaluate sprites for visible scanlines
                    if next_scanline < 240 {
                        for i in 0..64 {
                            let offset = i * 4;
                            let sprite_y = self.oam_ram[offset];
                            let tile_height = if (registers.control & 0x20) != 0 { 16 } else { 8 };
                            
                            if next_scanline >= sprite_y as u32 && next_scanline < (sprite_y + tile_height) as u32 {
                                sprite_count += 1;
                                if sprite_count > 8 {
                                    registers.status |= 0x20; // Set sprite overflow flag
                                    break;
                                }
                            }
                        }
                    }
                }

                // Vertical VRAM increment at cycle 256
                if self.scanline_cycle == 256 {
                    // Render visible scanlines at cycle 256
                    

                    // Inlined increment_y logic
                    if (registers.vram_addr & 0x7000) != 0x7000 {
                        registers.vram_addr = registers.vram_addr.wrapping_add(0x1000);
                    } else {
                        registers.vram_addr &= 0x8FFF;
                        let y = (registers.vram_addr & 0x03E0) >> 5;

                        let y = match y {
                            29 => {
                                registers.vram_addr ^= 0x0800;
                                0
                            }
                            31 => 0,
                            _ => y.wrapping_add(1),
                        };

                        registers.vram_addr = (registers.vram_addr & 0xFC1F) | (y << 5);
                    }
                }

                // Horizontal VRAM copy from tmp_vram_addr at cycle 257
                if self.scanline_cycle == 257 {
                    registers.vram_addr = (registers.vram_addr & 0xFBE0) | (registers.tmp_vram_addr & 0x041F);
                    
                    // Reset horizontal scroll for scanlines 0-7 (non-scrolling region)
                    
                    // Render visible scanlines at cycle 257
                    
                }

                // Pre-render scanline specific VRAM copies
                if self.scanline == 261 {
                    // Vertical VRAM copy from tmp_vram_addr at cycles 280-304
                    if self.scanline_cycle >= 280 && self.scanline_cycle <= 304 {
                        registers.vram_addr = (registers.vram_addr & 0x841F) | (registers.tmp_vram_addr & 0x7BE0);
                    }
                    // Horizontal VRAM copy from tmp_vram_addr at cycles 328 and 336
                    if self.scanline_cycle == 328 || self.scanline_cycle == 336 {
                        registers.vram_addr = (registers.vram_addr & 0xFBE0) | (registers.tmp_vram_addr & 0x041F);
                    }
                }
            }

            // Increment cycle and scanline
            self.scanline_cycle += 1;

            // No longer render scanline here
            if self.scanline_cycle >= 341 {
                self.scanline_cycle = 0;
                
                // No need to render scanline here anymore
                self.scanline += 1;
                if self.scanline < 240 {
                    drop(registers); // Release mutable borrow before calling render_scanline
                    self.render_scanline(mapper); // Re-borrow after render_scanline
                }
                if self.scanline > 261 {
                    self.scanline = 0;
                }
            }
        }
    }

    fn render_scanline(&mut self, mapper: &mut Mapper) {
        
        let base_offset = self.scanline as usize * SCREEN_WIDTH;
        for color in self.frame_buffer[base_offset..base_offset + SCREEN_WIDTH].iter_mut() {
            *color = Color::RGBA(0, 0, 0, 255);
        }
        for prior in self.background_priority[base_offset..base_offset + SCREEN_WIDTH].iter_mut() {
            *prior = false;
        }

        self.render_background(mapper);
        self.render_sprites(mapper);
    }
    pub fn read(&self, mapper: &Mapper, addr: u16) -> u8 {
        let addr = addr & 0x3FFF;

        match addr {
            0..=0x1FFF => mapper.ppu_read(addr),
            0x2000..=0x3EFF => {
                let mirrored = Self::mirror_vram_addr(mapper, addr) as usize;
                self.vram[mirrored as usize]
            }
            0x3F00..=0x3FFF => {
                let mut mirrored = addr & 0x1F;
                if mirrored >= 0x10 && (mirrored % 4) == 0 {
                    mirrored -= 0x10;
                }
                self.palette_ram[mirrored as usize]
            }
            _ => 0,
        }
    }
    pub fn read_register(&self, mapper: &Mapper, addr: u16) -> u8 {
        match addr {
            0x2000 => self.registers.borrow().control,
            0x2002 => {
                let mut registers = self.registers.borrow_mut();
                let result = registers.status;
                registers.status &= !(0x80 | 0x40 | 0x20);
                registers.address_latch = false;
                result
            }
            0x2004 => self.oam_ram[self.registers.borrow().oam_addr as usize],
            0x2006 => (self.registers.borrow().ppu_addr >> 8) as u8,
            0x2007 => {
                let mut result = self.registers.borrow().data_buffer;
                let ppu_addr = self.registers.borrow().ppu_addr;
                let control = self.registers.borrow().control;

                self.registers.borrow_mut().data_buffer = self.read(mapper, ppu_addr);

                if ppu_addr >= 0x3F00 {
                    result = self.registers.borrow().data_buffer;
                }

                self.registers.borrow_mut().ppu_addr = if (control & 0x04) != 0 {
                    ppu_addr.wrapping_add(32)
                } else {
                    ppu_addr.wrapping_add(1)
                };

                result
            }
            _ => 0,
        }
    }
    fn write(&mut self, mapper: &mut Mapper, addr: u16, val: u8) {
        let addr = addr & 0x3FFF;

        match addr {
            0x0000..=0x1FFF => {
                mapper.ppu_write(addr, val);
            }
            0x2000..=0x3EFF => {
                let mirrored = Self::mirror_vram_addr(mapper, addr);
                self.vram[mirrored as usize] = val;
            }
            0x3F00..=0x3FFF => {
                let mut mirrored = addr & 0x1F;
                if mirrored >= 0x10 && (mirrored % 4) == 0 {
                    mirrored -= 0x10;
                }
                self.palette_ram[mirrored as usize] = val;
            }
            _ => {}
        }
    }
    pub fn write_register(&mut self, mapper: &mut Mapper, addr: u16, val: u8) {
        match addr {
            0x2000 => {
                let mut reg = self.registers.borrow_mut();
                reg.control = val;
                // Update nametable select bits in tmp_vram_addr from PPUCTRL
                reg.tmp_vram_addr = (reg.tmp_vram_addr & 0xF3FF) | ((val as u16 & 0x03) << 10);
            }
            0x2001 => {
                self.registers.borrow_mut().mask = val;
            }
            0x2002 => {
                let mut reg = self.registers.borrow_mut();
                (*reg).status &= 0x7F;
                (*reg).address_latch = false;
            }
            0x2003 => {
                self.registers.borrow_mut().oam_addr = val;
            }
            0x2004 => {
                let mut reg = self.registers.borrow_mut();
                reg.oam_data = val;
                self.oam_ram[reg.oam_addr as usize] = reg.oam_data;
                reg.oam_addr = reg.oam_addr.wrapping_add(1);
            }
            0x2005 => {
                let mut reg = self.registers.borrow_mut();
                if !reg.address_latch {
                    reg.scroll_x = val;
                    reg.fine_x = val & 0x07;
                    let t = reg.tmp_vram_addr;
                    reg.tmp_vram_addr = (t & 0xFFE0) | (val as u16 >> 3);
                } else {
                    reg.scroll_y = val;
                    reg.tmp_vram_addr = (reg.tmp_vram_addr & 0x8FFF) | ((val as u16 & 0x07) << 12);
                    reg.tmp_vram_addr = (reg.tmp_vram_addr & 0xFC1F) | ((val as u16 & 0xF8) << 2);
                }
                reg.address_latch = !reg.address_latch;
            }
            0x2006 => {
                let mut reg = self.registers.borrow_mut();
                if !reg.address_latch {
                    reg.tmp_vram_addr = ((val as u16) << 8) | (reg.tmp_vram_addr & 0xFF);
                    reg.ppu_addr = reg.tmp_vram_addr;
                } else {
                    reg.tmp_vram_addr = (reg.tmp_vram_addr & 0xFF00) | (val as u16);
                    reg.ppu_addr = reg.tmp_vram_addr;
                    reg.vram_addr = reg.tmp_vram_addr;
                }
                reg.address_latch = !reg.address_latch;
            }
            0x2007 => {
                self.registers.borrow_mut().ppu_data = val;
                let ppu_addr = self.registers.borrow().ppu_addr;
                self.write(mapper, ppu_addr, val);
                let control = self.registers.borrow().control;
                self.registers.borrow_mut().ppu_addr = if (control & 0x04) != 0 {
                    ppu_addr.wrapping_add(32)
                } else {
                    ppu_addr.wrapping_add(1)
                };
            }
            _ => {}
        }
    }
    fn render_background(&mut self, mapper: &Mapper) {
        if (self.registers.borrow().mask & 0x08) == 0 {
            return;
        }

        let mut current_vram_addr = self.registers.borrow().vram_addr;
        for tile_num in 0..33 {
            let coarse_x = current_vram_addr & 0x001F;
            let coarse_y = (current_vram_addr >> 5) & 0x001F;
            let name_table = (current_vram_addr >> 10) & 0x0003;

            let base_name_table_addr = 0x2000 + (name_table << 10);
            let tile_addr = base_name_table_addr + (coarse_y << 5) + coarse_x;
            let tile_idx = self.read(mapper, tile_addr) as u16;

            let fine_y = (current_vram_addr >> 12) & 0x07;

            let pattern_table = if (self.registers.borrow().control & 0x10) != 0 {
                0x1000
            } else {
                0x0000
            };
            let pattern_addr = pattern_table + (tile_idx << 4) + fine_y;

            let plane0 = self.read(mapper, pattern_addr);
            let plane1 = self.read(mapper, pattern_addr.wrapping_add(8));

            let attribute_x = coarse_x >> 2;
            let attribute_y = coarse_y >> 2;
            let attribute_addr = base_name_table_addr
                .wrapping_add(0x3C0)
                .wrapping_add(attribute_y << 3)
                .wrapping_add(attribute_x);
            let attribute_byte = self.read(mapper, attribute_addr);

            let attribute_shift = ((coarse_y & 0x02) << 1) | (coarse_x & 0x02);
            let palette_idx = (attribute_byte >> attribute_shift) & 0x03;

            // Get fine_x, but set to 0 for scanlines 0-7 (non-scrolling region)
            let current_fine_x = if self.scanline < 8 {
                0
            } else {
                self.registers.borrow().fine_x as i32
            };

            for i in 0..8 {
                let pixel_x_on_screen = (tile_num as i32 * 8) + i as i32 - current_fine_x;

                // Apply background leftmost 8-pixel mask
                if pixel_x_on_screen < 8 && (self.registers.borrow().mask & 0x02) == 0 {
                    continue;
                }

                if pixel_x_on_screen < 0 || pixel_x_on_screen >= SCREEN_WIDTH as i32 {
                    continue;
                }

                let bit_idx = 7 - i as u8;
                let bit0 = (plane0 >> bit_idx) & 1;
                let bit1 = (plane1 >> bit_idx) & 1;
                let color_idx = bit0 | (bit1 << 1);

                let screen_coor = (self.scanline as i32 * (SCREEN_WIDTH as i32) + pixel_x_on_screen) as usize;
                if color_idx != 0 {
                    self.background_priority[screen_coor] = true;
                }

                self.frame_buffer[screen_coor] =
                    self.fetch_background_color(color_idx, palette_idx);
            }
            // Inlined increment_x logic
            if (current_vram_addr & 0x001F) == 31 {
                current_vram_addr &= 0xFFE0;
                current_vram_addr ^= 0x0400; // Toggle nametable X
            } else {
                current_vram_addr += 1; // Increment coarse X
            }
        }
    }

    fn render_sprites(&mut self, mapper: &Mapper) {
        if (self.registers.borrow_mut().mask & 0x10) == 0 {
            return;
        }

        let mut pixel_drawn = vec![false; SCREEN_WIDTH];
        let mut scanline_sprites = 0;
        for i in 0..64 {
            let offset = i * 4;
            let (sprite_y, tile_idx, attributes, sprite_x) = (
                self.oam_ram[offset],
                self.oam_ram[offset + 1],
                self.oam_ram[offset + 2],
                self.oam_ram[offset + 3],
            );

            let palette_idx = attributes & 0x03;
            let flip_x = (attributes & 0x40) != 0;
            let flip_y = (attributes & 0x80) != 0;
            let priority = (attributes & 0x20) == 0;

            let is_8x16 = (self.registers.borrow().control & 0x20) != 0;

            let tile_height = if is_8x16 { 16 } else { 8 };

            if self.scanline < sprite_y as u32 || self.scanline >= (sprite_y + tile_height) as u32 {
                continue;
            }

            let sub_y = if flip_y {
                tile_height - 1 - (self.scanline as u8 - sprite_y)
            } else {
                self.scanline as u8 - sprite_y
            };

            let subtile_idx = if is_8x16 {
                (tile_idx & 0xFE) + (sub_y / 8)
            } else {
                tile_idx
            } as u16;

            let pattern_table = match is_8x16 {
                true => {
                    if (tile_idx & 1) != 0 {
                        0x1000
                    } else {
                        0x0000
                    }
                }
                false => {
                    if (self.registers.borrow().control & 0x08) != 0 {
                        0x1000
                    } else {
                        0x0000
                    }
                }
            };
            let base_addr = pattern_table + (subtile_idx << 4);

            let plane0 = self.read(mapper, base_addr + (sub_y % 8) as u16);
            let plane1 = self.read(mapper, base_addr + (sub_y as u16 % 8) + 8);

            for bit in 0..8 {
                let shift = if flip_x { bit } else { 7 - bit };

                let bit0 = (plane0 >> shift) & 1;
                let bit1 = (plane1 >> shift) & 1;

                let color = bit0 | (bit1 << 1);

                if color == 0 {
                    continue;
                }

                let pixel_x = sprite_x as usize + bit as usize;

                // Apply sprite leftmost 8-pixel mask
                if pixel_x < 8 && (self.registers.borrow().mask & 0x04) == 0 {
                    continue;
                }

                if pixel_x >= SCREEN_WIDTH {
                    continue;
                }
                let idx = self.scanline as usize * SCREEN_WIDTH + pixel_x;
                if i == 0 && self.background_priority[idx] && color != 0 {
                    self.registers.borrow_mut().status |= 0x40;
                }

                if pixel_drawn[pixel_x] {
                    continue;
                }

                if !priority && self.background_priority[idx] {
                    continue;
                }
                scanline_sprites += 1;
                self.frame_buffer[idx] = self.fetch_sprite_color(color, palette_idx);
                pixel_drawn[pixel_x] = true;
            }
        }
    }

    fn fetch_background_color(&self, color_idx: u8, palette_idx: u8) -> Color {
        if color_idx == 0 {
            let bg_color_idx = self.palette_ram[0] as usize;
            return NES_COLOR_PALETTE[bg_color_idx & 63];
        }
        let palette_base = (palette_idx << 2).wrapping_add(1);
        let palette_ram_idx = palette_base.wrapping_add(color_idx.wrapping_sub(1)) as usize;
        let palette_color_idx = self.palette_ram[palette_ram_idx] as usize;

        NES_COLOR_PALETTE[palette_color_idx & 63]
    }
    fn fetch_sprite_color(&self, color_idx: u8, palette_idx: u8) -> Color {
        let palette_base = 0x11 + (palette_idx << 2);
        let palette_color_idx =
            self.palette_ram[palette_base as usize + (color_idx - 1) as usize] as usize;
        NES_COLOR_PALETTE[palette_color_idx & 63]
    }
    fn mirror_vram_addr(mapper: &Mapper, addr: u16) -> u16 {
        let offset = addr & 0xFFF;

        let nt_idx = (offset / 0x400) as usize;
        let inner_offset = (offset % 0x400) as usize;

        use MirrorMode::*;
        match mapper.get_mirror_mode() {
            Vertical => ((nt_idx % 2) * 0x400 + inner_offset) as u16,
            Horizontal => ((nt_idx / 2) * 0x400 + inner_offset) as u16,
            SingleScreenA => inner_offset as u16,
            SingleScreenB => (0x400 + inner_offset) as u16,
        }
    }
}

const NES_COLOR_PALETTE: [Color; 64] = [
    Color::RGBA(84, 84, 84, 255),
    Color::RGBA(0, 30, 116, 255),
    Color::RGBA(8, 16, 144, 255),
    Color::RGBA(48, 0, 136, 255),
    Color::RGBA(68, 0, 100, 255),
    Color::RGBA(92, 0, 48, 255),
    Color::RGBA(84, 4, 0, 255),
    Color::RGBA(60, 24, 0, 255),
    Color::RGBA(32, 42, 0, 255),
    Color::RGBA(8, 58, 0, 255),
    Color::RGBA(0, 64, 0, 255),
    Color::RGBA(0, 60, 0, 255),
    Color::RGBA(0, 50, 60, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(152, 150, 152, 255),
    Color::RGBA(8, 76, 196, 255),
    Color::RGBA(48, 50, 236, 255),
    Color::RGBA(92, 30, 228, 255),
    Color::RGBA(136, 20, 176, 255),
    Color::RGBA(160, 20, 100, 255),
    Color::RGBA(152, 34, 32, 255),
    Color::RGBA(120, 60, 0, 255),
    Color::RGBA(84, 90, 0, 255),
    Color::RGBA(40, 114, 0, 255),
    Color::RGBA(8, 124, 0, 255),
    Color::RGBA(0, 118, 40, 255),
    Color::RGBA(0, 102, 120, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(236, 238, 236, 255),
    Color::RGBA(76, 154, 236, 255),
    Color::RGBA(120, 124, 236, 255),
    Color::RGBA(176, 98, 236, 255),
    Color::RGBA(228, 84, 236, 255),
    Color::RGBA(236, 88, 180, 255),
    Color::RGBA(236, 106, 100, 255),
    Color::RGBA(212, 136, 32, 255),
    Color::RGBA(160, 170, 0, 255),
    Color::RGBA(116, 196, 0, 255),
    Color::RGBA(76, 208, 32, 255),
    Color::RGBA(56, 204, 108, 255),
    Color::RGBA(56, 180, 204, 255),
    Color::RGBA(60, 60, 60, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(236, 238, 236, 255),
    Color::RGBA(168, 204, 236, 255),
    Color::RGBA(188, 188, 236, 255),
    Color::RGBA(212, 178, 236, 255),
    Color::RGBA(236, 174, 236, 255),
    Color::RGBA(236, 174, 212, 255),
    Color::RGBA(236, 180, 176, 255),
    Color::RGBA(228, 196, 144, 255),
    Color::RGBA(204, 210, 120, 255),
    Color::RGBA(180, 222, 120, 255),
    Color::RGBA(168, 226, 144, 255),
    Color::RGBA(152, 226, 180, 255),
    Color::RGBA(160, 214, 228, 255),
    Color::RGBA(160, 162, 160, 255),
    Color::RGBA(0, 0, 0, 255),
    Color::RGBA(0, 0, 0, 255),
];
