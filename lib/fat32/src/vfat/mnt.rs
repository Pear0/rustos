use alloc::boxed::Box;
use crate::vfat::{VFatHandle, Entry, Dir, VFat};
use mountfs::mount::mfs;
use shim::path::{Path, Component};
use shim::io;
use shim::ioerr;
use alloc::rc::Rc;
use common::mutex::Mutex;
use core::fmt;
use mountfs::fs;

#[derive(Clone)]
pub struct DynVFatHandle(Rc<Mutex<VFat<Self>>>);

// These impls are *unsound*. We should use `Arc` instead of `Rc` to implement
// `Sync` and `Send` trait for `PiVFatHandle`. However, `Arc` uses atomic memory
// access, which requires MMU to be initialized on ARM architecture. Since we
// have enabled only one core of the board, these unsound impls will not cause
// any immediate harm for now. We will fix this in the future.
unsafe impl Send for DynVFatHandle {}
unsafe impl Sync for DynVFatHandle {}

impl fmt::Debug for DynVFatHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "DynVFatHandle")
    }
}


impl VFatHandle for DynVFatHandle {
    fn new(val: VFat<DynVFatHandle>) -> Self {
        DynVFatHandle(Rc::new(Mutex::new(val)))
    }

    fn lock<R>(&self, f: impl FnOnce(&mut VFat<DynVFatHandle>) -> R) -> R {
        f(&mut self.0.lock())
    }
}

pub struct DynWrapper(pub DynVFatHandle);

impl mfs::FileSystem for DynWrapper {

    fn open(&self, manager: &fs::FileSystem, path: &Path) -> io::Result<mfs::Entry> {
        use crate::traits::Entry as TraitEntry;

        let mut pointer: Entry<DynVFatHandle> = Entry::Dir(Dir::root(self.0.clone()));

        for component in path.components() {
            match component {
                Component::RootDir => pointer = Entry::Dir(Dir::root(self.0.clone())),
                Component::Normal(s) => match pointer.as_dir() {
                    Some(d) => pointer = d.find(s)?,
                    None => return ioerr!(PermissionDenied, "found file in path traversal"),
                }
                _ => return ioerr!(InvalidInput, "unexpected path item"),
            }
        }

        if pointer.is_file() {
            Ok(mfs::Entry::File(Box::new(pointer.into_file().unwrap())))
        } else {
            Ok(mfs::Entry::Dir(Box::new(pointer.into_dir().unwrap())))
        }
    }
}