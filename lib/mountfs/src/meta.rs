use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use hashbrown::HashMap;
use spin::Mutex;

use shim::{ioerr, newioerr};
use shim::ffi::{OsStr, OsString};
use shim::io;
use shim::path::{Component, Path};

use crate::fs;
use crate::fs::FileSystem;
use crate::mount::{Metadata, mfs};
use crate::mount::mfs::{Dir, FileId, FileInfo, FsId, INode};

struct MetaDir {
    name: String,
    id: FileId,
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

    fn get_id(&self) -> FileId {
        self.id
    }
}

impl mfs::Dir for MetaDir {
    // fn entries<'a>(&'a self) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
    //     let children = self.children.clone();
    //     Ok(Box::new(children.into_iter().map(|x| mfs::DirEntry::new(x, Metadata::default(), 0, true))))
    // }
}

#[derive(Debug)]
struct DirTree {
    id: INode,
    name: OsString,
    children: HashMap<OsString, DirTree>,
}

impl DirTree {
    pub fn new(name: OsString, id: INode) -> Self {
        Self {
            id,
            name,
            children: HashMap::new(),
        }
    }
}

pub struct MetaFileSystem {
    id: FsId,
    virt_id: AtomicUsize,
    virt: Mutex<DirTree>,
}

impl MetaFileSystem {
    pub fn new() -> Self {
        Self {
            id: 0,
            virt_id: AtomicUsize::new(1),
            virt: Mutex::new(DirTree::new(OsString::from("/"), 0)),
        }
    }

    fn ensure_dir_tree(&self, manager: &fs::FileSystem) {
        let mut root = self.virt.lock();

        for mount in manager.filesystems.values() {
            let mut node = root.deref_mut();
            for comp in mount.path.components() {
                if let Component::Normal(c) = comp {
                    let name = c.to_os_string();

                    if node.children.contains_key(&name) {
                        node = node.children.get_mut(&name).unwrap();
                    } else {
                        node.children.insert(name.clone(), DirTree::new(
                            name.clone(), self.virt_id.fetch_add(1, Ordering::Relaxed) as INode));
                        node = node.children.get_mut(&name).unwrap();
                    }
                }
            }
        }
    }

    fn find_dir(node: &DirTree, id: INode) -> Option<&DirTree> {
        if node.id == id {
            return Some(node);
        }

        for child in node.children.values() {
            if let Some(s) = Self::find_dir(child, id) {
                return Some(s);
            }
        }

        None
    }
}

impl mfs::FileSystem for MetaFileSystem {
    fn set_id(&mut self, id: usize) {
        self.id = id
    }

    fn get_name(&self) -> Option<String> {
        Some(String::from("meta"))
    }

    fn open(&self, manager: &fs::FileSystem, path: &Path) -> io::Result<mfs::Entry> {
        self.ensure_dir_tree(manager);
        // TODO support meta not mounted at root? this would be complicated.

        assert_eq!(path.to_str(), Some("/"));

        Ok(mfs::Entry::Dir(Arc::new(MetaDir { id: FileId(self.id, 0), name: String::from("/") })))
    }

    fn entries(&self, manager: &FileSystem, dir: Arc<dyn Dir>) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        self.ensure_dir_tree(manager);

        let id = dir.get_id();

        let lock = self.virt.lock();

        let node = Self::find_dir(lock.deref(), id.1).ok_or(newioerr!(NotFound, "no directory"))?;

        let mut children = Vec::new();
        for child in node.children.values() {
            children.push(mfs::DirEntry::new(
                String::from(child.name.to_str().unwrap()),
                Metadata::default(),
                0,
                true,
                FileId(id.0, child.id),
            ));
        }

        Ok(Box::new(children.into_iter()))
    }

    fn dir_entry(&self, manager: &FileSystem, dir: Arc<dyn Dir>, path: &OsStr) -> io::Result<mfs::Entry> {
        self.ensure_dir_tree(manager);

        let entry = self.entries(manager, dir)?.find(|d| OsString::from(d.name()).as_os_str() == path);
        if let Some(entry) = entry {
            Ok(mfs::Entry::Dir(Arc::new(MetaDir {
                name: entry.name,
                id: entry.id,
            })))
        } else {
            ioerr!(NotFound, "no dir entry")
        }
    }
}

