
pub mod broadcom;
mod data_access;

pub use data_access::*;
use crate::console::CONSOLE;
use enumset::EnumSet;
use crate::process::{Process, HyperProcess};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use crate::vm::VirtualAddr;


#[derive(Debug, EnumSetType)]
pub enum IrqSource {
    GenericVirt,
    PeripheralTimer1,
}

impl IrqSource {
    pub fn irq_index(&self) -> usize  {
        match self {
            IrqSource::GenericVirt => 0,
            IrqSource::PeripheralTimer1 => 1,
        }
    }
}

pub struct IrqController {
    irqs: EnumSet<IrqSource>,
    mask: u64,
}

impl IrqController {
    pub fn new() -> Self {
        Self { irqs: EnumSet::empty(), mask: 0 }
    }

    pub fn assert(&mut self, irq: IrqSource) {
        self.irqs |= irq;
    }

    pub fn deassert(&mut self, irq: IrqSource) {
        self.irqs &= !irq;
    }

    pub fn set_asserted(&mut self, irq: IrqSource, val: bool) {
        if val {
            // if !self.irqs.contains(irq) {
            //     debug!("Asserting: {:?}", irq);
            // }
            self.assert(irq);
        } else {
            // if self.irqs.contains(irq) {
            //     debug!("Deasserting: {:?}", irq);
            // }
            self.deassert(irq);
        }
    }

    pub fn is_any_asserted(&self) -> bool {
        self.irq_bitmap_masked() != 0
    }

    pub fn irq_bitmap(&self) -> u64 {
        let mut map = 0;
        for irq in self.irqs.iter() {
            map |= (1 << irq.irq_index());
        }
        map
    }

    pub fn irq_bitmap_masked(&self) -> u64 {
        self.irq_bitmap() & self.mask
    }

    pub fn set_mask(&mut self, mask: u64) {
        self.mask = mask;
    }

    pub fn get_mask(&self) -> u64 {
        self.mask
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceError {
    Unmapped,
    BadSize,
    NotImplemented,
}

pub type Result<T> = core::result::Result<T, DeviceError>;

// virt device is expected to synchronize
pub trait VirtDevice : core::fmt::Debug + Send + Sync {

    /// Detect whether this device can handle an address.
    /// This should be used to check if a register is within a device's
    /// MMIO range.
    /// This is value must be cache-able, and, if false, may allow fallthrough
    /// to another device.
    fn is_mapped(&self, addr: VirtualAddr) -> bool;

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64>;

    fn write(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr, val: u64) -> Result<()>;

    fn update(&self, process: &mut HyperProcess) {
    }

}

fn assert_size(a: AccessSize, b: AccessSize) -> Result<()> {
    if a == b {
        Ok(())
    } else {
        Err(DeviceError::BadSize)
    }
}

#[derive(Debug)]
pub struct HwPassthroughDevice {
    start: u64,
    length: u64,
    verbose: bool,
}

impl HwPassthroughDevice {
    pub fn new(start: u64, length: u64) -> Self {
        Self { start, length, verbose: false }
    }

    pub fn new_verbose(start: u64, length: u64) -> Self {
        Self { start, length, verbose: true }
    }
}

impl VirtDevice for HwPassthroughDevice {
    fn is_mapped(&self, addr: VirtualAddr) -> bool {
        let addr = addr.as_u64();
        self.start <= addr && (addr - self.start) < self.length
    }

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64> {
        if !self.is_mapped(addr) {
            return Err(DeviceError::Unmapped);
        }

        let value = unsafe {
            match access.access_size {
                AccessSize::Byte => (addr.as_usize() as *const u8).read_volatile() as u64,
                AccessSize::HalfWord => (addr.as_usize() as *const u16).read_volatile() as u64,
                AccessSize::Word => (addr.as_usize() as *const u32).read_volatile() as u64,
                AccessSize::DoubleWord => (addr.as_usize() as *const u64).read_volatile(),
            }
        };

        if self.verbose {
            debug!("read(addr: {:#x?}, size: {:?}) -> {:#x}", addr, access.access_size, value);
        }

        Ok(value)
    }

    fn write(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr, val: u64) -> Result<()> {
        if !self.is_mapped(addr) {
            return Err(DeviceError::Unmapped);
        }

        if self.verbose {
            debug!("write(addr: {:#x?}, size: {:?}) <- {:#x}", addr, access.access_size, val);
        }

        unsafe {
            match access.access_size {
                AccessSize::Byte => (addr.as_usize() as *mut u8).write_volatile(val as u8),
                AccessSize::HalfWord => (addr.as_usize() as *mut u16).write_volatile(val as u16),
                AccessSize::Word => (addr.as_usize() as *mut u32).write_volatile(val as u32),
                AccessSize::DoubleWord => (addr.as_usize() as *mut u64).write_volatile(val),
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct StackedDevice {
    devices: VecDeque<Box<dyn VirtDevice>>,
}

impl StackedDevice {
    pub fn new() -> Self {
        Self { devices: VecDeque::new() }
    }

    pub fn add(&mut self, device: Box<dyn VirtDevice>) {
        self.devices.push_front(device);
    }
}

impl VirtDevice for StackedDevice {
    fn is_mapped(&self, addr: VirtualAddr) -> bool {
        for device in self.devices.iter() {
            if device.is_mapped(addr) {
                return true;
            }
        }
        false
    }

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64> {
        for device in self.devices.iter() {
            if device.is_mapped(addr) {
                return device.read(process, access, addr);
            }
        }
        Err(DeviceError::Unmapped)
    }

    fn write(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr, val: u64) -> Result<()> {
        for device in self.devices.iter() {
            if device.is_mapped(addr) {
                return device.write(process, access, addr, val);
            }
        }
        Err(DeviceError::Unmapped)
    }

    fn update(&self, process: &mut HyperProcess) {
        for device in self.devices.iter() {
            device.update(process);
        }
    }
}




