use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;

use shim::ffi::OsStr;
use shim::io;
use shim::ioerr;
use shim::path::Path;

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

    fn open(&self, _manager: &fs::FileSystem, _path: &Path) -> io::Result<mfs::Entry> {
        ioerr!(NotFound, "no file open()")
    }

    fn entries(&self, _manager: &FileSystem, _dir: Arc<dyn Dir>) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        ioerr!(NotFound, "no file entries()")
    }

    fn dir_entry(&self, _manager: &FileSystem, _dir: Arc<dyn Dir>, _path: &OsStr) -> io::Result<mfs::Entry> {
        ioerr!(NotFound, "no file dir_entry()")
    }
}


