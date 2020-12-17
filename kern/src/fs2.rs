use alloc::boxed::Box;

use dsx::sync::mutex::LockableMutex;

use fat32::vfat::{DynVFatHandle, DynWrapper, VFat};
use mountfs::{MetaFileSystem, NullFileSystem};
use mountfs::fs::FileSystem;
use mountfs::mount::mfs;
use shim::io;
use shim::path::Path;
use shim::path::PathBuf;

use crate::fs::proc::ProcFileSystem;
use crate::fs::sd;
use crate::hw;
use crate::hw::ArchVariant;
use crate::mutex::Mutex;

pub struct FileSystem2(pub Mutex<Option<mountfs::fs::FileSystem>>);

impl FileSystem2 {
    pub const fn uninitialized() -> Self {
        FileSystem2(Mutex::new(None))
    }

    pub unsafe fn initialize(&self) {
        let mut lock = self.0.lock();
        lock.replace({
            let mut fs = FileSystem::new();
            fs.mount(None, Box::new(MetaFileSystem::new()));

            fs.mount(Some(&PathBuf::from("/foo")), Box::new(NullFileSystem::new()));
            fs.mount(Some(&PathBuf::from("/bar")), Box::new(NullFileSystem::new()));

            fs.mount(Some(&PathBuf::from("/proc")), Box::new(ProcFileSystem::new()));

            if matches!(hw::arch_variant(), ArchVariant::Pi(_)) {
                let sd = sd::Sd::new().expect("failed to init sd card");
                let vfat = VFat::<DynVFatHandle>::from(sd).expect("failed to init vfat");
                fs.mount(Some(&PathBuf::from("/fat")), Box::new(DynWrapper(vfat)));
            }

            fs
        });
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<mfs::Entry> {
        self.critical(|fs| fs.open(path))
    }

    pub fn critical<R, F: FnOnce(&mut mountfs::fs::FileSystem) -> R>(&self, func: F) -> R {
        let mut lock = self.0.lock();
        let mut fs = lock.as_mut().expect("kernel::fs2 uninitialized");
        func(fs)
    }
}