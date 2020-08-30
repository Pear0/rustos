use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::fmt;

use common::mutex::Mutex;
use mountfs::fs;
use mountfs::fs::FileSystem;
use mountfs::mount::mfs;
use shim::{ioerr, newioerr};
use shim::ffi::OsStr;
use shim::io;
use shim::path::{Component, Path};

use crate::traits::Entry as TraitEntry;
use crate::vfat::{Dir, Entry, VFat, VFatHandle};
use mountfs::mount::mfs::{Dir as MfsDir, FsId, FileInfo, FileId};

#[derive(Clone)]
pub struct DynVFatHandle(Rc<Mutex<VFat<Self>>>, usize);

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
        DynVFatHandle(Rc::new(Mutex::new(val)), 0)
    }

    fn lock<R>(&self, f: impl FnOnce(&mut VFat<DynVFatHandle>) -> R) -> R {
        f(&mut self.0.lock())
    }

    fn get_id(&self) -> usize {
        self.1
    }
}

fn convert_entry(entry: Entry<DynVFatHandle>) -> mfs::DirEntry {
    match &entry {
        Entry::File(f) => {
            mfs::DirEntry::new(f.name.clone(), mfs::FileInfo::metadata(f), f.size as u64, false, f.get_id())
        },
        Entry::Dir(f) => {
            mfs::DirEntry::new(f.name.clone(), mfs::FileInfo::metadata(f), 0, true, f.get_id())
        },
    }
}

pub struct DynWrapper(pub DynVFatHandle);

impl mfs::FileSystem for DynWrapper {
    fn set_id(&mut self, id: usize) {
        (self.0).1 = id
    }

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
            Ok(mfs::Entry::Dir(Arc::new(pointer.into_dir().unwrap())))
        }
    }

    fn entries(&self, manager: &FileSystem, dir: Arc<dyn mfs::Dir>) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        let my_dir: &Dir<DynVFatHandle> = dir.downcast_ref().ok_or(newioerr!(InvalidInput, "[vfat] bad directory handle"))?;

        let entries = crate::traits::Dir::entries(my_dir)?;
        Ok(Box::new(entries.map(convert_entry)))

    }

    fn dir_entry(&self, _manager: &FileSystem, dir: Arc<dyn mfs::Dir>, path: &OsStr) -> io::Result<mfs::Entry> {
        let my_dir: &Dir<DynVFatHandle> = dir.downcast_ref().ok_or(newioerr!(InvalidInput, "[vfat] bad directory handle"))?;

        match my_dir.find(path) {
            Ok(d) => {
                if d.is_dir() {
                    Ok(mfs::Entry::Dir(Arc::new(d.into_dir().unwrap())))
                } else {
                    Ok(mfs::Entry::File(Box::new(d.into_file().unwrap())))
                }
            }
            Err(e) => Err(e),
        }
    }
}