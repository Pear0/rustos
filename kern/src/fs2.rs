use alloc::boxed::Box;

use mountfs::{MetaFileSystem, NullFileSystem};
use mountfs::fs::FileSystem;
use shim::path::PathBuf;

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
            fs.mount(PathBuf::from("/"), Box::new(MetaFileSystem::new()));

            fs.mount(PathBuf::from("/foo"), Box::new(NullFileSystem::new()));
            fs.mount(PathBuf::from("/bar"), Box::new(NullFileSystem::new()));

            fs
        });
    }
}