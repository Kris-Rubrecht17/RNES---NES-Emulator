use std::{cell::RefCell, rc::Rc, sync::{atomic::AtomicU8, Arc}};

use crate::{cartridge::Mapper, input::Input, ppu::PPU};

pub struct Bus {
    cartridge: Mapper,
    //using RefCell because reading input requires &mut Input,
    //which would require Bus::read to take &mut self otherwise.
    pub input: Rc<RefCell<Input>>,
    ram: Vec<u8>,
    pub ppu: PPU,
    pub irq: bool,
    pub nmi_request: bool,
    pub extra_cycles : i32
}

impl Bus {
    pub fn init() -> Self {
        Bus {
            cartridge : Mapper::None,
            input: Rc::new(RefCell::new(Input::new())),
            ram: vec![0; 2048],
            irq: false,
            nmi_request: false,
            ppu: PPU::new(),
            extra_cycles:0
        }
    }
    pub fn load_cartridge(&mut self, cartridge : Mapper) {
        self.reset();
        self.cartridge = cartridge;
    }
    pub fn reset(&mut self) {
        self.input.borrow_mut().controller_state = 0;
        self.input.borrow_mut().controller_shift = 0;
        self.ram = vec![0;2048];
        self.irq = false;
        self.nmi_request = false;
        self.ppu.reset();
        self.extra_cycles = 0;
    }
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x4016 => self.input.borrow_mut().read(),
            //
            0x0000..=0x1FFF => self.ram[addr as usize & 0x07FF],
            //
            0x2000..=0x3FFF => {
                let reg = 0x2000 + (addr & 0x07);

                self.ppu.read_register(&self.cartridge, reg)
            }
            //
            0x6000..=0xFFFF => self.cartridge.cpu_read(addr),
            _ => 0,
        }
    }
    pub fn read_word(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = (self.read(addr.wrapping_add(1)) as u16) << 8;
        hi | lo
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x4016 => self.input.borrow_mut().write(val),
            //
            0x4014 => self.write_oam_dma(val),
            //
            0x0000..=0x1FFF => self.ram[addr as usize & 0x7FF] = val,
            //
            0x2000..=0x3FFF => {
                let mapper = &mut self.cartridge;
                self.ppu.write_register(mapper, addr, val)
            }
            //
            0x6000..=0xFFFF => self.cartridge.cpu_write(addr, val),
            _ => {}
        }
    }
    fn write_oam_dma(&mut self, page: u8) {
        let base_addr = (page as u16) << 8;
        for i in 0..256 {
            let val = self.read(base_addr + i);
            let oam_addr = self.ppu.registers.borrow().oam_addr;
            self.ppu.oam_ram[oam_addr as usize] = val;
            self.ppu.registers.borrow_mut().oam_addr = oam_addr.wrapping_add(1);
        }
        self.extra_cycles = 513;
    }
    pub fn tick_ppu(&mut self, elapsed_cycles: i32) {
        let (ppu, mapper, irq, nmi) = (
            &mut self.ppu,
            &mut self.cartridge,
            &mut self.irq,
            &mut self.nmi_request,
        );

        ppu.step(mapper, nmi, irq, elapsed_cycles);
    }
}
