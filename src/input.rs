

pub struct Input {
    pub(crate) controller_state: u8,
    pub(crate) controller_shift: u8,
}

impl Input {
    pub fn new() -> Self {
        Input {
            controller_state:0,
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
