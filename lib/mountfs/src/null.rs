use crate::mount::mfs;
use shim::io;
use shim::{path::Path, path::PathBuf};
// use std::path::{Path, PathBuf};
use shim::ioerr;
use crate::fs;

pub struct NullFileSystem();

impl NullFileSystem {
    pub fn new() -> Self {
        NullFileSystem()
    }
}

impl mfs::FileSystem for NullFileSystem {
    fn open(&self, manager: &fs::FileSystem, path: &Path) -> io::Result<mfs::Entry> {
        ioerr!(NotFound, "no file")
    }
}


