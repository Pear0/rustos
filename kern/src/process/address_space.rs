use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;

use kernel_api::{OsError, OsResult};

use crate::param::{PAGE_ALIGN, PAGE_MASK, PAGE_SIZE};
use crate::virtualization::VirtDevice;
use crate::vm::{GuestPageTable, PagePerm, PhysicalAddr, UserPageTable, VirtualAddr};
use crate::process::ProcessImpl;

#[derive(Debug)]
pub enum KernelRegionKind {
    Normal,
}

#[derive(Debug)]
pub enum HyperRegionKind {
    Normal,
    Emulated(Arc<dyn VirtDevice>),
}

pub struct Region<T: ProcessImpl> {
    start: usize,
    length: usize,
    pub kind: T::RegionKind,
}

impl<T: ProcessImpl> Region<T> {
    pub fn new(start: VirtualAddr, length: usize, kind: T::RegionKind) -> Self {
        Self { start: start.as_usize(), length, kind }
    }

    pub fn repaint(&self, table: &mut T::PageTable) {
        assert_eq!(self.start % PAGE_SIZE, 0);
        assert_eq!(self.length % PAGE_SIZE, 0);

        // debug!("Repainting region 0x{:x}", self.start);

        // careful to avoid wrapping on 0xFFFFFFF0000 (stack) + 0x1000 == 0
        for offset in (0..self.length).step_by(PAGE_SIZE) {
            let base = self.start + offset;
            if !table.is_valid(VirtualAddr::from(base)) {
                // debug!("base not valid, allocating... 0x{:x}", base);
                table.alloc(VirtualAddr::from(base), PagePerm::RWX);
            } else {
                // debug!("base is valid, skipping 0x{:x}", base);
            }
        }
    }

    pub fn can_grow_up(&self, len: usize) -> bool {
        len % PAGE_SIZE == 0
    }

    pub fn grow_up(&mut self, table: &mut T::PageTable, len: usize) {
        if !self.can_grow_up(len) {
            panic!("invalid call to grow_up()");
        }
        self.length += len;
        self.repaint(table);
    }
}

impl<T: ProcessImpl> fmt::Debug for Region<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Region")
            .field("start", &self.start)
            .field("length", &self.length)
            .field("kind", &self.kind)
            .finish()
    }
}

pub struct AddressSpaceManager<T: ProcessImpl> {
    pub regions: Vec<Region<T>>,
    pub table: T::PageTable,
}

impl<T: ProcessImpl> AddressSpaceManager<T> {
    pub fn new() -> Self {
        Self {
            // vector sorted bty
            regions: Vec::new(),
            table: T::PageTable::new(),
        }
    }

    pub fn add_region(&mut self, region: Region<T>) -> OsResult<()> {
        if region.start % PAGE_SIZE != 0 || region.length % PAGE_SIZE != 0 {
            return Err(OsError::InvalidArgument);
        }

        let after: Option<(usize, &Region<T>)> = self.regions.iter().enumerate().find(|(_, reg)| reg.start > region.start);

        let before: Option<&Region<T>> = match after {
            None => self.regions.last(),
            Some((i, _)) => self.regions[..i].last(),
        };

        if let Some(before) = before {
            // overlap with previous region!
            if before.start + before.length > region.start {
                return Err(OsError::InvalidArgument);
            }
        }

        if let Some((_, after)) = after {
            // overlap with next region!
            if region.start + region.length > after.start {
                return Err(OsError::InvalidArgument);
            }
        }

        let index = after.map(|(x, _)| x).unwrap_or(self.regions.len());
        self.regions.insert(index, region);

        self.regions.get(index).unwrap().repaint(&mut self.table);

        Ok(())
    }

    pub fn get_region_idx(&self, va: VirtualAddr) -> Option<usize> {
        let va = va.as_usize();
        self.regions.iter()
            .enumerate()
            .find(|(_, region)| region.start <= va && va < region.start + region.length)
            .map(|(i, _)| i)
    }

    pub fn get_region(&self, va: VirtualAddr) -> Option<&Region<T>> {
        let va = va.as_usize();
        self.regions.iter().find(|region| region.start <= va && va < region.start + region.length)
    }

    pub fn get_region_mut(&mut self, va: VirtualAddr) -> Option<&mut Region<T>> {
        let va = va.as_usize();
        self.regions.iter_mut().find(|region| region.start <= va && va < region.start + region.length)
    }

    pub fn expand_region(&mut self, va: VirtualAddr, length: usize) -> OsResult<()> {
        if length % PAGE_SIZE != 0 {
            return Err(OsError::InvalidArgument);
        }
        let region = self.get_region_idx(va).ok_or(OsError::BadAddress)?;

        if !self.regions[region].can_grow_up(length) {
            return Err(OsError::Unknown);
        }

        self.regions[region].grow_up(&mut self.table, length);
        Ok(())
    }

    pub fn get_page_mut(&mut self, va: VirtualAddr) -> Option<&mut [u8]> {
        unsafe { self.table.get_page_ref(va) }
    }

    pub fn copy_out(&mut self, va: VirtualAddr, mut buf: &mut [u8]) -> OsResult<()> {
        let mut base = va & VirtualAddr::from(PAGE_MASK);
        let mut offset = (va - base).as_usize();

        while buf.len() > 0 {
            let mut page = self.get_page_mut(base).ok_or(OsError::BadAddress)?;
            // offset is always less than page size.
            if offset > 0 {
                page = &mut page[offset..];
                offset = 0;
            }

            let len = core::cmp::min(page.len(), buf.len());
            buf[..len].copy_from_slice(&page[..len]);
            buf = &mut buf[len..];
            base = base + VirtualAddr::from(PAGE_SIZE);
        }

        Ok(())
    }

    pub fn copy_in(&mut self, va: VirtualAddr, mut buf: &[u8]) -> OsResult<()> {
        let mut base = va & VirtualAddr::from(PAGE_MASK);
        let mut offset = (va - base).as_usize();

        while buf.len() > 0 {
            let mut page = self.get_page_mut(base).ok_or(OsError::BadAddress)?;
            // offset is always less than page size.
            if offset > 0 {
                page = &mut page[offset..];
                offset = 0;
            }

            let len = core::cmp::min(page.len(), buf.len());
            page[..len].copy_from_slice(&buf[..len]);
            buf = &buf[len..];
            base = base + VirtualAddr::from(PAGE_SIZE);
        }

        Ok(())
    }

    pub fn get_baddr(&self) -> PhysicalAddr {
        self.table.get_baddr()
    }
}



