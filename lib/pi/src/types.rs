
#[repr(C, packed)]
#[derive(Copy, Clone, Default, Debug)]
pub struct BigU16(u16);

impl BigU16 {
    pub fn new(val: u16) -> BigU16 {
        BigU16(u16::to_be(val))
    }

    pub fn get(&self) -> u16 {
        u16::from_be(self.0)
    }

    pub fn set(&mut self, val: u16) {
        self.0 = u16::to_be(val);
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Default, Debug)]
pub struct BigU32(u32);

impl BigU32 {
    pub fn new(val: u32) -> BigU32 {
        BigU32(u32::to_be(val))
    }

    pub fn get(&self) -> u32 {
        u32::from_be(self.0)
    }

    pub fn set(&mut self, val: u32) {
        self.0 = u32::to_be(val);
    }
}
