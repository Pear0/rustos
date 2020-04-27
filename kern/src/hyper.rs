use crate::process::{GlobalScheduler, HyperImpl, HyperProcess};
use crate::traps::irq::Irq;
use crate::VMM;

pub static HYPER_IRQ: Irq<HyperImpl> = Irq::uninitialized();
pub static HYPER_SCHEDULER: GlobalScheduler<HyperImpl> = GlobalScheduler::uninitialized();


pub fn hyper_main() -> ! {
    info!("VMM init");
    VMM.init_only();
    info!("VMM setup");
    VMM.setup_hypervisor();

    info!("Making hvc call");
    hvc!(5);

    unsafe {
        HYPER_IRQ.initialize();

        HYPER_SCHEDULER.initialize_hyper();
    }

    info!("Add kernel process");

    {
        let p = HyperProcess::load("/kernel.bin").expect("failed to find bin");

        let id = HYPER_SCHEDULER.add(p);
        info!("kernel id: {:?}", id);
    }

    info!("Starting kernel");

    use aarch64::{VTTBR_EL2, TTBR0_EL1, TTBR1_EL1};
    use aarch64::regs::*;

    unsafe {

        unsafe { ((0x4000_000C) as *mut u32).write_volatile(0b1111) };
        aarch64::dsb();

        // let vm = VirtualizationPageTable::new();
        //
        // VTTBR_EL2.set(vm.get_baddr().as_u64());

        asm!("dsb ish");
        aarch64::isb();

        TTBR0_EL1.set(0);
        TTBR1_EL1.set(0);

        // enable CNTP for EL1/EL0 (ref: D7.5.2, D7.5.13)
        // NOTE: This doesn't actually enable the counter stream.
        CNTHCTL_EL2.set(CNTHCTL_EL2.get() | CNTHCTL_EL2::EL0VCTEN | CNTHCTL_EL2::EL0PCTEN);
        CNTVOFF_EL2.set(0);

        // enable AArch64 in EL1 (A53: 4.3.36)
        HCR_EL2.set(HCR_EL2::RW | HCR_EL2::VM | HCR_EL2::CD | HCR_EL2::IMO | HCR_EL2::RES1);
        //  | HCR_EL2::ID | HCR_EL2::CD

        // enable floating point and SVE (SIMD) (A53: 4.3.38, 4.3.34)
        CPTR_EL2.set(0);
        CPACR_EL1.set(CPACR_EL1.get() | (0b11 << 20));

        // Set SCTLR to known state (A53: 4.3.30)
        SCTLR_EL1.set(SCTLR_EL1::RES1);

        SP_EL1.set(0x60_000);
        SP_EL0.set(0x60_000);

        // we don't want an exception in EL1 to try and use SP0 stack.
        SPSR_EL1.set(SPSR_EL1::M & 0b0101);

        DAIF.set(DAIF.get() | DAIF::D | DAIF::A | DAIF::I | DAIF::F);

        // MDSCR_EL1.set(MDSCR_EL1.get() | MDSCR_EL1::SS | MDSCR_EL1::KDE);

        MDCR_EL2.set(MDCR_EL2.get() | MDCR_EL2::TDE);

        // change execution level to EL1 (ref: C5.2.19)
        // SPSR_EL2.set(
        //     (SPSR_EL2::M & 0b0101) // EL1h
        //         | SPSR_EL2::F
        //         | SPSR_EL2::I
        //         | SPSR_EL2::D
        //         | SPSR_EL2::A,
        // );
        //
        // ELR_EL2.set(kernel_entry as u64);
    }

    HYPER_SCHEDULER.start_hyper();

    info!("looping in HVC");
    loop {
    }
}

