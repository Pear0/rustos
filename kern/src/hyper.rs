use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::time::Duration;

use aarch64::CNTHP_CTL_EL2;
use pi::usb::Usb;

use crate::{hw, smp, VMM};
use crate::BootVariant::Hypervisor;
use crate::iosync::Global;
use crate::net::physical::{Physical, PhysicalUsb};
use crate::process::{GlobalScheduler, HyperImpl, HyperProcess};
use crate::traps::irq::Irq;
use crate::virtualization::nic::VirtualSwitch;

pub static HYPER_IRQ: Irq<HyperImpl> = Irq::uninitialized();
pub static HYPER_SCHEDULER: GlobalScheduler<HyperImpl> = GlobalScheduler::uninitialized();

pub static NET_SWITCH: Global<VirtualSwitch> = Global::new(|| VirtualSwitch::new_hub());

fn net_thread() -> ! {
    if hw::is_qemu() {
        error!("detected qemu, not booting network services.");
        kernel_api::syscall::exit();
    }

    kernel_api::syscall::sleep(Duration::from_millis(1000));

    crate::mbox::with_mbox(|mbox| {
        info!("Serial: {:x?}", mbox.serial_number());
        info!("MAC: {:x?}", mbox.mac_address());
        info!("Board Revision: {:x?}", mbox.board_revision());
        info!("Temp: {:?}", mbox.core_temperature());
    });

    crate::mbox::with_mbox(|mbox| mbox.set_power_state(0x00000003, true));
    pi::timer::spin_sleep(Duration::from_millis(100));

    let usb = unsafe { Usb::new().expect("failed to init usb") };

    debug!("created usb");

    let usb = PhysicalUsb(usb);

    while !usb.is_connected() {
        debug!("waiting for link");
        kernel_api::syscall::sleep(Duration::from_millis(500));
    }

    let usb = Arc::new(usb);

    NET_SWITCH.critical(move |switch| {
        switch.debug = true;

        // VirtualSwitch holds a weak reference.
        // For now we always want to have usb ethernet connected,
        // so just leak the Arc.
        switch.register(usb.clone());
        core::mem::forget(usb);
    });

    loop {
        if !(NET_SWITCH.critical(|s| s.process(Duration::from_micros(500)))) {
            kernel_api::syscall::sleep(Duration::from_millis(5));
        }
    }

    error!("net thread exit.");
    kernel_api::syscall::exit();
}

fn test_thread() -> ! {
    use aarch64::regs::*;

    loop {
        let core = smp::core();
        info!("I am the hyper thread! {} {}, {:#b}", unsafe { CNTHP_CTL_EL2.get() & CNTHP_CTL_EL2::ISTATUS }, unsafe { DAIF.get() }, unsafe { ((0x4000_0040 + 4 * core) as *mut u32).read_volatile() });

        // pi::timer::spin_sleep(Duration::from_secs(1));
        kernel_api::syscall::sleep(Duration::from_secs(1));
    }
}

pub fn hyper_main() -> ! {
    info!("VMM init");
    VMM.init_only();
    info!("VMM setup");
    VMM.setup_hypervisor();

    info!("ID_AA64PFR0_EL1: {:#b}", unsafe { ID_AA64PFR0_EL1.get() });
    info!("ID_AA64MMFR1_EL1: {:#b}", unsafe { ID_AA64MMFR1_EL1.get() });

    info!("Making hvc call");
    hvc!(5);

    unsafe {
        HYPER_IRQ.initialize();

        HYPER_SCHEDULER.initialize_hyper();
    }

    for atag in pi::atags::Atags::get() {
        info!("{:?}", atag);
    }

    info!("Add kernel process");

    {
        // let p = HyperProcess::load("/kernel.bin").expect("failed to find bin");
        let p = HyperProcess::load_self().expect("failed to find bin");

        let id = HYPER_SCHEDULER.add(p);
        info!("kernel id: {:?}", id);
    }
    //
    // {
    //     let p = HyperProcess::hyper_process_old(String::from("hyper proc"), test_thread).expect("failed to create hyper thread");
    //     let id = HYPER_SCHEDULER.add(p);
    //     info!("kernel id: {:?}", id);
    // }

    {
        let p = HyperProcess::hyper_process_old(String::from("hyper net"), net_thread).expect("failed to create hyper thread");
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

        // don't trap CNTP for EL1/EL0 (ref: D7.5.2, D7.5.13)
        CNTHCTL_EL2.set(CNTHCTL_EL2.get() | CNTHCTL_EL2::EL1PCEN | CNTHCTL_EL2::EL1PCTEN);

    }

    HYPER_SCHEDULER.start_hyper();

    info!("looping in HVC");
    loop {}
}

