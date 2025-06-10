use crate::bus::Bus;

#[derive(PartialEq)]
pub enum AddressMode {
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    Relative,
}
impl AddressMode {
    pub fn decode(self, cpu: &mut CPU) -> (u16, i32) {
        use AddressMode::*;
        match self {
            Accumulator => {
                cpu.pc = cpu.pc.wrapping_add(1);
                (0, 0)
            }
            Immediate => {
                let result = (cpu.pc, 0);
                cpu.pc = cpu.pc.wrapping_add(1);
                result
            }
            ZeroPage => {
                let addr = cpu.fetch() as u16;
                (addr, 0)
            }
            ZeroPageX => {
                let addr = cpu.fetch();
                let _ = cpu.bus.read(addr as u16);
                let effective = addr.wrapping_add(cpu.x) as u16;
                (effective, 0)
            }
            ZeroPageY => {
                let base = cpu.fetch();
                //dummy-read
                let _ = cpu.bus.read(base as u16);
                let addr = base.wrapping_add(cpu.y);
                (addr as u16, 0)
            }
            Absolute => {
                let addr = cpu.fetch_word();
                (addr, 0)
            }
            AbsoluteX => {
                let base_addr = cpu.fetch_word();
                let effective = base_addr.wrapping_add(cpu.x as u16);

                let penalty = match Self::get_crosspage_penalty(base_addr, effective) {
                    1 => {
                        let _ = cpu.bus.read((base_addr & 0xFF00) | (effective & 0x00FF));
                        1
                    }
                    _ => 0,
                };
                (effective, penalty)
            }
            AbsoluteY => {
                let base_addr = cpu.fetch_word();
                let effective = base_addr.wrapping_add(cpu.y as u16);
                let penalty = match Self::get_crosspage_penalty(base_addr, effective) {
                    1 => {
                        cpu.bus.read((base_addr & 0xFF00) | (effective & 0xFF));
                        1
                    }
                    _ => 0,
                };
                (effective, penalty)
            }
            Indirect => {
                let ptr = cpu.fetch_word();
                let lo = cpu.bus.read(ptr) as u16;
                let hi = if (ptr & 0x00FF) == 0x00FF {
                    cpu.bus.read(ptr & 0xFF00) as u16
                } else {
                    cpu.bus.read(ptr + 1) as u16
                };
                ((hi << 8) | lo, 0)
            }
            IndirectX => {
                let base = cpu.fetch();
                let _ = cpu.bus.read(base as u16);
                let ptr = base.wrapping_add(cpu.x) as u16;

                let addr =
                    cpu.bus.read(ptr) as u16 | ((cpu.bus.read((ptr + 1) & 0xFF) as u16) << 8);
                (addr, 0)
            }
            IndirectY => {
                let ptr = cpu.fetch();
                let base_addr = cpu.bus.read(ptr as u16) as u16
                    | ((cpu.bus.read(ptr.wrapping_add(1) as u16) as u16) << 8);
                let effective = base_addr.wrapping_add(cpu.y as u16);

                let penalty = match Self::get_crosspage_penalty(base_addr, effective) {
                    1 => {
                        let _ = cpu.bus.read((base_addr & 0xFF00) | (effective & 0x00FF));
                        1
                    }
                    _ => 0,
                };

                (effective, penalty)
            }
            Relative => {
                let offset = cpu.fetch() as i8;
                let effective = cpu.pc.wrapping_add_signed(offset as i16);
                let penalty = Self::get_crosspage_penalty(cpu.pc, effective);
                (effective, penalty)
            }
        }
    }
    fn get_crosspage_penalty(base: u16, effective: u16) -> i32 {
        if (base & 0xFF00) != (effective & 0xFF00) {
            1
        } else {
            0
        }
    }
}

pub enum Register {
    A,
    X,
    Y,
}
impl Register {
    pub fn get(&self, cpu: &CPU) -> u8 {
        use Register::*;
        match self {
            A => cpu.a,
            X => cpu.x,
            Y => cpu.y,
        }
    }

    pub fn get_mut<'a>(&self, cpu: &'a mut CPU) -> &'a mut u8 {
        use Register::*;
        match self {
            A => &mut cpu.a,
            X => &mut cpu.x,
            Y => &mut cpu.y,
        }
    }
}

pub struct CPU {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u16,
    pub pc: u16,
    pub status: u8,
    pub bus: Bus,
    pub ir_disable: bool,
}

impl CPU {
    pub const FLAG_C: u8 = 1;
    pub const FLAG_Z: u8 = 1 << 1;
    pub const FLAG_I: u8 = 1 << 2;
    pub const FLAG_D: u8 = 1 << 3;
    pub const FLAG_B: u8 = 1 << 4;
    pub const FLAG_U: u8 = 1 << 5;
    pub const FLAG_V: u8 = 1 << 6;
    pub const FLAG_N: u8 = 1 << 7;

    pub fn init() -> Self {
        let mut cpu = CPU {
            a: 0,
            x: 0,
            y: 0,
            sp: 0,
            pc: 0,
            bus: Bus::init(),
            status: 0,
            ir_disable: false,
        };
        cpu.reset();

        cpu
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = 0x24;
        self.pc = self.bus.read_word(0xFFFC);
    }
    pub fn set_flag(&mut self, flag: u8, to_set: bool) {
        if to_set {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }
    pub fn get_flag(&self, flag: u8) -> bool {
        (self.status & flag) != 0
    }
    pub fn fetch(&mut self) -> u8 {
        let result = self.bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        result
    }
    pub fn fetch_word(&mut self) -> u16 {
        let result = self.bus.read_word(self.pc);
        self.pc = self.pc.wrapping_add(2);
        result
    }
    fn push(&mut self, val: u8) {
        self.bus.write(self.sp + 0x100, val);
        self.sp = self.sp.wrapping_sub(1) & 0xFF;
    }
    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1) & 0xFF;
        self.bus.read(self.sp + 0x100)
    }
    fn push_word(&mut self, word: u16) {
        self.push((word >> 8) as u8);
        self.push((word & 0xFF) as u8);
    }
    fn pop_word(&mut self) -> u16 {
        let lo = self.pop() as u16;
        let hi = self.pop() as u16;
        (hi << 8) | lo
    }
    fn set_zn(&mut self, val: u8) {
        self.set_flag(Self::FLAG_Z, val == 0);
        self.set_flag(Self::FLAG_N, (val & 0x80) != 0);
    }

    pub fn execute_instruction(&mut self) -> i32 {
        if self.bus.nmi_request {
            self.bus.nmi_request = false;
            return self.nmi();
        }

        if self.bus.irq {
            self.bus.irq = false;
            return self.irq();
        }

        let opcode = self.fetch();
        use AddressMode::*;
        use Register::*;
        match opcode {
            //add with carry
            0x69 => self.adc(Immediate, 2),
            0x65 => self.adc(ZeroPage, 3),
            0x75 => self.adc(ZeroPageX, 4),
            0x6D => self.adc(Absolute, 4),
            0x7D => self.adc(AbsoluteX, 4),
            0x79 => self.adc(AbsoluteY, 4),
            0x61 => self.adc(IndirectX, 6),
            0x71 => self.adc(IndirectY, 5),
            //bitwise and
            0x29 => self.and(Immediate, 2),
            0x25 => self.and(ZeroPage, 3),
            0x35 => self.and(ZeroPageX, 4),
            0x2D => self.and(Absolute, 4),
            0x3D => self.and(AbsoluteX, 4),
            0x39 => self.and(AbsoluteY, 4),
            0x21 => self.and(IndirectX, 6),
            0x31 => self.and(IndirectY, 5),
            //arithmetic shift left
            0x0A => self.asl(Accumulator, 2),
            0x06 => self.asl(ZeroPage, 5),
            0x16 => self.asl(ZeroPageX, 6),
            0x0E => self.asl(Absolute, 6),
            0x1E => self.asl(AbsoluteX, 7),
            //conditional branches
            0x90 => self.brcnd(!self.get_flag(Self::FLAG_C)),
            0xB0 => self.brcnd(self.get_flag(Self::FLAG_C)),
            0xF0 => self.brcnd(self.get_flag(Self::FLAG_Z)),
            0x30 => self.brcnd(self.get_flag(Self::FLAG_N)),
            0xD0 => self.brcnd(!self.get_flag(Self::FLAG_Z)),
            0x10 => self.brcnd(!self.get_flag(Self::FLAG_N)),
            0x70 => self.brcnd(self.get_flag(Self::FLAG_V)),
            0x50 => self.brcnd(!self.get_flag(Self::FLAG_V)),
            //bit test
            0x24 => self.bit(ZeroPage, 3),
            0x2C => self.bit(Absolute, 4),
            //brk
            0x00 => self.brk(),
            //clear flags
            0x18 => self.clf(Self::FLAG_C),
            0xD8 => self.clf(Self::FLAG_D),
            0x58 => self.clf(Self::FLAG_I),
            0xB8 => self.clf(Self::FLAG_V),
            //compare reg to mem
            0xC9 => self.cmp(A, Immediate, 2),
            0xC5 => self.cmp(A, ZeroPage, 3),
            0xD5 => self.cmp(A, ZeroPageX, 3),
            0xCD => self.cmp(A, Absolute, 4),
            0xDD => self.cmp(A, AbsoluteX, 4),
            0xD9 => self.cmp(A, AbsoluteY, 4),
            0xC1 => self.cmp(A, IndirectX, 6),
            0xD1 => self.cmp(A, IndirectY, 5),
            0xE0 => self.cmp(X, Immediate, 2),
            0xE4 => self.cmp(X, ZeroPage, 3),
            0xEC => self.cmp(X, Absolute, 4),
            0xC0 => self.cmp(Y, Immediate, 2),
            0xC4 => self.cmp(Y, ZeroPage, 3),
            0xCC => self.cmp(Y, Absolute, 4),
            //deccartridge
            0xC6 => self.dec(ZeroPage, 5),
            0xD6 => self.dec(ZeroPageX, 6),
            0xCE => self.dec(Absolute, 6),
            0xDE => self.dec(AbsoluteX, 7),
            0xCA => self.dec_reg(X),
            0x88 => self.dec_reg(Y),
            //inc
            0xE6 => self.inc(ZeroPage, 5),
            0xF6 => self.inc(ZeroPageX, 6),
            0xEE => self.inc(Absolute, 6),
            0xFE => self.inc(AbsoluteX, 7),
            0xE8 => self.inc_reg(X),
            0xC8 => self.inc_reg(Y),
            //xor
            0x49 => self.xor(Immediate, 2),
            0x45 => self.xor(ZeroPage, 3),
            0x55 => self.xor(ZeroPageX, 4),
            0x4D => self.xor(Absolute, 4),
            0x5D => self.xor(AbsoluteX, 4),
            0x59 => self.xor(AbsoluteY, 4),
            0x41 => self.xor(IndirectX, 6),
            0x51 => self.xor(IndirectY, 5),
            //jmp
            0x4C => self.jmp(Absolute, 3),
            0x6C => self.jmp(Indirect, 5),
            0x20 => self.jsr(),
            //ld reg
            0xA9 => self.ldreg(A, Immediate, 2),
            0xA5 => self.ldreg(A, ZeroPage, 3),
            0xB5 => self.ldreg(A, ZeroPageX, 4),
            0xAD => self.ldreg(A, Absolute, 4),
            0xBD => self.ldreg(A, AbsoluteX, 4),
            0xB9 => self.ldreg(A, AbsoluteY, 4),
            0xA1 => self.ldreg(A, IndirectX, 6),
            0xB1 => self.ldreg(A, IndirectY, 5),
            0xA2 => self.ldreg(X, Immediate, 2),
            0xA6 => self.ldreg(X, ZeroPage, 3),
            0xB6 => self.ldreg(X, ZeroPageY, 4),
            0xAE => self.ldreg(X, Absolute, 4),
            0xBE => self.ldreg(X, AbsoluteY, 4),
            0xA0 => self.ldreg(Y, Immediate, 2),
            0xA4 => self.ldreg(Y, ZeroPage, 3),
            0xB4 => self.ldreg(Y, ZeroPageX, 4),
            0xAC => self.ldreg(Y, Absolute, 4),
            0xBC => self.ldreg(Y, AbsoluteX, 4),
            //nop
            0xEA => self.nop(),
            //logical shift right
            0x4A => self.lsr(Accumulator, 2),
            0x46 => self.lsr(ZeroPage, 5),
            0x56 => self.lsr(ZeroPageX, 6),
            0x4E => self.lsr(Absolute, 6),
            0x5E => self.lsr(AbsoluteX, 7),
            //or accumulator
            0x09 => self.or(Immediate, 2),
            0x05 => self.or(ZeroPage, 3),
            0x15 => self.or(ZeroPageX, 4),
            0x0D => self.or(Absolute, 4),
            0x1D => self.or(AbsoluteX, 4),
            0x19 => self.or(AbsoluteY, 4),
            0x01 => self.or(IndirectX, 6),
            0x11 => self.or(IndirectY, 5),
            //push/pop
            0x48 => self.pha(),
            0x08 => self.php(),
            0x68 => self.pla(),
            0x28 => self.plp(),
            //rotate left
            0x2A => self.rol(Accumulator, 2),
            0x26 => self.rol(ZeroPage, 5),
            0x36 => self.rol(ZeroPageX, 6),
            0x2E => self.rol(Absolute, 6),
            0x3E => self.rol(AbsoluteX, 7),
            //rotate right
            0x6A => self.ror(Accumulator, 2),
            0x66 => self.ror(ZeroPage, 5),
            0x76 => self.ror(ZeroPageX, 6),
            0x6E => self.ror(Absolute, 6),
            0x7E => self.ror(AbsoluteX, 7),
            //return
            0x40 => self.rti(),
            0x60 => self.rts(),
            //sub with carry
            0xE9 => self.sbc(Immediate, 2),
            0xE5 => self.sbc(ZeroPage, 3),
            0xF5 => self.sbc(ZeroPageX, 4),
            0xED => self.sbc(Absolute, 4),
            0xFD => self.sbc(AbsoluteX, 4),
            0xF9 => self.sbc(AbsoluteY, 4),
            0xE1 => self.sbc(IndirectX, 6),
            0xF1 => self.sbc(IndirectY, 5),
            //set flags
            0x38 => self.stf(Self::FLAG_C),
            0xF8 => self.stf(Self::FLAG_D),
            0x78 => self.stf(Self::FLAG_I),
            //str register
            0x85 => self.sta(ZeroPage, 3),
            0x95 => self.sta(ZeroPageX, 4),
            0x8D => self.sta(Absolute, 4),
            0x9D => self.sta(AbsoluteX, 5),
            0x99 => self.sta(AbsoluteY, 5),
            0x81 => self.sta(IndirectX, 6),
            0x91 => self.sta(IndirectY, 6),
            0x86 => self.stx(ZeroPage, 3),
            0x96 => self.stx(ZeroPageY, 4),
            0x8E => self.stx(Absolute, 4),
            0x84 => self.sty(ZeroPage, 3),
            0x94 => self.sty(ZeroPageX, 4),
            0x8C => self.sty(Absolute, 4),
            //transfer reg
            0xAA => self.trr(X, A),
            0xA8 => self.trr(Y, A),
            0x8A => self.trr(A, X),
            0x98 => self.trr(A, Y),
            0xBA => self.tsx(),
            0x9A => self.txs(),
            //undocumented
            0xA7 => self.lax(ZeroPage, 3),
            0xB7 => self.lax(ZeroPageY, 4),
            0xAF => self.lax(Absolute, 4),
            0xBF => self.lax(AbsoluteY, 4),
            0xA3 => self.lax(IndirectX, 6),
            0xB3 => self.lax(IndirectY, 5),
            0x87 => self.sax(ZeroPage, 3),
            0x97 => self.sax(ZeroPageY, 4),
            0x8F => self.sax(Absolute, 4),
            0x83 => self.sax(IndirectX, 6),
            0x04 => self.multibyte_nop(AddressMode::ZeroPage, 3),
            0x44 => self.multibyte_nop(AddressMode::ZeroPage, 3),
            0x64 => self.multibyte_nop(AddressMode::ZeroPage, 3),

            0x0C => self.multibyte_nop(AddressMode::Absolute, 4),

            0x14 => self.multibyte_nop(AddressMode::ZeroPageX, 4),
            0x34 => self.multibyte_nop(AddressMode::ZeroPageX, 4),
            0x54 => self.multibyte_nop(AddressMode::ZeroPageX, 4),
            0x74 => self.multibyte_nop(AddressMode::ZeroPageX, 4),
            0xD4 => self.multibyte_nop(AddressMode::ZeroPageX, 4),
            0xF4 => self.multibyte_nop(AddressMode::ZeroPageX, 4),

            0x1C => self.multibyte_nop(AddressMode::AbsoluteX, 4),
            0x3C => self.multibyte_nop(AddressMode::AbsoluteX, 4),
            0x5C => self.multibyte_nop(AddressMode::AbsoluteX, 4),
            0x7C => self.multibyte_nop(AddressMode::AbsoluteX, 4),
            0xDC => self.multibyte_nop(AddressMode::AbsoluteX, 4),
            0xFC => self.multibyte_nop(AddressMode::AbsoluteX, 4),
            0x1A => self.nop(),
            0x3A => self.nop(),
            0x5A => self.nop(),
            0x7A => self.nop(),
            0xDA => self.multibyte_nop(AddressMode::AbsoluteX, 4),
            0x89 => self.multibyte_nop(AddressMode::Immediate, 2),
            //unofficial sbc
            0xEB => self.sbc(Immediate, 2),
            //dcp
            0xC7 => self.dcp(AddressMode::ZeroPage, 5),
            0xD7 => self.dcp(AddressMode::ZeroPageX, 6),
            0xCF => self.dcp(AddressMode::Absolute, 6),
            0xDF => self.dcp(AddressMode::AbsoluteX, 7),
            0xDB => self.dcp(AddressMode::AbsoluteY, 7),
            0xC3 => self.dcp(AddressMode::IndirectX, 8),
            0xD3 => self.dcp(AddressMode::IndirectY, 8),
            //isb
            0xE7 => self.isb(AddressMode::ZeroPage, 5),
            0xF7 => self.isb(AddressMode::ZeroPageX, 6),
            0xEF => self.isb(AddressMode::Absolute, 6),
            0xFF => self.isb(AddressMode::AbsoluteX, 7),
            0xFB => self.isb(AddressMode::AbsoluteY, 7),
            0xE3 => self.isb(AddressMode::IndirectX, 8),
            0xF3 => self.isb(AddressMode::IndirectY, 8),
            //slo
            0x07 => self.slo(AddressMode::ZeroPage, 8),
            0x17 => self.slo(AddressMode::ZeroPageX, 6),
            0x0F => self.slo(AddressMode::Absolute, 6),
            0x1F => self.slo(AddressMode::AbsoluteX, 7),
            0x03 => self.slo(AddressMode::IndirectX, 8),
            0x13 => self.slo(AddressMode::IndirectY, 8),
            0x1B => self.slo(AddressMode::AbsoluteY, 7),
            //rla
            0x23 => self.rla(AddressMode::IndirectX, 8),
            0x27 => self.rla(AddressMode::ZeroPage, 5),
            0x2F => self.rla(AddressMode::Absolute, 6),
            0x33 => self.rla(AddressMode::IndirectY, 8),
            0x37 => self.rla(AddressMode::ZeroPageX, 6),
            0x3B => self.rla(AddressMode::AbsoluteY, 7),
            0x3F => self.rla(AddressMode::AbsoluteX, 7),
            //srx
            0x43 => self.srx(AddressMode::IndirectX, 8),
            0x47 => self.srx(AddressMode::ZeroPage, 5),
            0x4F => self.srx(AddressMode::Absolute, 6),
            0x53 => self.srx(AddressMode::IndirectY, 8),
            0x57 => self.srx(AddressMode::ZeroPageX, 6),
            0x5F => self.srx(AddressMode::AbsoluteX, 7),
            0x5B => self.srx(AddressMode::AbsoluteY, 7),
            //rra
            0x67 => self.rra(AddressMode::ZeroPage, 5),
            0x77 => self.rra(AddressMode::ZeroPageX, 6),
            0x6F => self.rra(AddressMode::Absolute, 6),
            0x7F => self.rra(AddressMode::AbsoluteX, 7),
            0x7B => self.rra(AddressMode::AbsoluteY, 7),
            0x63 => self.rra(AddressMode::IndirectX, 8),
            0x73 => self.rra(AddressMode::IndirectY, 8),
            0x32 => {
                println!("Illegal Halt!!!!!!");
                0
            }
            _ => unreachable!("Undocumented opcode reached: 0x{opcode:02X}"),
        }
    }

    fn adc(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra_cycles) = address_mode.decode(self);

        let carry_in = if self.get_flag(Self::FLAG_C) { 1 } else { 0 };
        let val = self.bus.read(addr) as u16;

        let result = self.a as u16 + val + carry_in;

        self.set_flag(Self::FLAG_C, result > 0xFF);
        self.set_zn((result & 0xFF) as u8);
        self.set_flag(
            Self::FLAG_V,
            (!(self.a ^ val as u8) & (self.a ^ result as u8) & 0x80) != 0,
        );

        self.a = result as u8;

        cycles + extra_cycles
    }

    fn and(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra) = address_mode.decode(self);

        let val = self.bus.read(addr);
        self.a &= val;

        self.set_zn(self.a);

        cycles + extra
    }

    fn asl(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        match address_mode {
            AddressMode::Accumulator => {
                //dummy read
                let _ = self.bus.read(self.pc);

                let carry_out = (self.a & 0x80) != 0;
                self.a <<= 1;
                self.set_zn(self.a);
                self.set_flag(Self::FLAG_C, carry_out);
            }
            _ => {
                /*
                    Needed for weird dummy-read quirk.
                    It seems that for read-modify-write instructions,
                    'Abolute, X' addressing expects a dummy read, even
                    when page boundaries have not been crossed. FLAG_I.e. extra == 0
                */
                let absx = address_mode == AddressMode::AbsoluteX;

                let (addr, extra) = address_mode.decode(self);

                if absx && extra == 0 {
                    //dummy-read
                    let _ = self.bus.read(addr);
                }

                let value = self.bus.read(addr);

                let carry_out = (value & 0x80) != 0;
                self.bus.write(addr, value);
                self.bus.write(addr, value << 1);

                self.set_zn(value << 1);
                self.set_flag(Self::FLAG_C, carry_out);
            }
        }

        cycles
    }

    fn brcnd(&mut self, condition: bool) -> i32 {
        let (addr, extra) = AddressMode::Relative.decode(self);
        //dummy-read

        if condition {
            self.bus.read(self.pc);
            if (self.pc & 0xFF00) != (addr & 0xFF00) {
                self.bus.read((self.pc & 0xFF00) | (addr & 0xFF));
            }
            self.pc = addr;
            3 + extra
        } else {
            2
        }
    }
    fn bit(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, _) = address_mode.decode(self);
        let value = self.bus.read(addr);

        self.set_flag(Self::FLAG_Z, (value & self.a) == 0);
        self.set_flag(Self::FLAG_V, (value & 0b01000000) != 0);
        self.set_flag(Self::FLAG_N, (value & 0x80) != 0);

        cycles
    }
    fn brk(&mut self) -> i32 {
        //dummy read
        let _ = self.fetch();

        self.push_word(self.pc);
        self.push(self.status | Self::FLAG_B | Self::FLAG_U);

        self.set_flag(Self::FLAG_B, false);
        self.set_flag(Self::FLAG_I, true);

        self.pc = self.bus.read_word(0xFFFE);

        7
    }
    fn clf(&mut self, flag: u8) -> i32 {
        self.bus.read(self.pc);
        self.set_flag(flag, false);
        2
    }
    fn stf(&mut self, flag: u8) -> i32 {
        //dummy read
        self.bus.read(self.pc);
        self.set_flag(flag, true);
        2
    }
    fn cmp(&mut self, reg: Register, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra) = address_mode.decode(self);

        let reg = reg.get(self);
        let val = self.bus.read(addr);

        self.set_flag(Self::FLAG_Z, reg == val);
        self.set_flag(Self::FLAG_C, reg >= val);
        self.set_flag(Self::FLAG_N, (reg.wrapping_sub(val) & 0x80) != 0);

        cycles + extra
    }
    fn dec_reg(&mut self, reg: Register) -> i32 {
        {
            let r = reg.get_mut(self);
            *r = r.wrapping_sub(1);
        }
        let reg = reg.get(self);
        self.set_zn(reg);
        //dummy-read
        let _ = self.bus.read(self.pc);
        2
    }
    fn inc_reg(&mut self, reg: Register) -> i32 {
        {
            let r = reg.get_mut(self);
            *r = r.wrapping_add(1);
        }
        let reg = reg.get(self);
        self.set_zn(reg);
        //dummy-read
        let _ = self.bus.read(self.pc);
        2
    }
    fn dec(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let absx = address_mode == AddressMode::AbsoluteX;
        let (addr, extra) = address_mode.decode(self);
        let value = self.bus.read(addr);

        let res = value.wrapping_sub(1);

        if absx && extra == 0 {
            let _ = self.bus.read(addr);
        }
        self.bus.write(addr, value);
        self.bus.write(addr, res);

        self.set_zn(res);

        cycles
    }
    fn inc(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let absx = address_mode == AddressMode::AbsoluteX;

        let (addr, extra) = address_mode.decode(self);
        let value = self.bus.read(addr);

        let res = value.wrapping_add(1);

        if absx && extra == 0 {
            //dummy-read
            let _ = self.bus.read(addr);
        }
        self.bus.write(addr, value);
        self.bus.write(addr, res);

        self.set_zn(res);

        cycles
    }
    fn xor(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra) = address_mode.decode(self);

        let val = self.bus.read(addr);

        self.a ^= val;

        self.set_zn(self.a);

        cycles + extra
    }
    fn jmp(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, _) = address_mode.decode(self);
        self.pc = addr;
        cycles
    }
    fn jsr(&mut self) -> i32 {
        /*
            Another quirk of dummy reading. jsr expects a dummy read from the
            stack pointer to happen between fetching lo and hi bytes of the operand
        */
        let lo = self.fetch() as u16;

        //dummy-read
        let _ = self.bus.read(self.sp + 0x100);

        self.push_word(self.pc);

        let hi = self.fetch() as u16;

        self.pc = (hi << 8) | lo;

        6
    }
    fn ldreg(&mut self, reg: Register, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra) = address_mode.decode(self);
        let value = self.bus.read(addr);

        let reg = reg.get_mut(self);
        *reg = value;

        self.set_zn(value);

        cycles + extra
    }
    fn lsr(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        match address_mode {
            AddressMode::Accumulator => {
                let value = self.a;
                let carry_out = (self.a & 0x01) != 0;

                self.a = value >> 1;

                self.set_flag(Self::FLAG_C, carry_out);
                self.set_flag(Self::FLAG_Z, self.a == 0);
                self.set_flag(Self::FLAG_N, false);
                //dummy-read
                self.bus.read(self.pc);
            }
            _ => {
                let absx = address_mode == AddressMode::AbsoluteX;

                let (addr, extra) = address_mode.decode(self);

                if absx && extra == 0 {
                    //dummy-read
                    let _ = self.bus.read(addr);
                }

                let value = self.bus.read(addr);
                let carry_out = (value & 0x01) != 0;

                //dummy write
                self.bus.write(addr, value);

                self.bus.write(addr, value >> 1);

                self.set_flag(Self::FLAG_Z, (value >> 1) == 0);
                self.set_flag(Self::FLAG_C, carry_out);
                self.set_flag(Self::FLAG_N, false);
            }
        }

        cycles
    }
    fn nop(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);
        2
    }
    fn or(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra) = address_mode.decode(self);

        self.a |= self.bus.read(addr);

        self.set_zn(self.a);

        cycles + extra
    }
    fn pha(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);
        self.push(self.a);
        3
    }
    fn php(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);
        self.push(self.status | Self::FLAG_B | Self::FLAG_U);
        3
    }
    fn pla(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);
        let _ = self.bus.read(self.sp + 0x100);

        self.a = self.pop();
        self.set_zn(self.a);
        4
    }
    fn plp(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);
        let _ = self.bus.read(self.sp + 0x100);
        let val = self.pop();
        let status_to_write = val & !(Self::FLAG_B);

        self.status = status_to_write | Self::FLAG_U;
        4
    }
    fn rol(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        match address_mode {
            AddressMode::Accumulator => {
                //dummy-read
                self.bus.read(self.pc);

                let carry_in = if self.get_flag(Self::FLAG_C) { 1 } else { 0 };
                let carry_out = (self.a & 0x80) != 0;

                self.a = (self.a << 1) | carry_in;

                self.set_flag(Self::FLAG_C, carry_out);
                self.set_zn(self.a);
            }
            _ => {
                let absx = address_mode == AddressMode::AbsoluteX;

                let (addr, extra) = address_mode.decode(self);

                if absx && extra == 0 {
                    //dummy-read
                    let _ = self.bus.read(addr);
                }

                let value = self.bus.read(addr);

                let carry_in = if self.get_flag(Self::FLAG_C) { 1 } else { 0 };
                let carry_out = (value & 0x80) != 0;

                self.bus.write(addr, value);
                self.bus.write(addr, (value << 1) | carry_in);

                self.set_flag(Self::FLAG_C, carry_out);
                self.set_zn((value << 1) | carry_in);
            }
        }

        cycles
    }

    fn ror(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        match address_mode {
            AddressMode::Accumulator => {
                //dummy-read
                self.bus.read(self.pc);

                let carry_in = if self.get_flag(Self::FLAG_C) { 0x80 } else { 0 };
                let carry_out = (self.a & 0x01) != 0;

                self.a = (self.a >> 1) | carry_in;

                self.set_flag(Self::FLAG_C, carry_out);
                self.set_zn(self.a);
            }
            _ => {
                let absx = address_mode == AddressMode::AbsoluteX;

                let (addr, extra) = address_mode.decode(self);

                if absx && extra == 0 {
                    //dummy-read
                    let _ = self.bus.read(addr);
                }

                let value = self.bus.read(addr);

                let carry_in = if self.get_flag(Self::FLAG_C) { 0x80 } else { 0 };
                let carry_out = (value & 0x01) != 0;

                self.bus.write(addr, value);
                self.bus.write(addr, (value >> 1) | carry_in);

                self.set_flag(Self::FLAG_C, carry_out);
                self.set_zn((value >> 1) | carry_in);
            }
        }

        cycles
    }

    fn rti(&mut self) -> i32 {
        //dummy-reads
        let _ = self.bus.read(self.pc);
        let _ = self.bus.read(self.sp + 0x100);

        self.status = self.pop() & !Self::FLAG_B;
        self.status |= Self::FLAG_U;

        self.pc = self.pop_word();

        6
    }

    fn rts(&mut self) -> i32 {
        //dummy-reads
        self.bus.read(self.pc);
        self.bus.read(self.sp + 0x100);

        self.pc = self.pop_word();

        self.fetch();
        6
    }

    fn sbc(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra_cycles) = address_mode.decode(self);

        let carry_in = if self.get_flag(Self::FLAG_C) { 1 } else { 0 };
        let val = !self.bus.read(addr) as u16;

        let result = self.a as u16 + val + carry_in;

        self.set_flag(Self::FLAG_C, result > 0xFF);
        self.set_zn((result & 0xFF) as u8);
        self.set_flag(
            Self::FLAG_V,
            ((self.a ^ result as u8) & (val as u8 ^ result as u8) & 0x80) != 0,
        );

        self.a = result as u8;

        cycles + extra_cycles
    }
    fn sta(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        /*
           This instruction also seems to have a quirk where not only does
           Absolute, X always trigger a dummy-read, but so does Indirect, Y
           and Absolute, Y
        */

        let needs_dummy = address_mode == AddressMode::AbsoluteX
            || address_mode == AddressMode::AbsoluteY
            || address_mode == AddressMode::IndirectY;
        let (addr, extra) = address_mode.decode(self);
        if extra == 0 && needs_dummy {
            //dummy-read
            let _ = self.bus.read(addr);
        }

        self.bus.write(addr, self.a);

        cycles
    }
    fn stx(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, _) = address_mode.decode(self);

        self.bus.write(addr, self.x);

        cycles
    }
    fn sty(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, _) = address_mode.decode(self);

        self.bus.write(addr, self.y);

        cycles
    }
    fn trr(&mut self, dst: Register, src: Register) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);
        let value = src.get(self);
        let dst = dst.get_mut(self);
        *dst = value;
        self.set_zn(value);

        2
    }
    fn tsx(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);

        self.x = self.sp as u8;
        self.set_zn(self.x);
        2
    }
    fn txs(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.pc);

        self.sp = self.x as u16;

        2
    }

    //undocumented opcodes
    fn lax(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, extra) = address_mode.decode(self);

        let value = self.bus.read(addr);

        self.a = value;
        self.x = value;

        self.set_zn(value);

        cycles + extra
    }
    fn sax(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let (addr, _) = address_mode.decode(self);
        //dummy-read
        //let _ = self.bus.read(self.pc);

        self.bus.write(addr, self.a & self.x);

        cycles
    }

    fn multibyte_nop(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        //dummy-read
        let (_, extra) = address_mode.decode(self);

        cycles + extra
    }
    fn dcp(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let need_dummy = address_mode == AddressMode::AbsoluteX
            || address_mode == AddressMode::AbsoluteY
            || address_mode == AddressMode::IndirectY;
        let (addr, extra) = address_mode.decode(self);
        let mut val = self.bus.read(addr);

        if need_dummy && extra == 0 {
            //dummy read
            let _ = self.bus.read(addr);
        }

        self.bus.write(addr, val);
        val = val.wrapping_sub(1);
        self.bus.write(addr, val);

        let result = self.a.wrapping_sub(val);

        self.set_flag(Self::FLAG_C, self.a >= val);
        self.set_zn(result);

        cycles + extra
    }
    fn isb(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let need_dummy = address_mode == AddressMode::AbsoluteX
            || address_mode == AddressMode::AbsoluteY
            || address_mode == AddressMode::IndirectY;
        let (addr, extra) = address_mode.decode(self);
        let mut val = self.bus.read(addr);

        if need_dummy && extra == 0 {
            //dummy read
            let _ = self.bus.read(addr);
        }

        self.bus.write(addr, val);
        val = val.wrapping_add(1);
        self.bus.write(addr, val);
        let nval = !val as u16;
        let carry = if self.get_flag(Self::FLAG_C) { 1 } else { 0 };

        let result = self.a as u16 + nval + carry;

        self.set_flag(Self::FLAG_C, result > 0xFF);
        self.set_zn((result & 0xFF) as u8);
        self.set_flag(
            Self::FLAG_V,
            ((self.a ^ result as u8) & (nval as u8 ^ result as u8) & 0x80) != 0,
        );

        self.a = result as u8;

        cycles + extra
    }
    fn slo(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let need_dummy = address_mode == AddressMode::AbsoluteX
            || address_mode == AddressMode::AbsoluteY
            || address_mode == AddressMode::IndirectY;
        let (addr, extra) = address_mode.decode(self);
        let mut val = self.bus.read(addr);

        if need_dummy && extra == 0 {
            //dummy read
            let _ = self.bus.read(addr);
        }

        self.bus.write(addr, val);
        let carry_out = (val & 0x80) != 0;
        val <<= 1;
        self.bus.write(addr, val);
        self.set_flag(Self::FLAG_C, carry_out);

        self.a |= val;
        self.set_zn(self.a);

        cycles + extra
    }
    fn rla(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let need_dummy = address_mode == AddressMode::AbsoluteX
            || address_mode == AddressMode::AbsoluteY
            || address_mode == AddressMode::IndirectY;
        let (addr, extra) = address_mode.decode(self);
        let mut val = self.bus.read(addr);

        if need_dummy && extra == 0 {
            //dummy read
            let _ = self.bus.read(addr);
        }

        self.bus.write(addr, val);
        let carry_out = (val & 0x80) != 0;
        let carry_in = if self.get_flag(Self::FLAG_C) { 1 } else { 0 };
        val = (val << 1) | carry_in;
        self.bus.write(addr, val);

        self.a &= val;

        self.set_flag(Self::FLAG_C, carry_out);
        self.set_zn(self.a);

        cycles + extra
    }
    fn srx(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let need_dummy = address_mode == AddressMode::AbsoluteX
            || address_mode == AddressMode::AbsoluteY
            || address_mode == AddressMode::IndirectY;
        let (addr, extra) = address_mode.decode(self);
        let mut val = self.bus.read(addr);

        if need_dummy && extra == 0 {
            //dummy read
            let _ = self.bus.read(addr);
        }

        self.bus.write(addr, val);

        let carry = (val & 1) != 0;

        val >>= 1;

        self.bus.write(addr, val);

        self.a ^= val;
        self.set_flag(Self::FLAG_C, carry);
        self.set_zn(self.a);

        cycles + extra
    }
    fn rra(&mut self, address_mode: AddressMode, cycles: i32) -> i32 {
        let need_dummy = address_mode == AddressMode::AbsoluteX
            || address_mode == AddressMode::AbsoluteY
            || address_mode == AddressMode::IndirectY;
        let (addr, extra) = address_mode.decode(self);
        let mut val = self.bus.read(addr);

        if need_dummy && extra == 0 {
            //dummy read
            let _ = self.bus.read(addr);
        }

        self.bus.write(addr, val);

        let carry_in = if self.get_flag(Self::FLAG_C) { 0x80 } else { 0 };
        let carry_out = (val & 0x01) != 0;
        val = (val >> 1) | carry_in;

        self.bus.write(addr, val);
        self.set_flag(Self::FLAG_C, carry_out);

        let carry = if self.get_flag(Self::FLAG_C) { 1 } else { 0 };
        let sum = self.a as u16 + val as u16 + carry;

        self.set_flag(Self::FLAG_C, sum > 0xFF);

        let result = sum as u8;

        self.set_flag(
            Self::FLAG_V,
            ((self.a ^ result) & (val ^ result) & 0x80) != 0,
        );

        self.a = result;

        self.set_zn(self.a);

        cycles + extra
    }

    fn irq(&mut self) -> i32 {
        if !self.get_flag(Self::FLAG_I) {
            //dummy-read
            let _ = self.bus.read(self.sp + 0x100);

            self.push_word(self.pc);
            self.set_flag(Self::FLAG_B, false);
            self.set_flag(Self::FLAG_U, true);

            self.push(self.status);

            self.set_flag(Self::FLAG_I, true);

            self.pc = self.bus.read_word(0xFFFE);

            return 7;
        }
        0
    }
    fn nmi(&mut self) -> i32 {
        //dummy-read
        let _ = self.bus.read(self.sp + 0x100);

        self.push_word(self.pc);
        self.set_flag(Self::FLAG_B, false);
        self.set_flag(Self::FLAG_U, true);
        self.push(self.status);
        self.set_flag(Self::FLAG_I, true);

        self.pc = self.bus.read_word(0xFFFA);

        7
    }
}
