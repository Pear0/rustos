use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    pub attributes: Attributes,
    pub creation: Timestamp,
    pub last_accessed: Timestamp,
    pub last_modified: Timestamp,
}

impl Attributes {
    pub fn read_only(&self) -> bool {
        (self.0 & 0x1) != 0
    }
    pub fn hidden(&self) -> bool {
        (self.0 & 0x2) != 0
    }
    pub fn system(&self) -> bool {
        (self.0 & 0x4) != 0
    }
    pub fn volume_id(&self) -> bool {
        (self.0 & 0x8) != 0
    }
    pub fn directory(&self) -> bool {
        (self.0 & 0x10) != 0
    }
    pub fn archive(&self) -> bool {
        (self.0 & 0x20) != 0
    }
}

impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        1980 + (self.date.0 >> 9) as usize
    }

    fn month(&self) -> u8 {
        ((self.date.0 >> 5) & 0b1111) as u8
    }

    fn day(&self) -> u8 {
        (self.date.0 & 0b1_1111) as u8
    }

    fn hour(&self) -> u8 {
        (self.time.0 >> 11) as u8
    }

    fn minute(&self) -> u8 {
        ((self.time.0 >> 5) & 0b11_1111) as u8
    }

    fn second(&self) -> u8 {
        ((self.time.0 & 0b1_1111) * 2) as u8
    }
}

impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        self.attributes.read_only()
    }

    fn hidden(&self) -> bool {
        self.attributes.hidden()
    }

    fn created(&self) -> Self::Timestamp {
        self.creation
    }

    fn accessed(&self) -> Self::Timestamp {
        self.last_accessed
    }

    fn modified(&self) -> Self::Timestamp {
        self.last_modified
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use traits::Timestamp;
        f.write_fmt(format_args!("{:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}", self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second()))
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use traits::Metadata;
        f.debug_struct("Metadata")
            .field("read_only", &self.read_only())
            .field("hidden", &self.hidden())
            .field("created", &self.creation)
            .field("modified", &self.last_modified)
            .field("accessed", &self.last_accessed)
            .finish()
    }
}
