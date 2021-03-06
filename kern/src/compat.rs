#![allow(non_snake_case)]

use alloc::alloc::{GlobalAlloc, Layout};
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;

use pi::interrupt::{Controller, Interrupt};
use pi::mbox::MBox;
use pi::timer;

use crate::{ALLOCATOR, BootVariant, VMM};
use crate::allocator::tags::{MemTag, TaggingAlloc};
use crate::hyper::HYPER_IRQ;
use crate::kernel::KERNEL_IRQ;
use crate::mbox::with_mbox;
use crate::param::PAGE_MASK;
use crate::mini_allocators::NOCACHE_ALLOC;
use mini_alloc::Alloc;

/// Function implementations for linked C libraries

fn wrap_str_bytes(ptr: *const u8) -> &'static [u8] {
    unsafe {
        let mut len = 0;
        while *ptr.offset(len as isize) != b'\0' {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

fn wrap_str(ptr: *const u8) -> &'static str {
    unsafe {
        let mut len = 0;
        while *ptr.offset(len as isize) != b'\0' {
            len += 1;
        }
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len))
    }
}


// void uspi_assertion_failed (const char *pExpr, const char *pFile, unsigned nLine);
#[no_mangle]
extern "C" fn uspi_assertion_failed(expr: *const u8, file: *const u8, line: u32) {
    panic!("{}: {}:{}", wrap_str(expr), wrap_str(file), line);
}

// void MsDelay (unsigned nMilliSeconds);
// void usDelay (unsigned nMicroSeconds);

#[no_mangle]
extern "C" fn MsDelay(amt: u32) {
    timer::spin_sleep(Duration::from_millis(amt as u64));
}

#[no_mangle]
extern "C" fn usDelay(amt: u32) {
    timer::spin_sleep(Duration::from_micros(amt as u64));
}

// void *malloc (unsigned nSize);		// result must be 4-byte aligned
// void free (void *pBlock);

#[no_mangle]
extern "C" fn malloc(size: u32) -> *mut u8 {
    unsafe {
        let l = Layout::from_size_align_unchecked((size + 16) as usize, 16);

        let p = NOCACHE_ALLOC.alloc(l);
        p

        // let p = ALLOCATOR.alloc_tag(l, MemTag::CLib);
        // *(p as *mut u32) = size + 16; // store size (to deallocate) in first byte
        // p.offset(16)
    }
}

#[no_mangle]
extern "C" fn free(ptr: *mut u8) {
    // unsafe {
    //     let size = *(ptr.offset(-16) as *mut u32);
    //     let l = Layout::from_size_align_unchecked(size as usize, 1 << 16);
    //     ALLOCATOR.dealloc_tag(ptr.offset(-16), l, MemTag::CLib);
    // }
}

// // Severity (change this before building if you want different values)
// #define LOG_ERROR	1
// #define LOG_WARNING	2
// #define LOG_NOTICE	3
// #define LOG_DEBUG	4
//
// void LogWrite (const char *pSource,		// short name of module
// 	       unsigned	   Severity,		// see above
// 	       const char *pMessage, ...);	// uses printf format options

fn severity_str(s: u32) -> &'static str {
    match s {
        1 => "ERROR",
        2 => "WARNING",
        3 => "NOTICE",
        4 => "DEBUG",
        _ => "OTHER",
    }
}

#[no_mangle]
extern "C" fn LogWrite(source: *const u8, severity: u32, message: *const u8, arg1: u64) {
    let message = wrap_str_bytes(message);

    kprint!("[usb:{}] {}: ", wrap_str(source), severity_str(severity));

    let args = [arg1];
    let mut arg_pointer = 0;

    let mut next_arg = || {
        let arg = args.get(arg_pointer).map(|x| *x);
        arg_pointer += 1;
        arg
    };

    let mut i = 0;
    while i < message.len() {
        if message[i] == b'%' && i < message.len() - 1 {
            match message[i + 1] {
                b'%' => {
                    kprint!("%");
                    i += 2;
                    continue;
                }
                b's' => {
                    if let Some(arg) = next_arg() {
                        let str = wrap_str(arg as *const u8);
                        kprint!("{}", str);

                        i += 2;
                        continue;
                    }
                }
                b'u' => {
                    if let Some(arg) = next_arg() {
                        kprint!("{}", arg);

                        i += 2;
                        continue;
                    }
                }
                _ => {}
            }
        }

        kprint!("{}", message[i] as char);
        i += 1;
    }

    kprintln!("");
}

#[no_mangle]
extern "C" fn StartKernelTimer() {
    kprintln!("StartKernelTimer()");
    unimplemented!();
}

#[no_mangle]
extern "C" fn SetPowerStateOn() {
    debug!("Enabling USB subsystem");
    with_mbox(|mbox| mbox.set_power_state(0x00000003, true));
    kernel_api::syscall::sleep(Duration::from_millis(50));
    // pi::timer::spin_sleep(Duration::from_millis(5));
}

// typedef void TInterruptHandler (void *pParam);
//
// // USPi uses USB IRQ 9
// void ConnectInterrupt (unsigned nIRQ, TInterruptHandler *pHandler, void *pParam);

type TInterruptHandler = extern "C" fn(*mut u8);

#[no_mangle]
extern "C" fn ConnectInterrupt(_irq: u32, func: TInterruptHandler, data: *mut u8) {
    let data = data as usize;
    kprintln!("[net] Connecting USB interrupt");

    // Controller::new().enable(Interrupt::Timer3);
    Controller::new().enable(Interrupt::Usb);

    if BootVariant::kernel() {
        KERNEL_IRQ.register(Interrupt::Usb, Box::new(move |_| {
            aarch64::dmb();
            func(data as *mut u8);
            aarch64::dsb();
        }));
    } else {
        HYPER_IRQ.register(Interrupt::Usb, Box::new(move |_| {
            aarch64::dmb();
            // debug!("usb interrupt");
            func(data as *mut u8);
            aarch64::dsb();
        }));
    }

}

#[no_mangle]
extern "C" fn DebugHexdump() {
    unimplemented!();
}

#[no_mangle]
extern "C" fn GetMACAddress(ptr: &mut [u8; 6]) {
    match with_mbox(|mbox| mbox.mac_address()) {
        Some(raw) => {
            let raw: [u8; 8] = unsafe { core::mem::transmute(raw) };
            for i in 0..6 {
                ptr[i] = raw[i];
            }
        }
        None => *ptr = [0; 6],
    }
}

#[no_mangle]
extern "C" fn CancelKernelTimer() {
    unimplemented!();
}


