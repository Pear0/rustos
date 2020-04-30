use aarch64::*;

use crate::mutex::Mutex;
use crate::param::{KERNEL_MASK_BITS, USER_MASK_BITS, PAGE_MASK, PAGE_SIZE};

pub use self::address::{PhysicalAddr, VirtualAddr};
pub use self::pagetable::*;
use crate::{smp, BootVariant};
use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicU64;

mod address;
mod pagetable;

/// Thread-safe (locking) wrapper around a kernel page table.
pub struct VMManager(Mutex<Option<KernPageTable>>);

static FOO: AtomicU64 = AtomicU64::new(0);

fn flush_tlbs() {
    unsafe {
        if BootVariant::kernel() {
            asm!("dsb     sy
                  tlbi    vmalle1
                  dsb     sy
                  isb" ::: "memory" : "volatile");
        } else {
            asm!("dsb     sy
                  tlbi    alle2
                  tlbi    vmalls12e1
                  dsb     sy
                  isb" ::: "memory" : "volatile");
        }
    }
}

impl VMManager {
    /// Returns an uninitialized `VMManager`.
    ///
    /// The virtual memory manager must be initialized by calling `initialize()` and `setup()`
    /// before the first memory allocation. Failure to do will result in panics.
    pub const fn uninitialized() -> Self {
        VMManager(mutex_new!(None))
    }

    pub fn init_only(&self) {
        let mut lock = m_lock!(self.0);
        if lock.is_none() {
            lock.replace(KernPageTable::new());
        }

        let baddr = lock.as_ref().unwrap().get_baddr().as_u64();
        FOO.store(baddr, Ordering::SeqCst);
    }

    /// Initializes the virtual memory manager.
    /// The caller should assure that the method is invoked only once during the kernel
    /// initialization.
    pub fn initialize(&self) {
        self.init_only();
        kprintln!("setup()");

        self.setup_kernel();
    }

    pub unsafe fn mark_page_non_cached(&self, addr: usize) {
        assert_eq!(addr % PAGE_SIZE, 0);

        trace!("marking 0x{:x} as non cached", addr);

        let mut lock = m_lock!(self.0);
        let kern_page_table = lock.as_mut().unwrap();
        kern_page_table.set_entry(VirtualAddr::from(addr),
                                  KernPageTable::create_l3_entry(addr, EntryAttr::Nc));

        flush_tlbs();

        // match BootVariant::get_variant() {
        //     BootVariant::Kernel => self.setup_kernel(),
        //     BootVariant::Hypervisor =>  self.setup_hypervisor(),
        //     e => panic!("unknown variant: {:?}", e),
        // }
    }

    /// Set up the virtual memory manager.
    /// The caller should assure that `initialize()` has been called before calling this function.
    /// Sets proper configuration bits to MAIR_EL1, TCR_EL1, TTBR0_EL1, and TTBR1_EL1 registers.
    ///
    /// # Panics
    ///
    /// Panics if the current system does not support 64KB memory translation granule size.
    pub fn setup_kernel(&self) {

        unsafe {
            assert_eq!(ID_AA64MMFR0_EL1.get_value(ID_AA64MMFR0_EL1::TGran64), 0);
            let ips = ID_AA64MMFR0_EL1.get_value(ID_AA64MMFR0_EL1::PARange);

            // (ref. D7.2.70: Memory Attribute Indirection Register)
            MAIR_EL1.set(
                (0xFF <<  0) |// AttrIdx=0: normal, IWBWA, OWBWA, NTR
                    (0x04 <<  8) |// AttrIdx=1: device, nGnRE (must be OSH too)
                    (0x44 << 16), // AttrIdx=2: non cacheable
            );
            // (ref. D7.2.91: Translation Control Register)
            TCR_EL1.set(
                (0b00 << 37) |// TBI=0, no tagging
                    (ips  << 32) |// IPS
                    (0b11 << 30) |// TG1=64k
                    (0b11 << 28) |// SH1=3 inner
                    (0b01 << 26) |// ORGN1=1 write back
                    (0b01 << 24) |// IRGN1=1 write back
                    (0b0  << 23) |// EPD1 enables higher half
                    ((USER_MASK_BITS as u64) << 16) | // T1SZ=34 (1GB)
                    (0b01 << 14) |// TG0=64k
                    (0b11 << 12) |// SH0=3 inner
                    (0b01 << 10) |// ORGN0=1 write back
                    (0b01 <<  8) |// IRGN0=1 write back
                    (0b0  <<  7) |// EPD0 enables lower half
                    ((KERNEL_MASK_BITS as u64) << 0), // T0SZ=32 (4GB)
            );
            isb();

            let baddr = FOO.load(Ordering::SeqCst);
            // kprintln!("Reading: {:x}", baddr);
            TTBR0_EL1.set(baddr);
            TTBR1_EL1.set(baddr);

            asm!("dsb ish");
            isb();

            debug!("about to enable mmu");
            SCTLR_EL1.set(SCTLR_EL1.get() | SCTLR_EL1::I | SCTLR_EL1::C | SCTLR_EL1::M);
            debug!("enabled mmu");

            asm!("dsb sy");
            isb();

            flush_tlbs();


        }
    }

    pub fn setup_hypervisor(&self) {

        unsafe {
            assert_eq!(ID_AA64MMFR0_EL1.get_value(ID_AA64MMFR0_EL1::TGran64), 0);

            let ips = ID_AA64MMFR0_EL1.get_value(ID_AA64MMFR0_EL1::PARange);
            assert!(ips < 8);

            // (ref. D7.2.71: Memory Attribute Indirection Register)
            MAIR_EL2.set(
                (0xFF <<  0) |// AttrIdx=0: normal, IWBWA, OWBWA, NTR
                    (0x04 <<  8) |// AttrIdx=1: device, nGnRE (must be OSH too)
                    (0x44 << 16), // AttrIdx=2: non cacheable
            );

            let mut tcr = 0;
            tcr |= TCR_EL2::RES1;
            tcr |= TCR_EL2::as_value(0, TCR_EL2::TBI);
            tcr |= TCR_EL2::as_value(ips, TCR_EL2::PS); // physical address size
            tcr |= TCR_EL2::as_value(0b01, TCR_EL2::TG0); // 64kb
            tcr |= TCR_EL2::as_value(0b11, TCR_EL2::SH0); // inner
            tcr |= TCR_EL2::as_value(0b01, TCR_EL2::ORGN0); // write back
            tcr |= TCR_EL2::as_value(0b01, TCR_EL2::IRGN0); // write back

            // The size offset of the memory region for TTBR0_EL2
            tcr |= TCR_EL2::as_value(KERNEL_MASK_BITS as u64, TCR_EL2::T0SZ);
            TCR_EL2.set(tcr);

            let mut vtcr = 0;
            vtcr |= VTCR_EL2::RES1;
            vtcr |= VTCR_EL2::as_value(0, VTCR_EL2::TBI);
            vtcr |= VTCR_EL2::as_value(ips, VTCR_EL2::PS); // physical address size
            vtcr |= VTCR_EL2::as_value(0b01, VTCR_EL2::TG0); // 64kb
            vtcr |= VTCR_EL2::as_value(0b11, VTCR_EL2::SH0); // inner
            vtcr |= VTCR_EL2::as_value(0b01, VTCR_EL2::ORGN0); // write back
            vtcr |= VTCR_EL2::as_value(0b01, VTCR_EL2::IRGN0); // write back
            vtcr |= VTCR_EL2::as_value(0b01, VTCR_EL2::SL0); // starting level = 2

            // The size offset of the memory region for TTBR0_EL2
            vtcr |= VTCR_EL2::as_value(KERNEL_MASK_BITS as u64, VTCR_EL2::T0SZ);
            VTCR_EL2.set(vtcr);

            isb();

            let baddr = FOO.load(Ordering::SeqCst);
            // kprintln!("Reading: {:x}", baddr);
            TTBR0_EL2.set(baddr);

            VTTBR_EL2.set(baddr);

            asm!("dsb ish");
            isb();

            SCTLR_EL2.set(SCTLR_EL2.get() | SCTLR_EL1::I | SCTLR_EL1::C | SCTLR_EL1::M);

            HCR_EL2.set(HCR_EL2.get() | HCR_EL2::VM);

            asm!("dsb sy");
            isb();

            flush_tlbs();


        }
        
    }
    
    /// Returns the base address of the kernel page table as `PhysicalAddr`.
    pub fn get_baddr(&self) -> PhysicalAddr {
        let lock = m_lock!(self.0);
        lock.as_ref().unwrap().get_baddr()
    }
}
