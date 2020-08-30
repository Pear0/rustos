use alloc::boxed::Box;

use mountfs::{MetaFileSystem, NullFileSystem};
use mountfs::fs::FileSystem;
use mountfs::mount::mfs;
use shim::io;
use shim::path::Path;
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
            fs.mount(None, Box::new(MetaFileSystem::new()));

            fs.mount(Some(&PathBuf::from("/foo")), Box::new(NullFileSystem::new()));
            fs.mount(Some(&PathBuf::from("/bar")), Box::new(NullFileSystem::new()));

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