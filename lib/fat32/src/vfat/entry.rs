use crate::traits;
use crate::vfat::{self, Dir, File, Metadata, VFatHandle};
use core::fmt;

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    fn name(&self) -> &str {
        match self {
            Entry::File(f) => f.name.as_str(),
            Entry::Dir(d) => d.name.as_str(),
        }
    }

    fn metadata(&self) -> &Self::Metadata {
        match self {
            Entry::File(f) => &f.metadata,
            Entry::Dir(d) => &d.metadata,
        }
    }

    fn as_file(&self) -> Option<&File<HANDLE>> {
        match self {
            Entry::File(f) => Some(f),
            Entry::Dir(_) => None,
        }
    }

    fn as_dir(&self) -> Option<&Dir<HANDLE>> {
        match self {
            Entry::File(_) => None,
            Entry::Dir(d) => Some(d),
        }
    }

    fn into_file(self) -> Option<File<HANDLE>> {
        match self {
            Entry::File(f) => Some(f),
            Entry::Dir(_) => None,
        }
    }

    fn into_dir(self) -> Option<Dir<HANDLE>> {
        match self {
            Entry::File(_) => None,
            Entry::Dir(d) => Some(d),
        }
    }
}
