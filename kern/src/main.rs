#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(coerce_unsized)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![feature(panic_info_message)]
#![feature(c_variadic)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#![allow(unused_imports)]

extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate modular_bitfield;

#[macro_use]
extern crate serde;
extern crate serde_cbor;

#[macro_use]
extern crate shim;

extern crate pigrate_core as pigrate;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;

use aarch64::{CNTP_CTL_EL0, SP};
use allocator::Allocator;
use fs::FileSystem;
use net::ipv4;
use pi::{gpio, timer};
use pi::interrupt::CoreInterrupt;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;
use shim::{io, ioerr};

use crate::mutex::Mutex;
use crate::net::GlobalNetHandler;
use crate::process::{Process, Stack, Id};
use crate::traps::syndrome::Syndrome;
use crate::process::fd::FileDescriptor;
use crate::fs::handle::{SourceWrapper, SinkWrapper};
use crate::iosync::{SyncWrite, SyncRead, ReadWrapper, WriteWrapper};
use pigrate::Error;
use crate::mbox::with_mbox;
use crate::param::PAGE_SIZE;

#[macro_use]
pub mod console;
#[macro_use]
pub mod mutex;

#[cfg(not(test))]
mod init;

pub mod allocator;
pub mod cls;
mod compat;
pub mod debug;
pub mod fs;
pub mod hw;
pub mod iosync;
pub mod kernel_call;
mod logger;
pub mod mbox;
pub mod net;
pub mod pigrate_server;
pub mod shell;
pub mod smp;
pub mod sync;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();
pub static NET: GlobalNetHandler = GlobalNetHandler::uninitialized();

fn init_jtag() {
    use gpio::{Function, Gpio};

    for pin in 22..=27 {
        Gpio::new(pin).into_alt(Function::Alt4);
    }
}

fn network_thread() -> ! {

    // let serial = crate::mbox::with_mbox(|mbox| mbox.serial_number()).expect("could not get serial number");
    //
    // if serial == 0 {
    //     kprintln!("[net] skipping network thread init, qemu detected");
    //     kernel_api::syscall::exit();
    // }

    pi::timer::spin_sleep(Duration::from_millis(100));
    crate::mbox::with_mbox(|mbox| mbox.set_power_state(0x00000003, true));
    pi::timer::spin_sleep(Duration::from_millis(5));

    unsafe {
        NET.initialize();
    }

    NET.critical(|net| {

        let my_ip = ipv4::Address::from(&[10, 45, 52, 130]);

        net.tcp.add_listening_port((my_ip, 300), Box::new(|sink, source| {

            let mut proc = Process::kernel_process(String::from("net/echo"), |ctx| {

                let (source, sink) = ctx.get_stdio_or_panic();

                loop {
                    let mut buf = [0u8; 1024];

                    match source.read(&mut buf) {
                        Ok(0) => kernel_call::syscall::wait_waitable(source.clone()),
                        Ok(n) => {
                            let mut left = &mut buf[..];

                            while left.len() > 0 {

                                match sink.write(left) {
                                    Ok(0) => kernel_call::syscall::wait_waitable(sink.clone()),
                                    Ok(n) => {
                                        left = &mut left[n..];
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }

            }).or(ioerr!(Other, "foo"))?;

            proc.set_stdio(Arc::new(source), Arc::new(sink));

            SCHEDULER.add(proc);

            Ok(())
        }));


        net.tcp.add_listening_port((my_ip, 100), Box::new(|sink, source| {

            let mut proc = Process::kernel_process_old(String::from("net thread2"), my_net_thread2)
                .or(ioerr!(Other, "foo"))?;

            proc.file_descriptors.push(FileDescriptor::read(Arc::new(source)));
            proc.file_descriptors.push(FileDescriptor::write(Arc::new(sink)));

            SCHEDULER.add(proc);

            Ok(())
        }));

    });

    loop {
        if !NET.critical(|n| n.dispatch()) {
            kernel_api::syscall::sleep(Duration::from_micros(1000)).ok();
        }
    }
}

fn my_net_thread2() -> ! {
    let pid: Id = kernel_api::syscall::getpid();
    let (source, sink) = SCHEDULER.crit_process(pid, |f| {
        let f = f.unwrap();
        (f.file_descriptors[0].read.as_ref().unwrap().clone(), f.file_descriptors[1].write.as_ref().unwrap().clone())
    });

    shell::Shell::new("% ", SourceWrapper::new(source), SinkWrapper::new(sink)).shell_loop();

    kernel_api::syscall::exit();
}

static CORE_REGISTER: Mutex<Option<Vec<u64>>> = mutex_new!(None);

#[no_mangle]
fn core_bootstrap() -> ! {
    unsafe { smp::core_bootstrap(); }
}

#[inline(never)]
fn core_bootstrap_2() -> ! {
    kprintln!("Hello!");

    loop {}
}

fn my_thread() -> ! {
    kprintln!("initializing other threads");
    // CORE_REGISTER.lock().replace(Vec::new());

    kprintln!("all threads initialized");


    shell::shell("$ ");

    kernel_api::syscall::exit();
}

fn led_blink() -> ! {
    let mut g = gpio::Gpio::new(29).into_output();
    loop {
        g.set();
        kernel_api::syscall::sleep(Duration::from_millis(250)).ok();
        // timer::spin_sleep(Duration::from_millis(250));
        g.clear();
        kernel_api::syscall::sleep(Duration::from_millis(250)).ok();
        // timer::spin_sleep(Duration::from_millis(250));
    }
}


fn kmain() -> ! {
    init_jtag();

    // This is so that the host computer can attach serial console/screen whatever.
    timer::spin_sleep(Duration::from_millis(500));

    kprintln!("early boot");
    logger::register_global_logger();

    // for atag in pi::atags::Atags::get() {
    //     kprintln!("{:?}", atag);
    // }

    info!("hello");

    unsafe {
        debug!("init allocator");
        ALLOCATOR.initialize();
        debug!("init filesystem");
        FILESYSTEM.initialize();
    }

    debug!("init irq");
    IRQ.initialize();

    // initialize local timers for all cores
    unsafe { (0x4000_0008 as *mut u32).write_volatile(0x8000_0000) };
    unsafe { (0x4000_0040 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_0044 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_0048 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_004C as *mut u32).write_volatile(0b1010) };


    debug!("initing smp");

    if true {
        let cores = 4;
        unsafe { smp::initialize(cores); }
        smp::wait_for_cores(cores);
    }

    // aarch64::dsb();
    // aarch64::isb();
    // aarch64::dmb();

    // unsafe {
    //     asm!("dsb     ishst
    // tlbi    vmalle1
    // dsb     ish
    // isb":::"memory");
    // }

    // kprintln!("Cores: {}", smp::count_cores());

    // smp::run_on_secondary_cores(|| {
    //
    //     // let el = unsafe { aarch64::current_el() };
    //     // kprintln!("Current EL: {}", el);
    //
    //     kprintln!("Hello!");
    // });

    // VMM.initialize();

    debug!("init VMM data structures");
    VMM.init_only();

    info!("enabling VMM on all cores!");
    smp::run_on_all_cores(|| {
        VMM.setup();
    });

    info!("init Scheduler");
    unsafe {
        SCHEDULER.initialize();
    };

    use aarch64::regs::*;
    smp::run_on_secondary_cores(|| {
        unsafe {
            SCHEDULER.initialize();
        };
    });

    debug!("start some processes");

    {
        let proc = Process::kernel_process_old("shell".to_owned(), my_thread).unwrap();
        SCHEDULER.add(proc);
    }

    if !hw::is_qemu() {
        let mut proc = Process::kernel_process_old("net thread".to_owned(), network_thread).unwrap();
        proc.affinity.set_only(0);
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process_old("led".to_owned(), led_blink).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process("display".to_owned(), |ctx| {

            kernel_api::syscall::sleep(Duration::from_secs(1));

            let mut width: u32 = 0;
            let mut height: u32 = 0;
            let mut pitch: u32 = 0;
            let mut isrgb: u32 = 0;
            let mut lfb: &mut [u32] = &mut [];

            with_mbox(|mbox| {
                use pi::common::*;

                mbox.0[0] = 35*4;
                mbox.0[1] = MBOX_REQUEST;

                mbox.0[2] = 0x48003;  //set phy wh
                mbox.0[3] = 8;
                mbox.0[4] = 8;
                mbox.0[5] = 1024;         //FrameBufferInfo.width
                mbox.0[6] = 768;          //FrameBufferInfo.height

                mbox.0[7] = 0x48004;  //set virt wh
                mbox.0[8] = 8;
                mbox.0[9] = 8;
                mbox.0[10] = 1024;        //FrameBufferInfo.virtual_width
                mbox.0[11] = 768;         //FrameBufferInfo.virtual_height

                mbox.0[12] = 0x48009; //set virt offset
                mbox.0[13] = 8;
                mbox.0[14] = 8;
                mbox.0[15] = 0;           //FrameBufferInfo.x_offset
                mbox.0[16] = 0;           //FrameBufferInfo.y.offset

                mbox.0[17] = 0x48005; //set depth
                mbox.0[18] = 4;
                mbox.0[19] = 4;
                mbox.0[20] = 32;          //FrameBufferInfo.depth

                mbox.0[21] = 0x48006; //set pixel order
                mbox.0[22] = 4;
                mbox.0[23] = 4;
                mbox.0[24] = 1;           //RGB, not BGR preferably

                mbox.0[25] = 0x40001; //get framebuffer, gets alignment on request
                mbox.0[26] = 8;
                mbox.0[27] = 8;
                mbox.0[28] = 4096 * 16;        //FrameBufferInfo.pointer
                mbox.0[29] = 0;           //FrameBufferInfo.size

                mbox.0[30] = 0x40008; //get pitch
                mbox.0[31] = 4;
                mbox.0[32] = 4;
                mbox.0[33] = 0;           //FrameBufferInfo.pitch

                mbox.0[34] = MBOX_TAG_LAST;

                if unsafe { mbox.call(MBOX_CH_PROP) } && mbox.0[20] == 32 && mbox.0[28] != 0 {
                    mbox.0[28] &= 0x3FFFFFFF;   //convert GPU address to ARM address
                    width=mbox.0[5];          //get actual physical width
                    height=mbox.0[6];         //get actual physical height
                    pitch=mbox.0[33];         //get number of bytes per line
                    isrgb=mbox.0[24];         //get the actual channel order
                    lfb = unsafe { core::slice::from_raw_parts_mut(mbox.0[28] as *mut u32, mbox.0[29] as usize / 4) };
                } else {
                    error!("Unable to set screen resolution to 1024x768x32");
                }

            });

            if width == 0 {
                return;
            }

            info!("Frame buffer: 0x{:x}, width={}, height={}, pitch={}, isrgb={}, len={}", lfb.as_ptr() as usize, width, height, pitch, isrgb, lfb.len());

            // lfb = &mut lfb[..PAGE_SIZE];

            for i in (0..(lfb.len()*4)).step_by(PAGE_SIZE) {
                let addr = (lfb.as_ptr() as usize) + i;
                unsafe { VMM.mark_page_non_cached(addr) };
            }

            use fat32::util::*;

            let bytes: &mut [u8] = unsafe { lfb.cast_mut() };

            loop {
                for i in 0..bytes.len() {
                    let b = unsafe { (0x80000 as *const u32).offset(i as isize).read() };

                    bytes[i] = ((b & 0xFF) ^ ((b >> 8) & 0xFF) ^ ((b >> 16) & 0xFF) ^ ((b >> 24) & 0xFF)) as u8;
                }
                // lfb.copy_from_slice(unsafe { core::slice::from_raw_parts(0x80000 as *const u32, lfb.len()) });
                kernel_api::syscall::sleep(Duration::from_millis(100));
            }

        }).unwrap();
        SCHEDULER.add(proc);
    }

    // {
    //     let mut proc = Process::load("/fib.bin").expect("failed to load");
    //     SCHEDULER.add(proc);
    // }
    //
    // smp::run_on_secondary_cores(|| {
    //     kprintln!("Baz");
    // });

    smp::run_no_return(|| {
        let core = smp::core();
        pi::timer::spin_sleep(Duration::from_millis(4 * core as u64));
        debug!("Core {} starting scheduler", core);
        SCHEDULER.start();

        error!("RIP RIP");
    });

    pi::timer::spin_sleep(Duration::from_millis(50));
    kprintln!("Core 0 starting scheduler");

    SCHEDULER.start();
}
