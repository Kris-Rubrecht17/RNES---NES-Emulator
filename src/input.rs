use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

#[repr(u8)]
#[derive(Copy,Clone,Debug)]
pub enum NESButton {
    A = 1,
    B = 1 << 1,
    Select = 1 << 2,
    Start = 1 << 3,
    Up = 1 << 4,
    Down = 1 << 5,
    Left = 1 << 6,
    Right = 1 << 7,
}

impl BitOr<NESButton> for u8 {
    type Output = u8;
    fn bitor(self, rhs: NESButton) -> Self::Output {
        self | rhs as u8
    }
}
impl BitOrAssign<NESButton> for u8 {
    fn bitor_assign(&mut self, rhs: NESButton) {
        *self |= rhs as u8;
    }
}
impl BitAnd<NESButton> for u8 {
    type Output = u8;
    fn bitand(self, rhs: NESButton) -> Self::Output {
        self & rhs as u8
    }
}
impl BitAndAssign<NESButton> for u8 {
    fn bitand_assign(&mut self, rhs: NESButton) {
        *self &= rhs as u8;
    }
}
impl Not for NESButton {
    type Output = u8;
    fn not(self) -> Self::Output {
        !(self as u8)
    }
}

pub struct Input {
    pub(crate) controller_state: u8,
    pub(crate) controller_shift: u8,
}

impl Input {
    pub fn new() -> Self {
        Input {
            controller_state: 0,
            controller_shift: 0,
        }
    }
    pub fn read(&mut self) -> u8 {
        let result = self.controller_shift & 1;
        self.controller_shift >>= 1;
        result
    }
    pub fn write(&mut self, val: u8) {
        if (val & 1) != 0 {
            self.controller_shift = self.controller_state
        }
    }
}
