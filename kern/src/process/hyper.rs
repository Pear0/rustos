use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use kernel_api::{OsError, OsResult};

use aarch64::{SPSR_EL1, SPSR_EL2, HCR_EL2};
use pigrate_core::bundle::MemoryBundle;
use pigrate_core::bundle::ProcessBundle;
use shim::io;
use shim::path::Path;

use crate::{FILESYSTEM, VMM};
use crate::fs::handle::{Sink, Source};
use crate::kernel::KERNEL_SCHEDULER;
use crate::param::{PAGE_SIZE, USER_IMG_BASE, PAGE_MASK};
use crate::process::{Id, Process, ProcessImpl, State};
use crate::process::address_space::{KernelRegionKind, Region, HyperRegionKind};
use crate::process::fd::FileDescriptor;
use crate::sync::Completion;
use crate::traps::{Frame, KernelTrapFrame, HyperTrapFrame};

use crate::vm::{VirtualAddr, VirtualizationPageTable, GuestPageTable};
use alloc::format;
use crate::virtualization::{IrqController, StackedDevice, HwPassthroughDevice, broadcom, DataAccess, AccessSize, VirtDevice};

pub struct HyperImpl {
    pub irqs: IrqController,
    virt_device: Arc<StackedDevice>,
}

fn create_virt_device() -> StackedDevice {
    let mut dev = StackedDevice::new();

    dev.add(Box::new(HwPassthroughDevice::new(false))); // base case
    dev.add(Box::new(broadcom::Interrupts::new()));
    dev.add(Box::new(broadcom::SystemTimer::new()));

    dev
}

impl ProcessImpl for HyperImpl {
    type Frame = HyperTrapFrame;
    type RegionKind = HyperRegionKind;
    type PageTable = VirtualizationPageTable;

    fn new() -> OsResult<Self> {
        Ok(Self {
            irqs: IrqController::new(),
            virt_device: Arc::new(create_virt_device()),
        })
    }

    fn create_idle_processes(count: usize) -> Vec<Process<Self>> {
        let mut idle_tasks = Vec::new();
        idle_tasks.reserve_exact(count);
        for i in 0..count {
            let name = format!("idle_task{}", i);
            let proc = Process::<Self>::kernel_process_old(name, || {
                loop {
                    aarch64::wfe();
                    // trigger context switch immediately after WFE so we dont take a full
                    // scheduler slice.
                    kernel_api::syscall::sched_yield();
                }
            }).expect("failed to create idle task");
            idle_tasks.push(proc);
        }
        idle_tasks
    }

}

pub type HyperProcess = Process<HyperImpl>;

impl Process<HyperImpl> {
    pub fn kernel_process_old(name: String, f: fn() -> !) -> OsResult<Self> {
        use crate::VMM;

        let mut p = Self::new(name)?;

        p.context.sp = p.stack.top().as_u64();
        p.context.elr = f as u64;

        p.context.spsr |= SPSR_EL1::M & 0b1001;

        // kernel thread still gets a vmap because it's easy
        p.context.vttbr = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to the its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Self> {
        use crate::VMM;

        let mut p = Self::do_load(pn)?;

        p.context.spsr = (SPSR_EL2::M & 0b0101) // EL1h
        ;
        // todo route all interrupts to EL2 and use virtual interrupts

        p.context.spsr |= SPSR_EL2::D | SPSR_EL2::A | SPSR_EL2::I | SPSR_EL2::F;

        p.context.sp = 0;
        p.context.elr = 0x80000;

        p.context.vttbr = p.vmap.get_baddr().as_u64();
        p.context.hcr = HCR_EL2::RW | HCR_EL2::VM | HCR_EL2::CD | HCR_EL2::IMO | HCR_EL2::RES1;


        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Self> {
        use fat32::traits::*;
        use shim::io::Read;
        let mut proc = Self::new(pn.as_ref().to_str().ok_or(OsError::InvalidArgument)?.to_owned())?;

        let total_size = 8000 * PAGE_SIZE;

        // Allocate 512 megabytes
        proc.vmap.add_region(Region::new(VirtualAddr::from(0), total_size, HyperRegionKind::Normal));

        assert!(proc.vmap.table.is_valid(VirtualAddr::from(0x80000)));

        {
            use pi::atags::raw;
            use fat32::util::*;
            let mut buf = proc.vmap.get_page_mut(VirtualAddr::from(0)).expect("tried to deref bad page");
            unsafe { VMM.mark_page_non_cached(buf.as_ptr() as usize) };

            // start of atags
            let mut buf: &mut [u32] = unsafe { core::slice::from_raw_parts_mut((buf.as_mut_ptr() as usize + 0x100) as *mut u32, 100) };

            buf[0] = 5;
            buf[1] = raw::Atag::CORE;
            buf[2] = 0; // flags
            buf[3] = 4096; // pagesize
            buf[4] = 0; // root dev

            buf[5] = 4;
            buf[6] = raw::Atag::MEM;
            buf[7] = total_size as u32; // size
            buf[8] = 0; // physical start address

            buf[9] = 2;
            buf[10] = raw::Atag::NONE;


        }

        // 257 = ceil( (0x4000_00FC - 0x3f00_0000) / PAGE_SIZE )
        proc.vmap.add_region(Region::new(VirtualAddr::from(0x3f000000), 257 * PAGE_SIZE, HyperRegionKind::Emulated(proc.detail.virt_device.clone())));

        let mut file = FILESYSTEM.open(pn)?.into_file().ok_or(OsError::InvalidArgument)?;

        let mut base = VirtualAddr::from(0x80_000);
        'page_loop: loop {
            let mut buf = proc.vmap.get_page_mut(base).expect("tried to deref bad page");
            unsafe { VMM.mark_page_non_cached(buf.as_ptr() as usize) };

            while buf.len() > 0 {
                let read_amount = file.read(buf)?;
                if read_amount == 0 {
                    break 'page_loop;
                }
                buf = &mut buf[read_amount..];
            }

            base = base + VirtualAddr::from(PAGE_SIZE);
        }

        Ok(proc)
    }

    pub fn update(&mut self) {
        use crate::virtualization::VirtDevice;

        let device = self.detail.virt_device.clone();
        device.update(self);

        // clear virtual irq flag.
        self.context.hcr &= !HCR_EL2::VI;
        if self.detail.irqs.is_any_asserted() {
            // assert virtual irq flag.
            self.context.hcr |= HCR_EL2::VI;
        }
    }

    pub fn on_access_fault(&mut self, esr: u32, addr: VirtualAddr, tf: &mut HyperTrapFrame) {
        let region = self.vmap.get_region(addr).expect("on_access_fault() called on unmapped address");
        match &region.kind {
            HyperRegionKind::Normal => {
                use aarch64::regs::*;
                use crate::traps::syndrome::Syndrome;

                kprintln!("access flag: ipa={:#x?} FAR_EL1 = 0x{:x}, FAR_EL2 = 0x{:x}, HPFAR_EL2 = 0x{:x}", addr, unsafe { FAR_EL1.get() }, unsafe { FAR_EL2.get() }, unsafe { HPFAR_EL2.get() });

                kprintln!("    FAR_EL1 = 0x{:x}, FAR_EL2 = 0x{:x}, HPFAR_EL2 = 0x{:x}", unsafe { FAR_EL1.get() }, unsafe { FAR_EL2.get() }, unsafe { HPFAR_EL2.get() });
                kprintln!("    EL1: {:?} (raw=0x{:x})", Syndrome::from(unsafe { ESR_EL1.get() } as u32), unsafe { ESR_EL1.get() });
                kprintln!("    SP: {:#x}, ELR_EL1: {:#x}, SPSR: {:#x}", unsafe { SP_EL1.get() }, unsafe { ELR_EL1.get() }, tf.spsr);

                self.vmap.table.mark_accessed(VirtualAddr::from(addr.as_u64() & PAGE_MASK as u64));

            }
            HyperRegionKind::Emulated(_) => {
                use aarch64::regs::*;
                let access = DataAccess::parse_esr(esr).expect("ISV not valid");
                // kprint!("data access: {:#x?} {:#x?} -> {:?}", unsafe { ELR_EL2.get() }, addr, access);

                let device = self.detail.virt_device.clone();

                if access.write {
                    let normalized: u64;
                    if access.register_idx == 31 {
                        normalized = 0;
                    } else {
                        let val = tf.regs[access.register_idx];
                        normalized = match access.access_size {
                            AccessSize::Byte => val as u8 as u64,
                            AccessSize::HalfWord => val as u16 as u64,
                            AccessSize::Word => val as u32 as u64,
                            AccessSize::DoubleWord => val,
                        };
                    }

                    // kprintln!(", write: r{} <- {:#x}", access.register_idx, normalized);
                    if let Err(e) = device.write(self, &access, addr, normalized) {
                        error!("write addr: {:#x?}, access: {:?} -> err: {:?}", addr, &access, e);
                        loop {}
                    }
                } else {
                    let val = match device.read(self, &access, addr) {
                        Ok(val) => val,
                        Err(e) => {
                            error!("read addr: {:#x?}, access: {:?} -> err: {:?}", addr, &access, e);
                            loop {}
                        }
                    };

                    let normalized = if access.sign_extend {
                        match access.access_size {
                            AccessSize::Byte => val as u8 as i8 as i64 as u64,
                            AccessSize::HalfWord => val as u16 as i16 as i64 as u64,
                            AccessSize::Word => val as u32 as i32 as i64 as u64,
                            AccessSize::DoubleWord => val,
                        }
                    } else {
                        match access.access_size {
                            AccessSize::Byte => val as u8 as u64,
                            AccessSize::HalfWord => val as u16 as u64,
                            AccessSize::Word => val as u32 as u64,
                            AccessSize::DoubleWord => val,
                        }
                    };

                    if access.register_idx != 31 {
                        tf.regs[access.register_idx] = normalized;
                    }
                    // kprintln!(", read: r{} -> {:#x}", access.register_idx, normalized);
                }

                // We emulated the instruction so skip it.
                tf.elr += 4;

            }
        }
    }



}

