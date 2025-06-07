use std::path::Path;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MirrorMode {
    Vertical,
    Horizontal,
    SingleScreenA,
    SingleScreenB,
}
#[derive(Clone, Debug)]

pub struct Cartridge {
    rom_data: Vec<u8>,
    pub prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_banks: i32,
    chr_banks: i32,
    pub mapper_id: i32,
    mirror_horz: bool,
    mirror_vert: bool,
    mirror_mode: MirrorMode,
    has_battery: bool,
    prg_ram: Vec<u8>,
    chr_ram: Vec<u8>,
}

use std::error::Error;

#[derive(Copy, Clone, Debug)]
struct CartridgeLoadError {
    pub reason: &'static str,
}

impl std::fmt::Display for CartridgeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl Error for CartridgeLoadError {}

impl Cartridge {
    pub fn from_file<PathLike: AsRef<Path>>(file_path: PathLike) -> Result<Self, Box<dyn Error>> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(file_path)?;

        let mut rom_data = Vec::new();

        let _ = file.read_to_end(&mut rom_data)?;
        if rom_data[0..4] != [b'N', b'E', b'S', b'\x1A'] {
            return Err(Box::new(CartridgeLoadError {
                reason: "Not a valid nes rom",
            }));
        }
        let cart = Cartridge::from_bytes(rom_data);

        return Ok(cart);
    }

    pub fn from_bytes(rom_data: Vec<u8>) -> Self {
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
        let chr_size = chr_banks * 8 * 1024;

        let mut offset = 16;

        let prg_rom = rom_data[offset..offset + prg_size as usize].to_vec();
        offset += prg_size as usize;
        let chr_rom = rom_data[offset..offset + chr_size as usize].to_vec();

        let prg_ram = vec![0u8; 8 * 1024];
        let chr_ram = vec![0u8; 8 * 1024];

        Self {
            rom_data,
            prg_rom,
            chr_rom,
            prg_banks,
            chr_banks,
            mapper_id,
            mirror_horz,
            mirror_vert,
            mirror_mode,
            has_battery: false,
            prg_ram,
            chr_ram,
        }
    }
    pub fn set_mirroring(&mut self, mode: MirrorMode) {
        self.mirror_mode = mode;
        self.mirror_vert = mode == MirrorMode::Vertical;
        self.mirror_horz = mode == MirrorMode::Horizontal;
    }
}

#[derive(Clone, Debug)]
pub struct MMC1Cartridge {
    cart: Cartridge,
    shift_reg: u8,
    control: u8,
    chr_banks: (u8, u8),
    prg_bank: u8,
    shift_count: u8,
    prg_bank_offsets: (u16, u16),
    chr_bank_offsets: (u16, u16),
}
impl MMC1Cartridge {
    pub fn with_cartridge(cart: Cartridge) -> Self {
        let mut cartridge = MMC1Cartridge {
            cart,
            shift_reg: 0x10,
            control: 0x0C,
            chr_banks: (0, 0),
            prg_bank: 0,
            shift_count: 0,
            prg_bank_offsets: (0, 0),
            chr_bank_offsets: (0, 0),
        };
        cartridge.reset();
        cartridge
    }
    fn reset(&mut self) {
        self.shift_reg = 0x10;
        self.control = 0x0C;
        self.chr_banks = (0, 0);
        self.prg_bank = 0;
        self.shift_count = 0;
        self.apply_mirroring();
        self.apply_banks();
    }
    pub fn apply_mirroring(&mut self) {
        match self.control & 0x03 {
            0 => self.cart.set_mirroring(MirrorMode::SingleScreenA),
            1 => self.cart.set_mirroring(MirrorMode::SingleScreenB),
            2 => self.cart.set_mirroring(MirrorMode::Vertical),
            3 => self.cart.set_mirroring(MirrorMode::Horizontal),
            _ => unreachable!(),
        }
    }
    pub fn apply_banks(&mut self) {
        let chr_mode = (self.control >> 4) & 0x01;

        if chr_mode == 0 {
            self.chr_bank_offsets.0 = ((self.chr_banks.0 & 0x1E) as u16) << 12;
            self.chr_bank_offsets.1 = self.chr_bank_offsets.0.wrapping_add(0x1000);
        } else {
            self.chr_bank_offsets.0 = (self.chr_banks.0 as u16) << 12;
            self.chr_bank_offsets.1 = (self.chr_banks.1 as u16) << 12;
        }

        let prg_mode = (self.control >> 2) & 0x03;
        let prg_bank_count = (self.cart.prg_rom.len() >> 14) as u16;

        match prg_mode {
            0 | 1 => {
                let bank = ((self.prg_bank & 0x0E) as u16) % prg_bank_count.max(1);
                self.prg_bank_offsets.0 = bank << 12;
                self.prg_bank_offsets.1 = self.prg_bank_offsets.0.wrapping_add(0x4000);
            }
            2 => {
                self.prg_bank_offsets.0 = 0;
                self.prg_bank_offsets.1 = ((self.prg_bank as u16) % prg_bank_count.max(1)) << 12;
            }
            3 => {
                self.prg_bank_offsets.0 = ((self.prg_bank as u16) % prg_bank_count.max(1)) << 12;
                self.prg_bank_offsets.1 = (prg_bank_count - 1) << 12;
            }
            _ => {}
        }
        self.prg_bank_offsets.0 %= self.cart.prg_rom.len() as u16;
        self.prg_bank_offsets.1 %= self.cart.prg_rom.len() as u16;
    }
}

#[derive(Clone, Debug)]
pub enum Mapper {
    Mapper0(Cartridge),
    Mapper1(MMC1Cartridge),
}

impl Mapper {
    pub fn with_cart(cart: Cartridge) -> Self {
        match cart.mapper_id {
            0 => Self::Mapper0(cart),
            1 => Self::Mapper1(MMC1Cartridge::with_cartridge(cart)),
            2 => todo!("Mapper2"),
            4 => todo!("Mapper4"),
            _ => unreachable!(),
        }
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        use Mapper::*;

        match self {
            //
            Mapper0(cart) => match addr {
                0x6000..=0x7FFF => cart.prg_ram[addr as usize - 0x6000],
                0x8000..=0xFFFF => {
                    if cart.prg_banks == 1 {
                        cart.prg_rom[addr as usize & 0x3FFF]
                    } else {
                        cart.prg_rom[addr as usize - 0x8000]
                    }
                }
                _ => 0,
            },
            Mapper1(mmc1) => match addr {
                0x6000..=0x7FFF => mmc1.cart.prg_ram[(addr as usize) - 0x6000],
                0x8000..=0xBFFF => {
                    let idx = mmc1.prg_bank_offsets.0.wrapping_add(addr - 0x8000) as usize;
                    mmc1.cart.prg_rom[idx]
                }
                0xC000..=0xFFFF => {
                    let idx = mmc1.prg_bank_offsets.1.wrapping_add(addr - 0xC000) as usize;
                    mmc1.cart.prg_rom[idx]
                }
                _ => 0,
            },
        }
    }

    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        use Mapper::*;
        match self {
            //
            Mapper0(cart) => {
                if (0x6000..=0x7FFF).contains(&addr) {
                    cart.prg_ram[addr as usize - 0x6000] = val;
                }
            } //
            _ => todo!("Other Mappers"),
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        use Mapper::*;
        match self {
            Mapper0(cart) => {
                if addr < 0x2000 {
                    if cart.chr_banks != 0 {
                        return cart.chr_rom[addr as usize];
                    } else {
                        return cart.chr_ram[addr as usize];
                    }
                }
                0
            }
            _ => todo!("Other Mappers"),
        }
    }

    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        use Mapper::*;
        match self {
            Mapper0(cart) => {
                if addr < 0x2000 && cart.chr_banks == 0 {
                    cart.chr_ram[addr as usize] = val;
                }
            }
            Mapper1(mmc1) => {
                if addr < 0x2000 && mmc1.cart.chr_banks == 0 {
                    mmc1.cart.chr_ram[addr as usize] = val
                }
            }
        }
    }
    pub fn get_mirror_mode(&self) -> MirrorMode {
        use Mapper::*;
        match self {
            Mapper0(cart) => cart.mirror_mode,
            Mapper1(MMC1Cartridge { cart, .. }) => cart.mirror_mode,
        }
    }
    pub fn run_scanline_irq(&mut self) {
        use Mapper::*;
        match self {
            Mapper0(_) => {}
            _ => todo!("Mapper4"),
        }
    }
    pub fn irq_pending(&self) -> bool {
        use Mapper::*;
        match self {
            Mapper0(_) => false,
            _ => todo!("All mappers other besides Mapper0"),
        }
    }
}
