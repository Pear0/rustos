use alloc::rc::Rc;
use core::fmt::{self, Debug};

use dsx::sync::mutex::LockableMutex;

pub use fat32::traits;
use fat32::vfat::{VFat, VFatHandle};
use shim::io;
use shim::path::Path;

use crate::mutex::Mutex;

pub mod handle;
pub mod proc;
pub mod sd;
pub mod service;

#[derive(Clone)]
pub struct PiVFatHandle(Rc<Mutex<VFat<Self>>>);

// These impls are *unsound*. We should use `Arc` instead of `Rc` to implement
// `Sync` and `Send` trait for `PiVFatHandle`. However, `Arc` uses atomic memory
// access, which requires MMU to be initialized on ARM architecture. Since we
// have enabled only one core of the board, these unsound impls will not cause
// any immediate harm for now. We will fix this in the future.
unsafe impl Send for PiVFatHandle {}

unsafe impl Sync for PiVFatHandle {}

impl Debug for PiVFatHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "PiVFatHandle")
    }
}

impl VFatHandle for PiVFatHandle {
    fn new(val: VFat<PiVFatHandle>) -> Self {
        PiVFatHandle(Rc::new(mutex_new!(val)))
    }

    fn lock<R>(&self, f: impl FnOnce(&mut VFat<PiVFatHandle>) -> R) -> R {
        f(&mut m_lock!(self.0))
    }
}

pub struct FileSystem(pub Mutex<Option<PiVFatHandle>>);

impl FileSystem {
    /// Returns an uninitialized `FileSystem`.
    ///
    /// The file system must be initialized by calling `initialize()` before the
    /// first memory allocation. Failure to do will result in panics.
    pub const fn uninitialized() -> Self {
        FileSystem(mutex_new!(None))
    }

    /// Initializes the file system.
    /// The caller should assure that the method is invoked only once during the
    /// kernel initialization.
    ///
    /// # Panics
    ///
    /// Panics if the underlying disk or file sytem failed to initialize.
    pub unsafe fn initialize(&self) {
        let sd = sd::Sd::new().expect("failed to init sd card");
        let vfat = VFat::<PiVFatHandle>::from(sd).expect("failed to init vfat");

        m_lock!(self.0).replace(vfat);
    }
}

impl fat32::traits::FileSystem for &FileSystem {
    type File = fat32::vfat::File<PiVFatHandle>;
    type Dir = fat32::vfat::Dir<PiVFatHandle>;
    type Entry = fat32::vfat::Entry<PiVFatHandle>;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        m_lock!(self.0).as_ref().expect("kernel::fs uninitialized").open(path)
    }
}
