use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;

use shim::{path::Path, path::PathBuf};
use shim::ffi::OsStr;
use shim::io;
use shim::ioerr;

use crate::fs;
use crate::fs::FileSystem;
use crate::mount::mfs;
use crate::mount::mfs::{Dir, FsId};

pub struct NullFileSystem(FsId);

impl NullFileSystem {
    pub fn new() -> Self {
        NullFileSystem(0)
    }
}

impl mfs::FileSystem for NullFileSystem {
    fn set_id(&mut self, id: FsId) {
        self.0 = id;
    }

    fn get_name(&self) -> Option<String> {
        Some(String::from("null"))
    }

    fn open(&self, manager: &fs::FileSystem, path: &Path) -> io::Result<mfs::Entry> {
        ioerr!(NotFound, "no file open()")
    }

    fn entries(&self, manager: &FileSystem, dir: Arc<dyn Dir>) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        ioerr!(NotFound, "no file entries()")
    }

    fn dir_entry(&self, manager: &FileSystem, dir: Arc<dyn Dir>, path: &OsStr) -> io::Result<mfs::Entry> {
        ioerr!(NotFound, "no file dir_entry()")
    }
}


