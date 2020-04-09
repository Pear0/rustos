use alloc::boxed::Box;
use alloc::string::String;
use shim::{io, path::Path};
use crate::mount::Metadata;
use crate::fs;

pub trait FileInfo {
    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str;

    /// The metadata associated with the entry.
    fn metadata(&self) -> Metadata;

    fn size(&self) -> u64;

    fn is_directory(&self) -> bool;
}

/// Trait implemented by files in the file system.
pub trait File: FileInfo + io::Read + io::Write + io::Seek {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()>;

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64;
}

/// Trait implemented by directories in a file system.
pub trait Dir: FileInfo {
    /// The type of entry stored in this directory.

    /// Returns an interator over the entries in this directory.
    fn entries<'a>(&'a self) -> io::Result<Box<dyn Iterator<Item=DirEntry> + 'a>>;
}

pub struct DirEntry {
    pub name: String,
    pub metadata: Metadata,
    pub size: u64,
    pub is_directory: bool,
}

impl DirEntry {
    pub fn new(name: String, metadata: Metadata, size: u64, is_directory: bool) -> Self {
        Self { name, metadata, size, is_directory }
    }
}

impl FileInfo for DirEntry {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn metadata(&self) -> Metadata {
        self.metadata.clone()
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn is_directory(&self) -> bool {
        self.is_directory
    }
}

pub enum Entry {
    File(Box<dyn File>),
    Dir(Box<dyn Dir>),
}

impl Entry {

    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    pub fn as_file(&self) -> Option<&dyn File> {
        if let Entry::File(f) = self {
            Some(f.as_ref())
        } else {
            None
        }
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    pub fn as_dir(&self) -> Option<&dyn Dir> {
        if let Entry::Dir(d) = self {
            Some(d.as_ref())
        } else {
            None
        }
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    pub fn into_file(self) -> Option<Box<dyn File>> {
        if let Entry::File(f) = self {
            Some(f)
        } else {
            None
        }
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    pub fn into_dir(self) -> Option<Box<dyn Dir>> {
        if let Entry::Dir(d) = self {
            Some(d)
        } else {
            None
        }
    }

    pub fn is_file(&self) -> bool {
        if let Entry::File(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_dir(&self) -> bool {
        !self.is_file()
    }

    pub fn name(&self) -> &str {
        match self {
            Entry::File(f) => f.name(),
            Entry::Dir(d) => d.name(),
        }
    }

    pub fn metadata(&self) -> Metadata {
        match self {
            Entry::File(f) => f.metadata(),
            Entry::Dir(d) => d.metadata(),
        }
    }
}

impl FileInfo for Entry {
    fn name(&self) -> &str {
        Entry::name(self)
    }

    fn metadata(&self) -> Metadata {
        Entry::metadata(self)
    }

    fn size(&self) -> u64 {
        match self {
            Entry::File(f) => File::size(f.as_ref()),
            Entry::Dir(d) => 0,
        }
    }

    fn is_directory(&self) -> bool {
        self.is_dir()
    }
}

/// Trait implemented by file systems.
pub trait FileSystem {

    /// Opens the entry at `path`. `path` must be absolute.
    ///
    /// # Errors
    ///
    /// If `path` is not absolute, an error kind of `InvalidInput` is returned.
    ///
    /// If any component but the last in `path` does not refer to an existing
    /// directory, an error kind of `InvalidInput` is returned.
    ///
    /// If there is no entry at `path`, an error kind of `NotFound` is returned.
    ///
    /// All other error values are implementation defined.
    fn open(&self, manager: &fs::FileSystem, path: &Path) -> io::Result<Entry>;

}







