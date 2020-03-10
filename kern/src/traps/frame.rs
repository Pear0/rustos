use shim::io;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
    pub tpidr: u64,
    pub ttbr0: u64,
    pub ttbr1: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
}

impl TrapFrame {

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