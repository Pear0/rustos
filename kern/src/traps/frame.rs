use fat32::util::SliceExt;
pub use gen::*;
use shim::io;

mod gen;

pub trait Frame {
    fn get_id(&self) -> u64;

    fn set_id(&mut self, val: u64);
}

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
        (SPSR_EL1::get_value(self.SPSR_EL1, SPSR_EL1::M) >> 2) as u8
    }

    pub fn is_el1(&self) -> bool {
        self.get_el() == 1
    }

    pub fn dump<T: io::Write>(&self, w: &mut T, full: bool) -> io::Result<()> {
        writeln!(w, "Trap Frame:")?;

        writeln!(w, "ELR_EL1: 0x{:08x}", self.ELR_EL1)?;
        writeln!(w, "SPSR_EL1: 0x{:08x}", self.SPSR_EL1)?;
        writeln!(w, "SP_EL0: 0x{:08x}", self.SP_EL0)?;
        writeln!(w, "TPIDR_EL0: 0x{:08x}", self.TPIDR_EL0)?;
        writeln!(w, "TTBR0_EL1: 0x{:08x}", self.TTBR0_EL1)?;
        writeln!(w, "TTBR1_EL1: 0x{:08x}", self.TTBR1_EL1)?;

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
        self.TPIDR_EL0
    }

    fn set_id(&mut self, val: u64) {
        self.TPIDR_EL0 = val;
    }
}

impl HyperTrapFrame {
    pub fn as_bytes(&self) -> &[u8] {
        use fat32::util::SliceExt;
        unsafe { core::slice::from_ref(self).cast() }
    }

    pub fn dump<T: io::Write>(&self, w: &mut T, full: bool) -> io::Result<()> {
        writeln!(w, "Hyper Trap Frame:")?;

        writeln!(w, "ELR_EL2: 0x{:08x}", self.ELR_EL2)?;
        writeln!(w, "SPSR_EL2: 0x{:08x}", self.SPSR_EL2)?;
        writeln!(w, "SP_EL1: 0x{:08x}", self.SP_EL1)?;
        writeln!(w, "TPIDR_EL2: 0x{:08x}", self.TPIDR_EL2)?;
        writeln!(w, "VTTBR_EL2: 0x{:08x}", self.VTTBR_EL2)?;
        writeln!(w, "HCR_EL2: 0x{:08x}", self.HCR_EL2)?;

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
        self.TPIDR_EL2
    }

    fn set_id(&mut self, val: u64) {
        self.TPIDR_EL2 = val;
    }
}

