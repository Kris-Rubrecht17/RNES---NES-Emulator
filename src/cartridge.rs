
#[derive(Copy,Clone,PartialEq)]
pub enum MirrorMode {
    Vertical,
    Horizontal,
    SingleScreenA,
    SingleScreenB
}


pub struct Cartridge {
    rom_data : Vec<u8>,
    prg_rom : Vec<u8>,
    chr_rom : Vec<u8>,
    prg_banks : i32,
    chr_banks : i32,
    pub mapper_id : i32,
    mirror_horz : bool,
    mirror_vert : bool,
    mirror_mode : MirrorMode,
    has_battery : bool,
    prg_ram : Vec<u8>,
    chr_ram : Vec<u8>
}
impl Cartridge {
    pub fn from_bytes(rom_data : Vec<u8>) -> Self {
        if rom_data[0..4] != [b'N',b'E',b'S',b'\x1A'] {
            panic!("Rom is not in iNES format");
        }

        let prg_banks = rom_data[4] as i32;
        let chr_banks = rom_data[5] as i32;

        let flag6 = rom_data[6];
        let flag7 = rom_data[7];

        let mirror_vert = (flag6 & 0x01) != 0;
        let mirror_horz = !mirror_vert;

        let mut mirror_mode = MirrorMode::Horizontal;

        if (flag6 & 0x08) == 0 && (flag6 & 1) != 0 {
            mirror_mode = MirrorMode::Vertical;
        }

        let mapper_id = ((flag6 >> 4) | (flag7 & 0xF0)) as i32;

        let prg_size = prg_banks * 16 * 1024;
        let chr_size = chr_banks  * 8 * 1024;
        
        let mut offset = 16;

        let prg_rom = rom_data[offset..offset + prg_size as usize].to_vec();
        offset += prg_size as usize;
        let chr_rom = rom_data[offset..offset + chr_size as usize].to_vec();

        let prg_ram = vec![0u8;8 * 1024];
        let chr_ram = vec![0u8; 8 * 1024];

        Self { rom_data, prg_rom, chr_rom, prg_banks, chr_banks, mapper_id, mirror_horz, mirror_vert, mirror_mode, has_battery: false, prg_ram, chr_ram}


    }
}


pub enum Mapper {
    Mapper0(Cartridge),

}

impl Mapper {

    pub fn with_cart(cart : Cartridge) ->Self {
        match cart.mapper_id {
            0 => Self::Mapper0(cart),
            1 => todo!("Mapper1"),
            2 => todo!("Mapper2"),
            4 => todo!("Mapper4"),
            _ => unreachable!()
        }
    }

    pub fn cpu_read(&self, addr : u16) -> u8 {
        use Mapper::*;
        
        match self {
            //
            Mapper0(cart) => {
                match addr {
                    0x6000..=0x7FFF => cart.prg_ram[addr as usize - 0x6000],
                    0x8000..=0xFFFF => {
                        if cart.prg_banks == 1 {
                            cart.prg_rom[addr as usize & 0x3FFF]
                        }
                        else {
                            cart.prg_rom[addr as usize - 0x8000]
                        }
                    }
                    _ => 0
                }             
            }
            //
        }
    }

    pub fn cpu_write(&mut self, addr : u16, val : u8) {
        use Mapper::*;
        match self {
            //
            Mapper0(cart)=> {
                if (0x6000..=0x7FFF).contains(&addr) {
                    cart.prg_ram[addr as usize - 0x6000] = val;
                }
            }
            //
        }
    }

    pub fn ppu_read(&self, addr : u16)->u8{
        use Mapper::*;
        match self {
            Mapper0(cart)=> {
                if addr < 0x2000 {
                    if cart.chr_banks != 0 {
                        return cart.chr_rom[addr as usize];
                    }
                    else {
                        return cart.chr_ram[addr as usize];
                    }
                }
                0
            }
        }
    }

    pub fn ppu_write(&mut self, addr : u16, val : u8) {
        use Mapper::*;
        match self {
            Mapper0(cart) => {
                if addr < 0x2000 && cart.chr_banks == 0 {
                    cart.chr_ram[addr as usize] = val;
                }
            }
        }

    }
    pub fn get_mirror_mode(&self)->MirrorMode {
        use Mapper::*;
        match self {
            Mapper0(cart)=>cart.mirror_mode
        }
    }
}


