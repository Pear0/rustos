use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use hashbrown::HashMap;

use mountfs::fs::FileSystem;
use mountfs::mount::*;
use mountfs::mount::mfs::{Dir, FsId, FileId};
use shim::{io, path::Path, path::Component};
use shim::ffi::OsStr;
use crate::mutex::Mutex;
use crate::iosync::Global;

pub static PROC_FILES: Global<ProcFiles> = Global::new(|| ProcFiles::new());

pub type ProcFileHandler = Box<dyn Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static>;

struct ProcFile {
    name: Arc<String>,
    inode: usize,
    file_handler: ProcFileHandler,
}

pub struct ProcFiles {
    next_inode: usize,
    files: HashMap<String, ProcFile>,
}

impl ProcFiles {
    pub fn new() -> Self {
        let mut s = Self {
            next_inode: 5,
            files: HashMap::new(),
        };

        s.add_file(String::from("cpuinfo"), Box::new(|w| {

            writeln!(w, "hello world")?;
            writeln!(w, "foo bar")?;

            Ok(())
        }));

        s
    }

    pub fn add_file(&mut self, name: String, handler: ProcFileHandler) {
        let inode = self.next_inode;
        self.next_inode += 1;
        self.files.insert(name.clone(), ProcFile {
            name: Arc::new(name),
            inode,
            file_handler: handler,
        });
    }
}

pub struct ProcFileSystem {
    id: FsId,
}

impl ProcFileSystem {
    const ROOT_INODE: usize = 1;

    pub fn new() -> Self {
        Self {
            id: 0,
        }
    }
}

impl mfs::FileSystem for ProcFileSystem {
    fn set_id(&mut self, id: FsId) {
        self.id = id;
    }

    fn get_name(&self) -> Option<String> {
        Some(String::from("proc"))
    }

    fn open(&self, manager: &FileSystem, path: &Path) -> io::Result<mfs::Entry> {
        for comp in path.components() {
            if !matches!(comp, Component::RootDir) {
                info!("comp: {:?}", comp);
                return ioerr!(NotFound, "unexpected path component in open()");
            }
        }

        Ok(mfs::Entry::Dir(Arc::new(DummyDir(self.id))))
    }

    fn entries(&self, manager: &FileSystem, dir: Arc<dyn Dir>) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        let mut vec = Vec::new();

        PROC_FILES.critical(|files| {
            for (name, file) in files.files.iter() {
                vec.push(mfs::DirEntry::new(
                    String::clone(name), Metadata::default(), 0,
                    false, FileId(self.id, file.inode)));
            }
        });

        Ok(Box::new(vec.into_iter()))
    }

    fn dir_entry(&self, manager: &FileSystem, dir: Arc<dyn Dir>, path: &OsStr) -> io::Result<mfs::Entry> {
        let fs_id = self.id;
        let name = path.to_string_lossy().into_owned();

        PROC_FILES.critical(|files| {
            let file = match files.files.get(&name) {
                Some(f) => f,
                None => return ioerr!(NotFound, "file not found"),
            };

            let mut buffer: Vec<u8> = Vec::new();
            (file.file_handler)(&mut buffer)?;

            Ok(mfs::Entry::File(Box::new(RenderedFile {
                id: FileId(fs_id, file.inode),
                name: file.name.clone(),
                buffer,
                index: 0
            })))
        })
    }
}

struct DummyDir(FsId);

impl mfs::FileInfo for DummyDir {
    fn name(&self) -> &str {
        "/proc"
    }

    fn metadata(&self) -> Metadata {
        Metadata::default()
    }

    fn size(&self) -> u64 {
        0
    }

    fn is_directory(&self) -> bool {
        true
    }

    fn get_id(&self) -> FileId {
        FileId(self.0, 0)
    }
}

impl mfs::Dir for DummyDir {}

struct RenderedFile {
    id: FileId,
    name: Arc<String>,
    buffer: Vec<u8>,
    index: u64,
}

impl mfs::FileInfo for RenderedFile {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn metadata(&self) -> Metadata {
        Metadata::default()
    }

    fn size(&self) -> u64 {
        self.buffer.len() as u64
    }

    fn is_directory(&self) -> bool {
        false
    }

    fn get_id(&self) -> FileId {
        self.id
    }
}

impl io::Read for RenderedFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.index >= self.buffer.len() as u64 {
            return Ok(0)
        }
        let index = self.index as usize;
        let amount = core::cmp::min(self.buffer.len() - index as usize, buf.len());
        if amount > 0 {
            (&mut buf[..amount]).copy_from_slice(&self.buffer[index..index+amount]);
        }
        self.index += amount as u64;
        Ok(amount)
    }
}

impl io::Write for RenderedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        ioerr!(NotConnected, "read only file")
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for RenderedFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            io::SeekFrom::Start(index) => {
                self.index = index;
            }
            io::SeekFrom::End(offset) => {
                let index = (self.buffer.len() as i64) + offset;
                if index < 0 {
                    return ioerr!(InvalidInput, "cannot seek before start of file");
                }
                self.index = index as u64;
            }
            io::SeekFrom::Current(offset) => {
                let index = (self.index as i64) + offset;
                if index < 0 {
                    return ioerr!(InvalidInput, "cannot seek before start of file");
                }
                self.index = index as u64;
            }
        }

        Ok(self.index)
    }
}

impl mfs::File for RenderedFile {
    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn size(&self) -> u64 {
        self.buffer.len() as u64
    }
}