use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::ioerr;

use crate::traits;
use crate::util::{VecExt, SliceExt};
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp, VFat, mnt};
use crate::vfat::{Cluster, Entry, File, VFatHandle};
use mountfs::mount::mfs;
use mountfs::mount;
use crate::vfat::mnt::DynVFatHandle;

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub cluster: Cluster,
    pub name: String,
    pub metadata: Metadata,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct VFatRegularDirEntry {
    name: [u8; 8],
    ext: [u8; 3],
    attributes: Attributes,
    __r0: u8,
    creation_time_tenths: u8,
    creation_time: Time,
    creation_date: Date,
    accessed_date: Date,
    cluster_high: u16,
    modified_time: Time,
    modified_date: Date,
    cluster_low: u16,
    file_size: u32,
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct VFatLfnDirEntry {
    sequence_number: u8,
    name_set_1: [u8; 10],
    attributes: u8,
    lfn_type: u8,
    name_checksum: u8,
    name_set_2: [u8; 12],
    __r0: [u8; 2],
    name_set_3: [u8; 4],
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    valid: u8,
    __r0: [u8; 10],
    attributes: u8,
    __r1: [u8; 20],
}

const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl VFatDirEntry {
    pub fn is_lfn(&self) -> bool {
        (unsafe { self.unknown.attributes } == 0xF)
    }

    pub fn is_deleted(&self) -> bool {
        let valid = unsafe { self.unknown.valid };
        valid == 0xE5
    }

    pub fn was_prev_last(&self) -> bool {
        let valid = unsafe { self.unknown.valid };
        valid == 0
    }
}

impl VFatRegularDirEntry {
    pub fn metadata(&self) -> Metadata {
        Metadata {
            attributes: self.attributes,
            creation: Timestamp {
                date: self.creation_date,
                time: self.creation_time,
            },
            last_accessed: Timestamp {
                date: self.accessed_date,
                time: Default::default(),
            },
            last_modified: Timestamp {
                date: self.modified_date,
                time: self.modified_time,
            }
        }
    }

    pub fn cluster(&self) -> Cluster {
        Cluster::from(((self.cluster_high as u32) << 16) | self.cluster_low as u32)
    }

    pub fn basic_name(&self) -> String {
        let mut s = String::new();
        for c in self.name.iter().take_while(|c| ![b'\0', b' '].contains(c)) {
            s.push((*c).into());
        }
        let mut added_dot = false;
        for c in self.ext.iter().take_while(|c| ![b'\0', b' '].contains(c)) {
            if !added_dot {
                s.push('.');
                added_dot = true;
            }
            s.push((*c).into());
        }

        s
    }
}

impl VFatLfnDirEntry {
    pub fn sequence_number(&self) -> u8 {
        self.sequence_number & 0b1_1111
    }
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {

    pub fn root(vfat: HANDLE) -> Dir<HANDLE> {
        let cluster = vfat.lock(|fs| fs.root_cluster());

        Dir {
            vfat: vfat.clone(),
            cluster,
            name: String::from("/"),
            metadata: Default::default()
        }
    }

    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        use traits::{Dir, Entry};

        let name = name.as_ref().to_str().ok_or(io::ErrorKind::InvalidInput)?;

        for entry in self.entries()? {
            if str::eq_ignore_ascii_case(entry.name(), name) {
                return Ok(entry)
            }
        }

        ioerr!(NotFound, "file not found")
    }
}

pub struct EntriesIterator<HANDLE: VFatHandle>  {
    vfat: HANDLE,
    buf: Vec<VFatDirEntry>,
    index: usize,
}

fn parse_lfns(lfns: &mut Vec<VFatLfnDirEntry>) -> String {
    lfns.sort_by(|a, b| a.sequence_number().cmp(&b.sequence_number()) );

    let mut buf: Vec<u8> = Vec::new();
    for lfn in lfns.iter() {
        for c in lfn.name_set_1.iter()
            .chain(lfn.name_set_2.iter())
            .chain(lfn.name_set_3.iter()) {
            buf.push(*c);
        }
    }

    let mut chars: Vec<u16> = Vec::new();
    for slice in buf.chunks(2) {
        let value = (slice[0] as u16) | ((slice[1] as u16) << 8);
        if value == 0 || value == 0xFFFF {
            break;
        }
        chars.push(value);
    }

    String::from_utf16_lossy(chars.as_slice())
}

impl<HANDLE: VFatHandle> Iterator for EntriesIterator<HANDLE> {
    type Item = Entry<HANDLE>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut lfns: Vec<VFatLfnDirEntry> = Vec::new();

        while self.index < self.buf.len() {
            let entry = &self.buf[self.index];

            // the previous entry was the last entry.
            if entry.was_prev_last() {
                return None;
            }

            // we're valid, next iteration check the next entry.
            self.index += 1;

            // skip this entry
            if entry.is_deleted() {
                continue;
            }

            // defer LFNs until we see the corresponding normal entry.
            if entry.is_lfn() {
                lfns.push(unsafe { entry.long_filename });
                continue;
            }

            let entry = unsafe { entry.regular };

            let name: String;
            if lfns.len() > 0 {
                name = parse_lfns(&mut lfns);
                lfns.clear();
            } else {
                name = entry.basic_name();
            }

            return Some(if entry.attributes.directory() {
                Entry::Dir(Dir {
                    vfat: self.vfat.clone(),
                    cluster: entry.cluster(),
                    name,
                    metadata: entry.metadata(),
                })
            } else {
                Entry::File(File::new(
                    self.vfat.clone(),
                    entry.cluster(),
                    name,
                    entry.metadata(),
                    entry.file_size,
                ))
            })
        }

        None
    }
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    type Entry = Entry<HANDLE>;
    type Iter = EntriesIterator<HANDLE>;

    fn entries(&self) -> io::Result<Self::Iter> {
        let mut buf: Vec<u8> = Vec::new();

        let result = self.vfat.lock(|fs| fs.read_chain(self.cluster, &mut buf))?;

        Ok(EntriesIterator {
            vfat: self.vfat.clone(),
            buf: unsafe { buf.cast() },
            index: 0,
        })
    }
}

pub(crate) fn convert_ts(t: Timestamp) -> mount::Timestamp {
    use traits::Timestamp;
    mount::Timestamp::new_from_fields(t.year() as u32, t.month(), t.day(), t.hour(), t.minute(), t.second())
}

impl mfs::FileInfo for Dir<DynVFatHandle> {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn metadata(&self) -> mount::Metadata {
        use traits::Metadata;
        mount::Metadata {
            read_only: Some(self.metadata.read_only()),
            hidden: Some(self.metadata.hidden()),
            created: Some(convert_ts(self.metadata.created())),
            accessed: Some(convert_ts(self.metadata.accessed())),
            modified: Some(convert_ts(self.metadata.modified())),
        }
    }

    fn size(&self) -> u64 {
        0
    }

    fn is_directory(&self) -> bool {
        true
    }
}

fn convert_entry(entry: Entry<DynVFatHandle>) -> mfs::DirEntry {
    match &entry {
        Entry::File(f) => {
            mfs::DirEntry::new(f.name.clone(), mfs::FileInfo::metadata(f), f.size as u64, false)
        },
        Entry::Dir(f) => {
            mfs::DirEntry::new(f.name.clone(), mfs::FileInfo::metadata(f), 0, true)
        },
    }
}

impl mfs::Dir for Dir<DynVFatHandle> {
    fn entries(&self) -> io::Result<Box<dyn Iterator<Item=mfs::DirEntry>>> {
        let entries = traits::Dir::entries(self)?;
        Ok(Box::new(entries.map(convert_entry)))
    }
}