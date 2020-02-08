use core::fmt;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    bytes: [u8; 3],
    // 8 bits - head
    // 6 bits - sector  (Bits 6-7 are the upper two bits for the Starting Cylinder field)
    // 10 bits - cylinder
}

impl CHS {
    fn cylinder(&self) -> u16 {
        (self.bytes[2] as u16) | ((self.bytes[1] & 0b1100_0000u8) as u16) << 2
    }
    fn head(&self) -> u8 {
        self.bytes[0]
    }
    fn sector(&self) -> u8 {
        self.bytes[1] & 0b0011_1111u8
    }
}

impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CHS")
            .field("cylinder", &self.cylinder())
            .field("head", &self.head())
            .field("sector", &self.sector())
            .finish()
    }
}

const_assert_size!(CHS, 3);

#[repr(C, packed)]
#[derive(Debug)]
pub struct PartitionEntry {
    pub boot_indicator: u8,
    pub start: CHS,
    pub partition_type: u8,
    pub end: CHS,
    pub relative_sector: u32,
    pub total_sectors: u32,
}

const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436],
    pub disk_id: [u8; 10],
    pub partitions: [PartitionEntry; 4],
    signature: [u8; 2],
}

impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MasterBootRecord")
            .field("disk_id", &format_args!("{:?}", self.disk_id))
            .field("partitions", &format_args!("{:?}", self.partitions))
            .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut record = [0u8; 512];
        device.read_sector(0, &mut record).map_err(|e| Error::Io(e))?;

        let record: MasterBootRecord = unsafe { core::mem::transmute(record) };

        if record.signature != [0x55u8, 0xAAu8] {
            return Err(Error::BadSignature);
        }

        for (i, partition) in record.partitions.iter().enumerate() {
            if partition.boot_indicator != 0 && partition.boot_indicator != 0x80 {
                return Err(Error::UnknownBootIndicator(i as u8));
            }
        }

        Ok(record)
    }
}
