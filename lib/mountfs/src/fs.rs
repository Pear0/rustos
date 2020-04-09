use alloc::boxed::Box;
use alloc::vec::Vec;
use shim::{io, newioerr};
use shim::{path::Path, path::PathBuf};
// use std::path::{Path, PathBuf};
use crate::mount::mfs;



pub(crate) struct Mount {
    pub path: PathBuf,
    pub delegate: Box<dyn mfs::FileSystem>,
}

pub struct FileSystem {
    pub(crate) mounts: Vec<Mount>,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            mounts: Vec::new(),
        }
    }

    pub fn mount(&mut self, path: PathBuf, delegate: Box<dyn mfs::FileSystem>) {
        self.mounts.push(Mount{ path, delegate })
    }

    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> io::Result<mfs::Entry> {

        let mut most_depth = 0;
        let mut index = usize::max_value();

        for (i, mount) in self.mounts.iter().enumerate() {
            if path.as_ref().starts_with(&mount.path) && mount.path.iter().count() >= most_depth {
                most_depth = mount.path.iter().count();
                index = i;
            }
        }

        let mount = self.mounts.get(index).ok_or(newioerr!(NotFound, "path does not match any mount"))?;

        let leaf = path.as_ref().strip_prefix(&mount.path).unwrap();

        mount.delegate.open(self, leaf)
    }

}







