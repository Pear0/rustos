use shim::io;
use fat32::util::SliceExt;


pub trait Frame {
    fn get_id(&self) -> u64;

    fn set_id(&mut self, val: u64);
}


#[repr(C, packed)]
#[derive(Default, Copy, Clone, Debug)]
pub struct KernelTrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
    pub tpidr: u64,
    pub ttbr0: u64,
    pub ttbr1: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
}

const_assert_size!(KernelTrapFrame, 808);

impl KernelTrapFrame {

    pub fn as_bytes(&self) -> &[u8] {
        use fat32::util::SliceExt;
        unsafe { core::slice::from_ref(self).cast() }
    }

    pub fn decode_from_bytes(&mut self, bytes: &[u8]) -> Result<(), ()> {
        use fat32::util::SliceExt;
        if core::mem::size_of::<Self>() != bytes.len() {
            return Err(());
        }
        unsafe { core::slice::from_mut(self).cast_mut::<u8>() }.copy_from_slice(bytes);
        Ok(())
    }

    pub fn get_el(&self) -> u8 {
        use aarch64::regs::SPSR_EL1;
        (SPSR_EL1::get_value(self.spsr, SPSR_EL1::M) >> 2) as u8
    }

    pub fn is_el1(&self) -> bool {
        self.get_el() == 1
    }

    pub fn dump<T: io::Write>(&self, w: &mut T, full: bool) -> io::Result<()> {
        writeln!(w, "Trap Frame:")?;

        writeln!(w, "elr: 0x{:08x}", self.elr)?;
        writeln!(w, "spsr: 0x{:08x}", self.spsr)?;
        writeln!(w, "sp: 0x{:08x}", self.sp)?;
        writeln!(w, "tpidr: 0x{:08x}", self.tpidr)?;
        writeln!(w, "ttbr0: 0x{:08x}", self.ttbr0)?;
        writeln!(w, "ttbr1: 0x{:08x}", self.ttbr1)?;

        for (i, num) in self.regs.iter().enumerate() {
            writeln!(w, "regs[{:02}]: 0x{:08x}", i, *num)?;
        }

        if full {
            for (i, num) in self.simd.iter().enumerate() {
                writeln!(w, "simd[{:02}]: 0x{:032x}", i, *num)?;
            }
        }

        Ok(())
    }
}

impl Frame for KernelTrapFrame {
    fn get_id(&self) -> u64 {
        self.tpidr
    }

    fn set_id(&mut self, val: u64) {
        self.tpidr = val;
    }
}


#[repr(C, packed)]
#[derive(Default, Copy, Clone, Debug)]
pub struct HyperTrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp0: u64,
    pub tpidr0: u64,
    pub sp1: u64,
    pub tpidr2: u64,
    pub vttbr: u64,
    pub hcr: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
}

const_assert_size!(HyperTrapFrame, 808 + 16);

impl HyperTrapFrame {

    pub fn as_bytes(&self) -> &[u8] {
        use fat32::util::SliceExt;
        unsafe { core::slice::from_ref(self).cast() }
    }

    pub fn dump<T: io::Write>(&self, w: &mut T, full: bool) -> io::Result<()> {
        writeln!(w, "Hyper Trap Frame:")?;

        writeln!(w, "elr: 0x{:08x}", self.elr)?;
        writeln!(w, "spsr: 0x{:08x}", self.spsr)?;
        writeln!(w, "sp: 0x{:08x}", self.sp1)?;
        writeln!(w, "tpidr: 0x{:08x}", self.tpidr2)?;
        writeln!(w, "ttbr0: 0x{:08x}", self.vttbr)?;
        writeln!(w, "ttbr1: 0x{:08x}", self.hcr)?;

        for (i, num) in self.regs.iter().enumerate() {
            writeln!(w, "regs[{:02}]: 0x{:08x}", i, *num)?;
        }

        if full {
            for (i, num) in self.simd.iter().enumerate() {
                writeln!(w, "simd[{:02}]: 0x{:032x}", i, *num)?;
            }
        }

        Ok(())
    }

}

impl Frame for HyperTrapFrame {
    fn get_id(&self) -> u64 {
        self.tpidr2
    }

    fn set_id(&mut self, val: u64) {
        self.tpidr2 = val;
    }
}

