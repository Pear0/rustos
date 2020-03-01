#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(raw_vec_internals)]
#![feature(panic_info_message)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;


pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;

use console::kprintln;

use pi::{gpio, timer};
use core::time::Duration;
use core::ops::DerefMut;
use pi::uart::MiniUart;
use shim::io::{self, Write, Read};

use fat32::traits::{BlockDevice};

use crate::console::CONSOLE;

use allocator::Allocator;
use fs::FileSystem;

use fat32::vfat::{VFatHandle, Dir as VDir, Metadata};
use fat32::traits::FileSystem as fs32FileSystem;
use fat32::traits::{Dir, Entry, File};
use crate::fs::sd::Sd;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn init_jtag() {
   use gpio::{Function, Gpio};

   for pin in 22..=27 {
      Gpio::new(pin).into_alt(Function::Alt4);
   }
}

fn kmain() -> ! {

    init_jtag();

    // This is so that the host computer can attach serial console/screen whatever.
    timer::spin_sleep(Duration::from_millis(100));

    for atag in pi::atags::Atags::get() {
        kprintln!("{:?}", atag);
    }

    unsafe {
        kprintln!("Initing allocator");

        ALLOCATOR.initialize();

        kprintln!("Initing filesystem");

        FILESYSTEM.initialize();
    }

    // FIXME: Start the shell.



    // let mut pin = gpio::Gpio::new(16).into_output();
    // pin.set();

//    let mut large_buf: Vec<u8> = Vec::new();


//    kprintln!("Trying to read 2");
//    FILESYSTEM.0.lock().as_ref().unwrap().lock(|fs| {
//        fs.read_chain(Cluster::from(2), &mut large_buf);
//    });
//
//    kprintln!("open()");

    {
//        let lock = FILESYSTEM.0.lock();
//        let vfat = lock.as_ref().unwrap();
//
//        let cluster = vfat.lock(|fs| fs.root_cluster());
//
//        let vfat2 = vfat.clone();

//        Vec::<u8>::new().push(b'/');
        // let name = String::from("/");
//        let metadata: Metadata = Default::default();

//        VDir {
//            vfat: vfat.clone(),
//            cluster: cluster,
//            name: String::from("/"),
//            metadata: Default::default()
//        };

//        VDir::root(vfat.clone());

    }

    let mut entry = FILESYSTEM.open("/config.txt").expect("could not open");



//    kprintln!("Trying to read 308");
//    FILESYSTEM.0.lock().as_ref().unwrap().lock(|fs| {
//        fs.read_chain(Cluster::from(308), &mut large_buf);
//    });



    {
        kprintln!("Reading file:");
        let mut f = entry.into_file().unwrap();
        let mut lock = CONSOLE.lock();
        io::copy(&mut f, lock.deref_mut());
    }

//    match &mut entry {
//        fat32::vfat::Entry::File(f) => {
//            kprintln!("{:?}", f);
//
//            {
//                let mut lock = CONSOLE.lock();
//                io::copy(f, lock.deref_mut());
//            }
//
//        },
//        fat32::vfat::Entry::Dir(f) => {
//            kprintln!("{:?}", f);
//
//            let entries = f.entries();
//
//            kprintln!("got entries");
//
//            let entries = entries.expect("could not list");
//
//            kprintln!("unwrapped entries");
//
//            for entry in entries {
//                kprintln!("{:?}", entry);
//            }
//
//        },
//    }

    shell::shell("> ");
}
