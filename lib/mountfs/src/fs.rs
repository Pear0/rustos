use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use hashbrown::HashMap;

use shim::{io, ioerr, newioerr};
use shim::{path::Path, path::PathBuf};
use shim::ffi::OsStr;
use shim::path::Component;

// use std::path::{Path, PathBuf};
use crate::mount::mfs;
use crate::mount::mfs::{FileId, FsId, INode};

struct DirIterator<'a> {
    dir: Box<dyn mfs::Dir>,
    iter: Option<Box<dyn Iterator<Item=mfs::DirEntry> + 'a>>,
}

impl Iterator for DirIterator<'_> {
    type Item = mfs::DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

pub(crate) struct Mount {
    pub path: PathBuf,
    pub delegate: Box<dyn mfs::FileSystem>,
}

pub struct MountInfo {
    pub path: PathBuf,
    pub fs_name: Option<String>,
}

pub struct FileSystem {
    fs_id: usize,
    pub(crate) filesystems: HashMap<FsId, Mount>,
    pub(crate) mounts: HashMap<Option<FileId>, FsId>,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            fs_id: 1,
            filesystems: HashMap::new(),
            mounts: HashMap::new(),
        }
    }

    pub fn get_mounts(&self) -> Vec<MountInfo> {
        let mut mounts: Vec<MountInfo> = Vec::new();
        for mount in self.filesystems.values() {
            mounts.push(MountInfo {
                path: mount.path.clone(),
                fs_name: mount.delegate.get_name(),
            })
        }
        mounts
    }

    pub fn mount(&mut self, path: Option<&dyn AsRef<Path>>, mut delegate: Box<dyn mfs::FileSystem>) -> io::Result<()> {
        let fs_id = (self.fs_id) as FsId;
        self.fs_id += 1;

        delegate.set_id(fs_id);

        self.filesystems.insert(fs_id, Mount {
            path: path.as_ref().map(|p| p.as_ref().to_path_buf()).unwrap_or(PathBuf::from("/")),
            delegate,
        });

        let mount_id = match path {
            Some(path) => Some(self.open(path.as_ref())?.get_id()),
            None => None,
        };

        self.mounts.insert(mount_id, fs_id);

        // TODO if there is a mounting error, remove filesystem from self.filesystems

        Ok(())
    }

    pub fn entries(&self, dir: Arc<dyn mfs::Dir>) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        let file_id = dir.get_id();

        if let Some(mounted_id) = self.mounts.get(&Some(file_id)) {
            let fs = self.filesystems.get(mounted_id).unwrap();
            let root = fs.delegate.open(self, &Path::new("/"))?.into_dir().expect("expected root to be a dir");
            fs.delegate.entries(self, root)
        } else {
            let fs = self.filesystems.get(&file_id.0).unwrap();
            fs.delegate.entries(self, dir)
        }
    }

    pub fn dir_entry(&self, dir: Arc<dyn mfs::Dir>, path: &OsStr) -> io::Result<mfs::Entry> {
        let file_id = dir.get_id();

        if let Some(mounted_id) = self.mounts.get(&Some(file_id)) {
            let fs = self.filesystems.get(mounted_id).unwrap();
            let root = fs.delegate.open(self, &Path::new("/"))?.into_dir().expect("expected root to be a dir");
            fs.delegate.dir_entry(self, root, path)
        } else {
            let fs = self.filesystems.get(&file_id.0).unwrap();
            fs.delegate.dir_entry(self, dir, path)
        }
    }

    fn root_dir(&self) -> io::Result<Arc<dyn mfs::Dir>> {
        let fs_id = self.mounts.get(&None).expect("no root filesystem");
        let root = self.filesystems.get(fs_id).expect("cannot find root fs");
        let dir = root.delegate.open(self, Path::new("/"))?.into_dir().expect("root is not a directory");

        Ok(dir)
    }

    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> io::Result<mfs::Entry> {
        let mut entry = mfs::Entry::Dir(self.root_dir()?);

        for component in path.as_ref().components() {
            if let Component::Normal(comp) = component {
                if let mfs::Entry::Dir(dir) = entry {
                    entry = self.dir_entry(dir, comp)?;
                } else {
                    return ioerr!(InvalidInput, "found file in directory traversal");
                }
            }
        }

        Ok(entry)
    }
}







