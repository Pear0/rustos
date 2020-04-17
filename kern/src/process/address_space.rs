use alloc::vec::Vec;
use crate::vm::{UserPageTable, VirtualAddr, PagePerm, PhysicalAddr};
use crate::param::{PAGE_SIZE, PAGE_MASK, PAGE_ALIGN};
use kernel_api::{OsResult, OsError};

#[derive(Debug)]
pub enum RegionKind {
    Normal,
}

#[derive(Debug)]
pub struct Region {
    start: usize,
    length: usize,
    kind: RegionKind,
}

impl Region {
    pub fn new(start: VirtualAddr, length: usize, kind: RegionKind) -> Self {
        Self { start: start.as_usize(), length, kind }
    }

    pub fn repaint(&self, table: &mut UserPageTable) {
        assert_eq!(self.start % PAGE_SIZE, 0);
        assert_eq!(self.length % PAGE_SIZE, 0);

        // debug!("Repainting region 0x{:x}", self.start);

        // careful to avoid wrapping on 0xFFFFFFF0000 (stack) + 0x1000 == 0
        for offset in (0..self.length).step_by(PAGE_SIZE) {
            let base = self.start + offset;
            if !table.is_valid( VirtualAddr::from(base)) {
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

    pub fn grow_up(&mut self, table: &mut UserPageTable, len: usize) {
        if !self.can_grow_up(len) {
            panic!("invalid call to grow_up()");
        }
        self.length += len;
        self.repaint(table);
    }
}

pub struct AddressSpaceManager {
    pub regions: Vec<Region>,
    pub table: UserPageTable,
}

impl AddressSpaceManager {
    pub fn new() -> Self {
        Self {
            // vector sorted bty
            regions: Vec::new(),
            table: UserPageTable::new(),
        }
    }

    pub fn add_region(&mut self, region: Region) -> OsResult<()> {
        if region.start % PAGE_SIZE != 0 || region.length % PAGE_SIZE != 0 {
            return Err(OsError::InvalidArgument);
        }

        let after: Option<(usize, &Region)> = self.regions.iter().enumerate().find(|(_, reg)| reg.start > region.start);

        let before: Option<&Region> = match after {
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
            .find(|(_, region)|  region.start <= va && va < region.start + region.length)
            .map(|(i, _)| i)
    }

    pub fn get_region(&self, va: VirtualAddr) -> Option<&Region> {
        let va = va.as_usize();
        self.regions.iter().find(|region|  region.start <= va && va < region.start + region.length)
    }

    pub fn get_region_mut(&mut self, va: VirtualAddr) -> Option<&mut Region> {
        let va = va.as_usize();
        self.regions.iter_mut().find(|region|  region.start <= va && va < region.start + region.length)
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

    pub fn get_baddr(&self) -> PhysicalAddr {
        self.table.get_baddr()
    }

}



