
use std::{rc::Rc, cell::RefCell};

use crate::cartridge::{Mapper, MirrorMode};







pub struct PPURegisters {
    pub control : u8,
    pub mask : u8,
    pub status : u8,
    pub oam_addr : u8,
    pub oam_data : u8,
    pub scroll_x : u8,
    pub scroll_y : u8,
    pub ppu_addr : u16,
    pub ppu_data : u8,
    pub scroll_latch : bool,
    pub address_latch : bool,
    pub fine_x : u8,
    pub vram_addr : u16,
    pub tmp_vram_addr : u16,
    pub data_buffer : u8
}


impl PPURegisters {
    pub fn new()->Self {
        PPURegisters { 
            control:0, 
            mask: 0, 
            status: 0, 
            oam_addr: 0, 
            oam_data: 0,
            ppu_addr: 0,
            ppu_data: 0, 
            scroll_x: 0, 
            scroll_y: 0, 
            scroll_latch: false,
            address_latch: false,
            fine_x: 0, 
            vram_addr: 0, 
            tmp_vram_addr: 0, 
            data_buffer : 0
        }
    }
    
}


pub struct PPU {
    //need interior mutability since a read from the registers might cause other registers to change.
    pub registers : Rc<RefCell<PPURegisters>>,
    vram : Vec<u8>,
    palette_ram : [u8;32],
    pub oam_ram : [u8;256],
    frame_buffer : Vec<u8>
}


impl PPU {
    pub const SCREEN_WIDTH : usize = 256;
    pub const SCREEN_HEIGHT : usize = 240;
    pub fn new()->Self {
        PPU{
            registers:Rc::new(RefCell::new(PPURegisters::new())),
            vram:vec![0;2048],
            palette_ram:[0;32],
            oam_ram:[0;256],
            frame_buffer:vec![0;Self::SCREEN_WIDTH * Self::SCREEN_HEIGHT]
        }
    }
    pub fn read(&self, mapper : &Mapper, addr : u16)-> u8 {
        let addr = addr & 0x3FFF;

        match addr {
            0..=0x1FFF=>mapper.ppu_read(addr),
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
            _=>0
        }
    }
    pub fn read_register(&self, mapper: &Mapper, addr : u16) -> u8 {
        match addr {
            0x2000=>{
                self.registers.borrow().control
            }
            0x2002=>{
                let result = self.registers.borrow().status;
                self.registers.borrow_mut().status &= 0x3F;
                self.registers.borrow_mut().address_latch = false;
                result
            }
            0x2004=>{
                self.oam_ram[self.registers.borrow().oam_addr as usize]
            }
            0x2006=>{
                (self.registers.borrow().ppu_addr >> 8) as u8
            }
            0x2007=>{
                let mut result = self.registers.borrow().data_buffer;
                let ppu_addr = self.registers.borrow().ppu_addr;
                let control = self.registers.borrow().control;
                
                self.registers.borrow_mut().data_buffer = self.read(mapper,ppu_addr);
                
                if ppu_addr >= 0x3F00 {
                    result = self.registers.borrow().data_buffer;
                }

                self.registers.borrow_mut().ppu_addr = if (control & 0x04) != 0 {ppu_addr.wrapping_add(32)} else {ppu_addr.wrapping_add(1)};

                result
            }
            _=>0
        }
    }
    fn write(&mut self, mapper : &mut Mapper, addr : u16, val :u8) {

        let addr = addr & 0x3FFF;

        match addr {
            0x0000..=0x1FFF=>{
                mapper.ppu_write(addr, val);
            }
            0x02000..=0x3EFF=>{
                let mirrored = Self::mirror_vram_addr(mapper, addr);
                self.vram[mirrored as usize] = val;
            }
            0x3F00..=0x3FFF=>{
                let mut mirrored = addr & 0x1F;
                if mirrored >= 0x10 && (mirrored % 4) == 0 {
                    mirrored -= 0x10;
                }
                self.palette_ram[mirrored as usize] = val;
            }
            _=>{}
        }



    }
    pub fn write_register(&mut self, mapper : &mut Mapper, addr : u16, val : u8) {
        match addr {
            0x2000 => {
                self.registers.borrow_mut().control = val;
            }
            0x2001 =>{
                self.registers.borrow_mut().mask = val;
            }
            0x2002=>{
                let mut reg = self.registers.borrow_mut();
                (*reg).status &= 0x7F;
                (*reg).scroll_latch = false;
            }
            0x2003 => {
                self.registers.borrow_mut().oam_addr = val;
            }
            0x2004=>{
               let mut reg = self.registers.borrow_mut();
               reg.oam_data = val;
               self.oam_ram[reg.oam_addr as usize] = reg.oam_data;
               reg.oam_addr = reg.oam_addr.wrapping_add(1);
            }
            0x2005=>{
                let mut reg = self.registers.borrow_mut();
                if !reg.scroll_latch {
                    reg.scroll_x = val;
                    reg.fine_x = val & 0x07;
                    let t = reg.tmp_vram_addr;
                    reg.tmp_vram_addr = (t & 0xFFE0) | (val as u16 >> 3); 

                }
                else {
                    reg.scroll_y = val;
                    let t = reg.tmp_vram_addr;
                    reg.tmp_vram_addr = (t & 0xBFFF) | ((val as u16 & 0x07) << 12);
                    let t = reg.tmp_vram_addr;
                    reg.tmp_vram_addr = (t & 0xFC1F) | ((val as u16  & 0xF8 ) << 3);
                }
            }
            0x2006=>{
                let mut reg = self.registers.borrow_mut();
                if !reg.address_latch {
                    reg.tmp_vram_addr = ((val as u16) << 8) | (reg.tmp_vram_addr & 0xFF);
                    reg.ppu_addr = reg.tmp_vram_addr;
                }
                else{
                    reg.tmp_vram_addr = (reg.tmp_vram_addr & 0xFF00) | (val as u16);
                    reg.ppu_addr = reg.tmp_vram_addr;
                    reg.vram_addr = reg.tmp_vram_addr;
                }
                reg.address_latch = !reg.address_latch;
            }
            0x2007=>{
                self.registers.borrow_mut().ppu_data = val;
                let ppu_addr = self.registers.borrow().ppu_addr;
                self.write(mapper,ppu_addr,val);
                let control = self.registers.borrow().control;
                self.registers.borrow_mut().ppu_addr = if (control & 0x04) != 0 {ppu_addr.wrapping_add(32)} else {ppu_addr.wrapping_add(1)};
            }
            _=>{}
        }
    }

    fn mirror_vram_addr(mapper : &Mapper,addr : u16) -> u16{
        let offset = addr & 0xFFF;

        let nt_idx = (offset/0x400) as usize;
        let inner_offset = (offset % 0x400) as usize;

        use MirrorMode::*;
        match mapper.get_mirror_mode() {
            Vertical=>((nt_idx % 2) * 0x400 + inner_offset) as u16,
            Horizontal=>((nt_idx/2) * 0x400 + inner_offset) as u16,
            SingleScreenA=>inner_offset as u16,
            SingleScreenB=>(0x400 + inner_offset) as u16
        }
    
    }
    
}




