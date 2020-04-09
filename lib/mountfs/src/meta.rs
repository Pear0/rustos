use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::mount::{mfs, Metadata};
use shim::io;
use shim::{path::Path, path::PathBuf};
// use std::path::{Path, PathBuf};
use shim::ioerr;
use crate::fs;

struct MetaDir {
    name: String,
    children: Vec<String>
}

impl mfs::FileInfo for MetaDir {
    fn name(&self) -> &str {
        self.name.as_str()
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
}

impl mfs::Dir for MetaDir {
    fn entries<'a>(&'a self) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        let children = self.children.clone();
        Ok(Box::new(children.into_iter().map(|x| mfs::DirEntry::new(x, Metadata::default(), 0, true))))
    }
}

pub struct MetaFileSystem();

impl MetaFileSystem {
    pub fn new() -> Self {
        Self()
    }
}

impl mfs::FileSystem for MetaFileSystem {
    fn open(&self, manager: &fs::FileSystem, path: &Path) -> io::Result<mfs::Entry> {
        // TODO support meta not mounted at root? this would be complicated.
        let root = PathBuf::from("/");

        // debug!("path: {:?}", path);

        let path = root.join(path.strip_prefix("/").unwrap_or(path));

        // debug!("path2: {:?}", path);

        let mut children: Vec<String> = Vec::new();

        for mount in manager.mounts.iter() {
            // debug!("mount: {:?} ->", mount.path);
            if let Ok(tail) = mount.path.strip_prefix(path.as_path()) {
                // debug!("  tail: {:?}", tail);
                if let Some(base) = tail.components().next() {
                    // debug!("  base: {:?}", base);
                    let base: String = base.as_os_str().to_string_lossy().into();
                    // debug!("  base2: {:?}", base);

                    if !children.contains(&base) {
                        children.push(base);
                    }

                }
            }
        }

        // debug!("children: {:?}", children);

        if children.len() == 0 && path != root {
            return ioerr!(NotFound, "unknown directory")
        }

        let self_name: String = path.file_name().map_or("/", |x| x.to_str().unwrap_or("???")).into();

        Ok(mfs::Entry::Dir(Box::new(MetaDir { name: self_name, children })))
    }
}

