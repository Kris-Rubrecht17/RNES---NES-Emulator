use crate::cpu::CPU;
use serde::Deserialize;

pub struct TestBus {
    pub ram: Vec<u8>,
    pub(self) cycles: Vec<Cycle>,
}
impl TestBus {
    pub(crate) fn new() -> Self {
        TestBus {
            ram: vec![0u8; 0x10000],
            cycles: Vec::new(),
        }
    }
    pub fn read(&mut self, addr: u16) -> u8 {
        let res = self.ram[addr as usize];
        self.cycles.push(Cycle(addr, res, String::from("read")));
        res
    }
    pub fn write(&mut self, addr: u16, val: u8) {
        self.cycles.push(Cycle(addr, val, String::from("write")));

        self.ram[addr as usize] = val;
    }
    pub fn read_word(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = (self.read(addr.wrapping_add(1)) as u16) << 8;
        hi | lo
    }
    pub fn write_word(&mut self, addr: u16, val: u16) {
        let lo = (val & 0xFF) as u8;
        let hi = (val >> 8) as u8;
        self.write(addr, lo);
        self.write(addr.wrapping_add(1), hi);
    }
}

#[derive(Debug, Deserialize, Clone)]
struct Test {
    pub name: String,
    pub initial: CpuState,
    #[serde(rename = "final")]
    _final: CpuState,
    pub cycles: Vec<Cycle>,
}

#[derive(Debug, Deserialize, Clone)]
struct CpuState {
    pub pc: u16,
    pub s: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub ram: Vec<RamEntry>,
}

impl PartialEq for CpuState {
    fn eq(&self, other: &CpuState) -> bool {
        if self.pc != other.pc
            || self.s != other.s
            || self.x != other.x
            || self.p != other.p
            || self.a != other.a
            || self.y != other.y
        {
            return false;
        }

        let mut ram1 = self.ram.clone();
        let mut ram2 = other.ram.clone();
        ram1.sort_by(|item, item1| item.cmp(item1));
        ram2.sort_by(|item, item1| item.cmp(item1));
        for (x, y) in std::iter::zip(ram1.into_iter(), ram2.into_iter()) {
            if x != y {
                println!("Inequality in ram:{x:#?} {y:#?}");
                return false;
            }
        }
        return true;
    }
}

impl CpuState {
    pub fn clone_to_cpu(&self) -> CPU {
        let mut cpu = CPU {
            a: self.a,
            x: self.x,
            y: self.y,
            status: self.p,
            sp: self.s as u16,
            pc: self.pc,
            bus: TestBus::new(),
            ir_disable: false,
        };

        for RamEntry(addr, val) in self.ram.iter() {
            cpu.bus.ram[*addr as usize] = *val;
        }
        cpu
    }
    pub fn clone_from_cpu(&self, cpu: &CPU) -> Self {
        let mut state = CpuState {
            a: cpu.a,
            x: cpu.x,
            y: cpu.y,
            s: cpu.sp as u8,
            pc: cpu.pc,
            p: cpu.status,
            ram: Vec::new(),
        };

        for RamEntry(addr, _) in self.ram.iter() {
            state.ram.push(RamEntry(*addr, cpu.bus.ram[*addr as usize]))
        }

        state
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, PartialOrd, Ord)]
struct RamEntry(u16, u8);

#[derive(Debug, Deserialize, PartialEq, Clone)]
struct Cycle(pub u16, pub u8, pub String);

type TestRes = Result<(), Box<dyn std::error::Error>>;

fn load_test_file(file_no: u8) -> Result<Vec<Test>, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Read;

    let file_path = format!("tests/nes6502/v1/{:02x}.json", file_no);
    println!("{}", file_path);
    let mut file = File::open(&file_path)?;

    let mut file_contents = String::new();
    let _ = file.read_to_string(&mut file_contents)?;

    let tests: Vec<Test> = serde_json::from_str(&file_contents)?;

    Ok(tests)
}

fn run_test(test: Test) {
    let start_state = test.initial.clone();
    let end_state = test._final.clone();

    let mut cpu = start_state.clone_to_cpu();

    cpu.execute_instruction();

    assert_eq!(
        end_state,
        end_state.clone_from_cpu(&cpu),
        "Failed Test: {} Expected:\n\t{:?}\nGot:\n\t{:?}",
        &test.name,
        end_state,
        end_state.clone_from_cpu(&cpu)
    );

    assert_eq!(
        cpu.bus.cycles, test.cycles,
        "Failed Test: {} Expected:\n\t{:?}\nGot:\n\t{:?}",
        &test.name, test.cycles, cpu.bus.cycles
    );
}

fn run_test_file(test_no: u8) -> TestRes {
    //use threadpool::ThreadPool;
    let tests = load_test_file(test_no)?;
    //let pool = ThreadPool::new(8);

    for test in tests {
        /*pool.execute(move ||{
            run_test(test)
        })*/
        run_test(test)
    }
    //pool.join();

    Ok(())
}

mod file_tests {
    use super::*;

    mod misc {
        use super::*;

        #[test] //brk
        fn file_00() -> TestRes {
            run_test_file(0)
        }
        #[test] //nop
        fn file_ea() -> TestRes {
            run_test_file(0xEA)
        }
    }
    mod or_ops {
        use super::*;
        #[test]
        fn file_01() -> TestRes {
            run_test_file(1)
        }

        #[test]
        fn file_09() -> TestRes {
            run_test_file(0x09)
        }
        #[test]
        fn file_05() -> TestRes {
            run_test_file(0x05)
        }
        #[test]
        fn file_15() -> TestRes {
            run_test_file(0x15)
        }
        #[test]
        fn file_0d() -> TestRes {
            run_test_file(0x0D)
        }
        #[test]
        fn file_1d() -> TestRes {
            run_test_file(0x1D)
        }
        #[test]
        fn file_19() -> TestRes {
            run_test_file(0x19)
        }
        #[test]
        fn file_11() -> TestRes {
            run_test_file(0x11)
        }
    }

    mod adc_ops {
        use super::*;
        #[test]
        fn file_69() -> TestRes {
            run_test_file(0x69)
        }
        #[test]
        fn file_65() -> TestRes {
            run_test_file(0x65)
        }
        #[test]
        fn file_75() -> TestRes {
            run_test_file(0x75)
        }
        #[test]
        fn file_6d() -> TestRes {
            run_test_file(0x6D)
        }
        #[test]
        fn file_7d() -> TestRes {
            run_test_file(0x7D)
        }
        #[test]
        fn file_79() -> TestRes {
            run_test_file(0x79)
        }
        #[test]
        fn file_61() -> TestRes {
            run_test_file(0x61)
        }
        #[test]
        fn file_71() -> TestRes {
            run_test_file(0x71)
        }
    }

    mod and_ops {
        use super::*;
        #[test]
        fn file_29() -> TestRes {
            run_test_file(0x29)
        }
        #[test]
        fn file_25() -> TestRes {
            run_test_file(0x25)
        }
        #[test]
        fn file_35() -> TestRes {
            run_test_file(0x35)
        }
        #[test]
        fn file_2d() -> TestRes {
            run_test_file(0x2D)
        }
        #[test]
        fn file_3d() -> TestRes {
            run_test_file(0x3D)
        }
        #[test]
        fn file_39() -> TestRes {
            run_test_file(0x39)
        }
        #[test]
        fn file_21() -> TestRes {
            run_test_file(0x21)
        }
        #[test]
        fn file_31() -> TestRes {
            run_test_file(0x31)
        }
    }

    mod asl_ops {
        use super::*;

        #[test]
        fn file_0a() -> TestRes {
            run_test_file(0x0A)
        }
        #[test]
        fn file_06() -> TestRes {
            run_test_file(0x06)
        }
        #[test]
        fn file_16() -> TestRes {
            run_test_file(0x16)
        }
        #[test]
        fn file_0e() -> TestRes {
            run_test_file(0x0E)
        }
        #[test]
        fn file_1e() -> TestRes {
            run_test_file(0x1E)
        }
    }

    mod conditional_branches {
        use super::*;

        #[test]
        fn file_90() -> TestRes {
            run_test_file(0x90)
        }
        #[test]
        fn file_b0() -> TestRes {
            run_test_file(0xB0)
        }
        #[test]
        fn file_f0() -> TestRes {
            run_test_file(0xF0)
        }
        #[test]
        fn file_30() -> TestRes {
            run_test_file(0x30)
        }
        #[test]
        fn file_d0() -> TestRes {
            run_test_file(0xD0)
        }
        #[test]
        fn file_10() -> TestRes {
            run_test_file(0x10)
        }
        #[test]
        fn file_70() -> TestRes {
            run_test_file(0x70)
        }
        #[test]
        fn file_50() -> TestRes {
            run_test_file(0x50)
        }
    }

    mod bit_ops {
        use super::*;
        #[test]
        fn file_2c() -> TestRes {
            run_test_file(0x2C)
        }
        #[test]
        fn file_24() -> TestRes {
            run_test_file(0x24)
        }
    }
    mod flags {
        use super::*;
        #[test]
        fn file_18() -> TestRes {
            run_test_file(0x18)
        }
        #[test]
        fn file_d8() -> TestRes {
            run_test_file(0xD8)
        }
        #[test]
        fn file_58() -> TestRes {
            run_test_file(0x58)
        }
        #[test]
        fn file_b8() -> TestRes {
            run_test_file(0xB8)
        }
        #[test]
        fn file_38() -> TestRes {
            run_test_file(0x38)
        }
        #[test]
        fn file_f8() -> TestRes {
            run_test_file(0xF8)
        }
        #[test]
        fn file_78() -> TestRes {
            run_test_file(0x78)
        }
    }
    mod cmp_instructions {
        use super::*;

        #[test]
        fn file_c9() -> TestRes {
            run_test_file(0xC9)
        }
        #[test]
        fn file_c5() -> TestRes {
            run_test_file(0xC5)
        }
        #[test]
        fn file_d5() -> TestRes {
            run_test_file(0xD5)
        }
        #[test]
        fn file_cd() -> TestRes {
            run_test_file(0xCD)
        }
        #[test]
        fn file_dd() -> TestRes {
            run_test_file(0xDD)
        }
        #[test]
        fn file_d9() -> TestRes {
            run_test_file(0xD9)
        }
        #[test]
        fn file_c1() -> TestRes {
            run_test_file(0xC1)
        }
        #[test]
        fn file_d1() -> TestRes {
            run_test_file(0xD1)
        }
        #[test]
        fn file_e0() -> TestRes {
            run_test_file(0xE0)
        }
        #[test]
        fn file_e4() -> TestRes {
            run_test_file(0xE4)
        }
        #[test]
        fn file_ec() -> TestRes {
            run_test_file(0xEC)
        }
        #[test]
        fn file_c0() -> TestRes {
            run_test_file(0xC0)
        }
        #[test]
        fn file_c4() -> TestRes {
            run_test_file(0xC4)
        }
        #[test]
        fn file_cc() -> TestRes {
            run_test_file(0xCC)
        }
    }

    mod dec {
        use super::*;
        #[test]
        fn file_c6() -> TestRes {
            run_test_file(0xC6)
        }
        #[test]
        fn file_d6() -> TestRes {
            run_test_file(0xD6)
        }
        #[test]
        fn file_ce() -> TestRes {
            run_test_file(0xCE)
        }
        #[test]
        fn file_de() -> TestRes {
            run_test_file(0xDE)
        }
        #[test]
        fn file_ca() -> TestRes {
            run_test_file(0xCA)
        }
        #[test]
        fn file_88() -> TestRes {
            run_test_file(0x88)
        }
    }

    mod inc {
        use super::*;

        #[test]
        fn file_e6() -> TestRes {
            run_test_file(0xE6)
        }
        #[test]
        fn file_f6() -> TestRes {
            run_test_file(0xF6)
        }
        #[test]
        fn file_ee() -> TestRes {
            run_test_file(0xEE)
        }
        #[test]
        fn file_fe() -> TestRes {
            run_test_file(0xFE)
        }
        #[test]
        fn file_e8() -> TestRes {
            run_test_file(0xE8)
        }
        #[test]
        fn file_c8() -> TestRes {
            run_test_file(0xC8)
        }
    }
    mod xor_ops {
        use super::*;

        #[test]
        fn file_49() -> TestRes {
            run_test_file(0x49)
        }
        #[test]
        fn file_45() -> TestRes {
            run_test_file(0x45)
        }
        #[test]
        fn file_55() -> TestRes {
            run_test_file(0x55)
        }
        #[test]
        fn file_4d() -> TestRes {
            run_test_file(0x4D)
        }
        #[test]
        fn file_5d() -> TestRes {
            run_test_file(0x5D)
        }
        #[test]
        fn file_59() -> TestRes {
            run_test_file(0x59)
        }
        #[test]
        fn file_41() -> TestRes {
            run_test_file(0x41)
        }
        #[test]
        fn file_51() -> TestRes {
            run_test_file(0x51)
        }
    }

    mod jmp_ret {
        use super::*;

        #[test]
        fn file_4c() -> TestRes {
            run_test_file(0x4C)
        }
        #[test]
        fn file_6c() -> TestRes {
            run_test_file(0x6C)
        }
        #[test]
        fn file_20() -> TestRes {
            run_test_file(0x20)
        }
        #[test]
        fn file_40() -> TestRes {
            run_test_file(0x40)
        }
        #[test]
        fn file_60() -> TestRes {
            run_test_file(0x60)
        }
    }

    mod ld_reg {
        use super::*;
        #[test]
        fn file_a9() -> TestRes {
            run_test_file(0xA9)
        }
        #[test]
        fn file_a5() -> TestRes {
            run_test_file(0xA5)
        }
        #[test]
        fn file_b5() -> TestRes {
            run_test_file(0xB5)
        }
        #[test]
        fn file_ad() -> TestRes {
            run_test_file(0xAD)
        }
        #[test]
        fn file_bd() -> TestRes {
            run_test_file(0xBD)
        }
        #[test]
        fn file_b9() -> TestRes {
            run_test_file(0xB9)
        }
        #[test]
        fn file_a1() -> TestRes {
            run_test_file(0xA1)
        }
        #[test]
        fn file_b1() -> TestRes {
            run_test_file(0xB1)
        }
        #[test]
        fn file_a2() -> TestRes {
            run_test_file(0xA2)
        }
        #[test]
        fn file_a6() -> TestRes {
            run_test_file(0xA6)
        }
        #[test]
        fn file_b6() -> TestRes {
            run_test_file(0xB6)
        }
        #[test]
        fn file_ae() -> TestRes {
            run_test_file(0xAE)
        }
        #[test]
        fn file_be() -> TestRes {
            run_test_file(0xBE)
        }
        #[test]
        fn file_a0() -> TestRes {
            run_test_file(0xA0)
        }
        #[test]
        fn file_a4() -> TestRes {
            run_test_file(0xA4)
        }
        #[test]
        fn file_b4() -> TestRes {
            run_test_file(0xB4)
        }
        #[test]
        fn file_ac() -> TestRes {
            run_test_file(0xAC)
        }
        #[test]
        fn file_bc() -> TestRes {
            run_test_file(0xBC)
        }
    }

    mod lsr {
        use super::*;

        #[test]
        fn file_4a() -> TestRes {
            run_test_file(0x4A)
        }
        #[test]
        fn file_46() -> TestRes {
            run_test_file(0x46)
        }
        #[test]
        fn file_56() -> TestRes {
            run_test_file(0x56)
        }
        #[test]
        fn file_4e() -> TestRes {
            run_test_file(0x4E)
        }
        #[test]
        fn file_5e() -> TestRes {
            run_test_file(0x5E)
        }
    }

    mod push_pop {
        use super::*;
        #[test]
        fn file_48() -> TestRes {
            run_test_file(0x48)
        }

        #[test]
        fn file_08() -> TestRes {
            run_test_file(0x08)
        }
        #[test]
        fn file_28() -> TestRes {
            run_test_file(0x28)
        }
        #[test]
        fn file_68() -> TestRes {
            run_test_file(0x68)
        }
    }

    mod rotate {
        use super::*;

        #[test]
        fn file_2a() -> TestRes {
            run_test_file(0x2A)
        }
        #[test]
        fn file_26() -> TestRes {
            run_test_file(0x26)
        }
        #[test]
        fn file_36() -> TestRes {
            run_test_file(0x36)
        }
        #[test]
        fn file_2e() -> TestRes {
            run_test_file(0x2E)
        }
        #[test]
        fn file_3e() -> TestRes {
            run_test_file(0x3E)
        }
        #[test]
        fn file_6a() -> TestRes {
            run_test_file(0x6A)
        }
        #[test]
        fn file_66() -> TestRes {
            run_test_file(0x66)
        }
        #[test]
        fn file_76() -> TestRes {
            run_test_file(0x76)
        }
        #[test]
        fn file_6e() -> TestRes {
            run_test_file(0x6E)
        }
        #[test]
        fn file_7e() -> TestRes {
            run_test_file(0x7E)
        }
    }
    mod subc {
        use super::*;
        #[test]
        fn file_e9() -> TestRes {
            run_test_file(0xE9)
        }
        #[test]
        fn file_e5() -> TestRes {
            run_test_file(0xE5)
        }
        #[test]
        fn file_f5() -> TestRes {
            run_test_file(0xF5)
        }
        #[test]
        fn file_ed() -> TestRes {
            run_test_file(0xED)
        }
        #[test]
        fn file_fd() -> TestRes {
            run_test_file(0xFD)
        }
        #[test]
        fn file_f9() -> TestRes {
            run_test_file(0xF9)
        }
        #[test]
        fn file_e1() -> TestRes {
            run_test_file(0xE1)
        }
        #[test]
        fn file_f1() -> TestRes {
            run_test_file(0xF1)
        }
    }
    mod store_reg {
        use super::*;

        #[test]
        fn file_85() -> TestRes {
            run_test_file(0x85)
        }
        #[test]
        fn file_95() -> TestRes {
            run_test_file(0xC5)
        }
        #[test]
        fn file_8d() -> TestRes {
            run_test_file(0x8D)
        }
        #[test]
        fn file_9d() -> TestRes {
            run_test_file(0x9D)
        }
        #[test]
        fn file_99() -> TestRes {
            run_test_file(0x99)
        }
        #[test]
        fn file_81() -> TestRes {
            run_test_file(0x81)
        }
        #[test]
        fn file_91() -> TestRes {
            run_test_file(0x91)
        }
        #[test]
        fn file_86() -> TestRes {
            run_test_file(0x86)
        }
        #[test]
        fn file_96() -> TestRes {
            run_test_file(0x96)
        }
        #[test]
        fn file_8e() -> TestRes {
            run_test_file(0x8E)
        }
        #[test]
        fn file_84() -> TestRes {
            run_test_file(0x84)
        }
        #[test]
        fn file_94() -> TestRes {
            run_test_file(0x94)
        }
        #[test]
        fn file_8c() -> TestRes {
            run_test_file(0x8C)
        }
    }
    mod transfer_reg {
        use super::*;
        #[test]
        fn file_aa() -> TestRes {
            run_test_file(0xAA)
        }
        #[test]
        fn file_a8() -> TestRes {
            run_test_file(0xA8)
        }
        #[test]
        fn file_8a() -> TestRes {
            run_test_file(0x8A)
        }
        #[test]
        fn file_98() -> TestRes {
            run_test_file(0x98)
        }
        #[test]
        fn test_ba() -> TestRes {
            run_test_file(0xBA)
        }
        #[test]
        fn test_9a() -> TestRes {
            run_test_file(0x9A)
        }
    }
    mod undocumented {
        use super::*;
        #[test]
        fn file_a7() -> TestRes {
            run_test_file(0xA7)
        }
        #[test]
        fn file_87() -> TestRes {
            run_test_file(0x87)
        }
        #[test]
        fn file_97() -> TestRes {
            run_test_file(0x97)
        }
        #[test]
        fn file_8f() -> TestRes {
            run_test_file(0x8F)
        }
        #[test]
        fn file_83() -> TestRes {
            run_test_file(0x83)
        }
    }
}


mod cartridge_tests {
    use super::TestRes;
    use crate::cartridge::{Cartridge,Mapper};

    #[test]
    fn load_test_rom()-> TestRes {
        let cart = Cartridge::from_file("test_roms/nestest.nes")?;

        let mapper = Mapper::with_cart(cart);

        println!("{}",mapper.cpu_read(0xFFFC));
        
        Ok(())
    }



}