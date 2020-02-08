use alloc::string::String;

use shim::io::{self, SeekFrom};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle};
use crate::vfat::vfat::SeekHandle;

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    vfat: HANDLE,
    pub cluster: Cluster,
    pub name: String,
    pub metadata: Metadata,
    pub size: u32,
    pointer: SeekHandle,
}

impl<HANDLE: VFatHandle> File<HANDLE> {
    pub fn new(vfat: HANDLE, cluster: Cluster, name: String, metadata: Metadata, size: u32) -> File<HANDLE> {
        File {
            vfat,
            cluster,
            name,
            metadata,
            size,
            pointer: SeekHandle {
                cluster,
                offset: 0,
                total_offset: 0,
            },
        }
    }
}

impl<HANDLE: VFatHandle> traits::File for File<HANDLE> {
    fn sync(&mut self) -> io::Result<()> {
        unimplemented!()
    }

    fn size(&self) -> u64 {
        self.size as u64
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> io::Result<()> {
        unimplemented!()
    }
}


impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {

        if self.pointer.total_offset >= self.size as usize {
            return Ok(0);
        }

        let max_file_read = core::cmp::min((self.size as usize) - self.pointer.total_offset, buf.len());

        let (written, cloff) = self.vfat.lock(|fs| fs.read_cluster_unaligned(self.pointer, &mut buf[..max_file_read]))?;
        self.pointer = cloff;
        Ok(written)
    }
}

impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        unimplemented!("File::seek()")
    }
}
