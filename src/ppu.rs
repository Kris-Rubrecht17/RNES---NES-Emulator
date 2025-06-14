use std::ops::{BitAndAssign, BitOr};
use std::{ops::BitAnd, rc::Rc};
use std::cell::RefCell;
use sdl2::pixels::Color;

use crate::cartridge::{Mapper,MirrorMode};

pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 240;
pub const SCANLINE_DOTS: u32 = 256;
pub const SCANLINE_END_CYCLE : u32 = 340;
pub(self) enum PPUPhase {
    PreRender,
    Render,
    PostRender,
    VBlank
}

#[repr(u8)]
pub(self) enum StatusFlags {
    VBlank = 1 << 7,
    SpriteZeroHit = 1 << 6,
    SpriteOverflow = 1 << 5
}
#[repr(u8)]
pub(self) enum ContolFlags {
    GenerateInterrupt = 0x80,
    TallSprites = 0x20,
    BgPage = 0x10,
    SpritePage = 0x08
}

#[repr(u8)]
pub(self) enum MaskFlags {
    GreyScale = 1,
    ShowEdgeBG = 2,
    ShowEdgeSprites = 4,
    ShowBackground = 8,
    ShowSprites = 0x10
}

impl BitAnd<u8> for StatusFlags{
    type Output = u8;
    fn bitand(self, rhs: u8) -> Self::Output {
        self as u8 & rhs
    }
}
impl BitAnd<StatusFlags> for u8 {
    type Output = u8;
    fn bitand(self, rhs: StatusFlags) -> Self::Output {
        self & rhs as u8
    }
}
impl BitOr<StatusFlags> for StatusFlags {
    type Output = u8;
    fn bitor(self, rhs: StatusFlags) -> Self::Output {
        self as u8 | rhs as u8
    }
}
impl BitAndAssign<StatusFlags> for u8 {
    
    fn bitand_assign(&mut self, rhs: StatusFlags) {
        *self &= rhs as u8;
    }
}
impl BitAnd<u8> for MaskFlags{
    type Output = u8;
    fn bitand(self, rhs: u8) -> Self::Output {
        self as u8 & rhs
    }
}
impl BitAnd<MaskFlags> for u8 {
    type Output = u8;
    fn bitand(self, rhs: MaskFlags) -> Self::Output {
        self & rhs as u8
    }
}
impl BitOr<MaskFlags> for MaskFlags {
    type Output = u8;
    fn bitor(self, rhs: MaskFlags) -> Self::Output {
        self as u8 | rhs as u8
    }
}
impl BitAndAssign<MaskFlags> for u8 {
    
    fn bitand_assign(&mut self, rhs: MaskFlags) {
        *self &= rhs as u8;
    }
}


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
    back_buffer: Box<[Color; SCREEN_HEIGHT * SCREEN_WIDTH]>,
    pub frame_buffer: Box<[Color; SCREEN_HEIGHT * SCREEN_WIDTH]>,
    background_priority: Box<[bool; SCREEN_HEIGHT * SCREEN_WIDTH]>,
    scanline: u32,
    scanline_cycle: u32,
    current_phase : PPUPhase,
    even_frame:bool,
    line_sprites:Vec<u8>
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            registers: Rc::new(RefCell::new(PPURegisters::new())),
            vram: vec![0; 2048],
            palette_ram: [0; 32],
            oam_ram: [0; 256],
            back_buffer: Box::new([Color::BLACK; SCREEN_HEIGHT * SCREEN_WIDTH]),
            frame_buffer: Box::new([Color::BLACK; SCREEN_HEIGHT * SCREEN_WIDTH]),
            background_priority: Box::new([false; SCREEN_HEIGHT * SCREEN_WIDTH]),
            scanline: 0,
            scanline_cycle: 0,
            current_phase:PPUPhase::PreRender,
            even_frame:true,
            line_sprites:Vec::with_capacity(8)
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
        _irq: &mut bool
    ){
        use PPUPhase::*;
        
        match self.current_phase {
            PreRender=>{
                if self.scanline_cycle == 1 {
                    use StatusFlags::*;
                    let mut reg = self.registers.borrow_mut();
                    reg.status &= !(VBlank | SpriteZeroHit);
                }
                else if self.scanline_cycle == SCANLINE_DOTS + 2 && 
                    self.get_mask_flag(MaskFlags::ShowBackground) &&
                    self.get_mask_flag(MaskFlags::ShowSprites) {
                        let mut reg = self.registers.borrow_mut();
                        let t = reg.tmp_vram_addr;
                        reg.vram_addr &= !0x41F;
                        reg.vram_addr |= t & 0x41F;
                }
                else if (281..=304).contains(&self.scanline_cycle) && self.get_mask_flag(MaskFlags::ShowBackground)
                && self.get_mask_flag(MaskFlags::ShowSprites) {
                    let mut reg = self.registers.borrow_mut();
                    let t = reg.tmp_vram_addr;
                    reg.vram_addr &= !0x7BE0;
                    reg.vram_addr |= t & 0x7BE0;
                }
                else if self.scanline_cycle >= (SCANLINE_END_CYCLE - self.even_frame_adjustment()) {
                    self.current_phase = Render;
                    self.scanline_cycle = 0;
                    self.scanline = 0;
                }
            }
            Render=>{
                if self.scanline_cycle > 0 && self.scanline_cycle <= SCANLINE_DOTS {
                    let vram_addr = self.registers.borrow().vram_addr;
                    
                    let x = self.scanline_cycle - 1;
                    let y = self.scanline;
                    let screen_coor = y as usize * SCREEN_WIDTH + x as usize;

                    let mut sprite_color = 0;
                    let mut sprite_palette_idx = 0;
                    let mut sprite_foreground = false;
                    
                    if self.get_mask_flag(MaskFlags::ShowBackground) {
                        let x_fine = (self.registers.borrow().scroll_x + x as u8) % 8;

                        if self.get_mask_flag(MaskFlags::ShowEdgeBG) || x >= 8 {
                            let mut addr = 0x2000 | (vram_addr & 0x0FFF);
                            let tile = self.read(mapper,addr);

                            addr = tile as u16 * 16 + ((vram_addr >> 12) & 0x07);
                            addr |= self.get_bg_page();

                            let mut bg_color = (self.read(mapper,addr) >> (7 ^ x_fine)) & 1;
                            bg_color |= ((self.read(mapper,addr + 8) >> (7 ^ x_fine)) & 1) << 1;

                            self.background_priority[screen_coor] = bg_color != 0;

                            addr = 0x23C0 | (vram_addr & 0x0C00) | ((vram_addr >> 4) & 0x38) | ((vram_addr >> 2) & 0x07);

                            let attribute = self.read(mapper,addr);
                            let shift = (((vram_addr >> 4) & 0x04) | (vram_addr & 0x02)) as u8;

                            let palette_idx = (attribute >> shift) & 0x03;
                            self.back_buffer[screen_coor] = self.fetch_background_color(bg_color, palette_idx);
                        }
                        if x_fine == 7 {
                            let mut reg = self.registers.borrow_mut();
                            if (reg.vram_addr & 0x1F) == 31 {
                                reg.vram_addr &= !0x1F;
                                reg.vram_addr ^= 0x0400;
                            }
                            else {
                                reg.vram_addr += 1;
                            }
                        }
                    }

                    if self.get_mask_flag(MaskFlags::ShowSprites) && (self.get_mask_flag(MaskFlags::ShowEdgeSprites) || x >= 8) {
                        for idx in self.line_sprites.iter().map(|item|*item as usize) {
                            let sprite_x = self.oam_ram[idx * 4 + 3] as u32;

                            if x < sprite_x || x >= (sprite_x + 8) {
                                continue;
                            }

                            let (sprite_y,
                                tile,
                                attribute) = (
                                    (self.oam_ram[idx * 4] as u32) + 1,
                                    self.oam_ram[idx * 4 + 1] as u16,
                                    self.oam_ram[idx * 4 + 2]
                                );
                            
                            let sprite_height = self.get_sprite_height();
                            let mut x_shift = (x - sprite_x) % 8;
                            let mut y_offset = (y - sprite_y) % sprite_height;

                            if (attribute & 0x40) == 0 {
                                x_shift ^= 7;
                            }
                            if (attribute & 0x80) != 0 {
                                y_offset ^= sprite_height - 1;
                            }
                            let mut addr = 0;

                            if sprite_height == 8 {
                                addr = tile * 16 + y_offset as u16;
                                addr += self.get_sprite_page();    
                            }
                            else {
                                let tile_offset = if y_offset >= 8 { 1 } else { 0 };
                                let fine_y = y_offset & 7;
                                addr = ((tile & 0xFE) as u16 + tile_offset as u16) * 16 + fine_y as u16;
                                addr |= (tile & 1) << 12;
                            }

                            sprite_color |= (self.read(mapper,addr) >> x_shift) & 0x01;
                            sprite_color |= ((self.read(mapper,addr + 8) >> x_shift) & 0x01) << 1;

                            if sprite_color == 0 {
                                continue;
                            }
                            sprite_palette_idx = attribute & 0x03;
                            sprite_foreground = (attribute & 0x20) == 0;

                            if !self.get_status_flag(StatusFlags::SpriteZeroHit) && self.get_mask_flag(MaskFlags::ShowBackground) && idx == 0
                            && self.background_priority[screen_coor] && sprite_color != 0 {
                                let mut reg = self.registers.borrow_mut();
                                reg.status |= StatusFlags::SpriteZeroHit as u8;
                            } 

                            break;
                        }
                        if !self.background_priority[screen_coor] && sprite_color != 0 || (
                            self.background_priority[screen_coor] && sprite_color != 0 && sprite_foreground
                        ) {
                            self.back_buffer[screen_coor] = self.fetch_sprite_color(sprite_color, sprite_palette_idx);
                        }
                        else if !self.background_priority[screen_coor] && sprite_color == 0 {
                            self.back_buffer[screen_coor] = self.fetch_background_color(0, 0);
                        }
                    }
                }
                else if self.scanline_cycle == SCANLINE_DOTS + 1 && self.get_mask_flag(MaskFlags::ShowBackground) {
                    let mut reg = self.registers.borrow_mut();
                    if (reg.vram_addr & 0x7000) != 0x7000 {
                        reg.vram_addr += 0x1000;
                    }
                    else {
                        reg.vram_addr &= !0x7000;
                        let mut y = (reg.vram_addr & 0x03E0) >> 5;
                        y = if y == 29 {
                            reg.vram_addr ^= 0x0800;
                            0
                        } else if y == 31 { 
                            0
                        } else {
                            y + 1
                        };

                        reg.vram_addr = (reg.vram_addr & !0x03E0) | (y << 5);
                    }
                }
                else if self.scanline_cycle == SCANLINE_DOTS + 2 && self.get_mask_flag(MaskFlags::ShowBackground) && self.get_mask_flag(MaskFlags::ShowSprites)
                {
                    let mut reg = self.registers.borrow_mut();
                    let t = reg.tmp_vram_addr;
                    reg.vram_addr &= !0x041F;
                    reg.vram_addr |= t & 0x41F;
                }

                if self.scanline_cycle >= SCANLINE_END_CYCLE {
                    self.line_sprites.clear();

                    let range = self.get_sprite_height() as i32;
                    let mut j = 0;
                    let oam_addr = self.registers.borrow().oam_addr;
                    for i in (oam_addr/4) as usize..64 {
                        let diff = self.scanline as i32 - self.oam_ram[i * 4] as i32;
                        if 0 <= diff && diff < range {
                            if j >= 8 {
                                let mut reg = self.registers.borrow_mut();
                                reg.status |= StatusFlags::SpriteOverflow as u8;
                                break;
                            }
                            self.line_sprites.push(i as u8);
                            j += 1;
                        }
                    }

                    self.scanline += 1;
                    self.scanline_cycle = 0;
                }
                if self.scanline >= 240 {
                    self.current_phase = PostRender;
                }
            }
            PostRender=>{
                if self.scanline_cycle >= SCANLINE_END_CYCLE {
                    self.scanline += 1;
                    self.scanline_cycle = 0;
                    self.current_phase = VBlank;

                    self.frame_buffer.copy_from_slice(&self.back_buffer[..]);
                }
            }
            VBlank=>{
                if self.scanline_cycle == 1 && self.scanline == 241 {
                    let mut reg = self.registers.borrow_mut();
                    reg.status |= StatusFlags::VBlank as u8;

                    if (reg.control & ContolFlags::GenerateInterrupt as u8) != 0 {
                        *nmi = true;
                    }
                }
                if self.scanline_cycle >= SCANLINE_END_CYCLE {
                    self.scanline += 1;
                    self.scanline_cycle = 0;
                }
                if self.scanline >= 261 {
                    self.current_phase = PreRender;
                    self.scanline = 0;
                    self.even_frame = !self.even_frame;
                }
            }
        }
        self.scanline_cycle += 1;
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
    fn get_status_flag(&self,flag:StatusFlags)->bool {
        (self.registers.borrow().status & flag) != 0
    }
    fn get_mask_flag(&self, flag : MaskFlags) -> bool {
        (self.registers.borrow().mask & flag) != 0
    }
    fn even_frame_adjustment(&self)->u32 {
        if !self.even_frame && self.get_mask_flag(MaskFlags::ShowBackground) && self.get_mask_flag(MaskFlags::ShowSprites){
            1
        } else {
            0
        }
    }
    fn get_bg_page(&self) -> u16 {
        if self.registers.borrow().control & 0x10 == 0 {
            0
        } else {
            1 << 12
        }
    }
    fn get_sprite_page(&self)->u16 {
        if self.registers.borrow().control & 0x08 != 0 {
            0x1000
        }
        else {
            0
        }
    }
    fn get_sprite_height(&self)->u32 {
        if self.registers.borrow().control & 0x20 != 0 {
            16
        } else {
            8
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
