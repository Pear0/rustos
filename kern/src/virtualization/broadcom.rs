use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use crate::console::CONSOLE;
use crate::mutex::Mutex;
use crate::process::{HyperProcess, Process};
use crate::virtualization::{DataAccess, DeviceError, IrqSource, Result, VirtDevice};
use crate::vm::VirtualAddr;
use crate::iosync::{SyncWrite, SyncRead};
use crate::sync::Waitable;

#[derive(Debug)]
pub struct Interrupts();

impl Interrupts {
    const BASE: u64 = 0x3F00B200;

    pub fn new() -> Self {
        Self()
    }
}

impl VirtDevice for Interrupts {
    fn is_mapped(&self, addr: VirtualAddr) -> bool {
        (Self::BASE..Self::BASE + 0x100).contains(&addr.as_u64())
    }

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64> {
        let addr = addr.as_u64() - Self::BASE;

        let irqs = process.detail.irqs.irq_bitmap_masked();

        match addr {
            0x04 => {
                Ok(irqs & u32::max_value() as u64)
            }
            0x08 => {
                Ok((irqs >> 32) & u32::max_value() as u64)
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }

    fn write(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr, val: u64) -> Result<()> {
        let addr = addr.as_u64() - Self::BASE;

        const LOW_MASK: u64 = u32::max_value() as u64;

        match addr {
            // interrupt enable registers
            // TODO interrupt disable registers
            0x10 => {
                let mut mask = process.detail.irqs.get_mask();
                // mask &= !low_mask;
                mask |= val & LOW_MASK;
                process.detail.irqs.set_mask(mask);

                Ok(())
            }
            0x14 => {
                let mut mask = process.detail.irqs.get_mask();
                // mask &= low_mask;
                mask |= (val & LOW_MASK) << 32;
                process.detail.irqs.set_mask(mask);

                Ok(())
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }
}

#[derive(Debug)]
struct SystemTimerImpl {
    matched: [bool; 4],
    compare: [u32; 4],
    last_compared: [u64; 4],
}

impl SystemTimerImpl {
    fn check_matches(&mut self, process: &HyperProcess) {
        let now = process.current_cpu_time().as_micros() as u64;

        for i in 0..4 {
            if !self.matched[i] {
                let timer_diff = now - self.last_compared[i];
                let compare_diff = self.compare[i].wrapping_sub(self.last_compared[i] as u32) as u64;

                if timer_diff >= compare_diff {
                    self.matched[i] = true;
                }
            }
            self.last_compared[i] = now;
        }
    }
}

#[derive(Debug)]
pub struct SystemTimer(Mutex<SystemTimerImpl>);

impl SystemTimer {
    const BASE: u64 = 0x3F003000;

    pub fn new() -> Self {
        Self(mutex_new!(SystemTimerImpl {
            matched: [false; 4],
            compare: [0; 4],
            last_compared: [0; 4],
        }))
    }
}

impl VirtDevice for SystemTimer {
    fn is_mapped(&self, addr: VirtualAddr) -> bool {
        (Self::BASE..Self::BASE + 0x1000).contains(&addr.as_u64())
    }

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64> {
        let addr = addr.as_u64() - Self::BASE;

        match addr {
            0x0 => {
                let mut l = m_lock!(self.0);
                l.check_matches(process);

                let mut ret = 0;
                for i in 0..4 {
                    if l.matched[i] {
                        ret |= (1 << i);
                    }
                }

                Ok(ret)
            }

            // Counter low 32 bits
            0x4 => {
                let micros = process.current_cpu_time().as_micros() as u64;
                Ok(micros & u32::max_value() as u64)
            }
            // Counter high 32 bits
            0x8 => {
                let micros = process.current_cpu_time().as_micros() as u64;
                Ok(micros >> 32)
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }

    fn write(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr, val: u64) -> Result<()> {
        let addr = addr.as_u64() - Self::BASE;
        let now = process.current_cpu_time().as_micros() as u64;

        match addr {
            0x0 => {
                let mut l = m_lock!(self.0);
                for i in 0..4 {
                    if val & (1 << i) != 0 {
                        l.matched[i] = false;
                    }
                }

                // info!("timer 1: {}", l.matched[1]);

                l.check_matches(process);

                // info!("timer 1 - 2: {}", l.matched[1]);

                process.detail.irqs.set_asserted(IrqSource::PeripheralTimer1, l.matched[1]);
                Ok(())
            }
            // Compare reg
            0xC | 0x10 | 0x14 | 0x18 => {
                let mut l = m_lock!(self.0);
                let idx = ((addr - 0xC) / 4) as usize;

                l.compare[idx] = val as u32;
                let low_mask = u32::max_value() as u64;
                l.last_compared[idx] = (now & !low_mask) | (val & low_mask);
                if l.last_compared[idx] < now {
                    l.last_compared[idx] += 0x1_0000_0000;
                }

                // debug!("will trigger timer1: {:?}", l.last_compared[idx] - now);

                Ok(())
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }

    fn update(&self, process: &mut HyperProcess) {
        let mut l = m_lock!(self.0);
        // let b = l.matched[1];
        l.check_matches(process);
        // if !b && l.matched[1] {
        //     info!("now asserted!");
        // }
        process.detail.irqs.set_asserted(IrqSource::PeripheralTimer1, l.matched[1]);
    }
}

#[derive(Default, Debug)]
struct MiniUartImpl {
    aux_enable: u64,
    ier: u64,
    // interrupt enable
    iir: u64,
    // interrupt identify
    lcr: u64,
    // line control
    mcr: u64,
    // modem control
    msr: u64,
    // modem status
    scratch: u64,
    // scratch
    cntl: u64,
    // extra control
    stat: u64,
    // extra status
    baud: u64, // baud rate
}

impl MiniUartImpl {
    pub fn by_addr_mut(&mut self, addr: u64) -> Option<&mut u64> {
        match addr {
            0x04 => Some(&mut self.aux_enable),

            0x44 => Some(&mut self.ier),
            0x48 => Some(&mut self.iir),
            0x4C => Some(&mut self.lcr),

            0x50 => Some(&mut self.mcr),
            0x58 => Some(&mut self.msr),
            0x5C => Some(&mut self.scratch),

            0x60 => Some(&mut self.cntl),
            0x64 => Some(&mut self.stat),
            0x68 => Some(&mut self.baud),

            _ => None,
        }
    }

    pub fn by_addr(&self, addr: u64) -> Option<u64> {
        // by_addr_mut() is only mutable to return a mutable reference.
        // by using unsafe here, we keep it DRY
        unsafe { &mut *(self as *const Self as *mut Self) }.by_addr_mut(addr).map(|x| *x)
    }
}

#[derive(Debug)]
pub struct MiniUart(Mutex<MiniUartImpl>);

impl MiniUart {
    const BASE: u64 = 0x3f215000;

    pub fn new() -> Self {
        Self(mutex_new!(MiniUartImpl::default()))
    }
}

impl VirtDevice for MiniUart {
    fn is_mapped(&self, addr: VirtualAddr) -> bool {
        (Self::BASE..Self::BASE + 0x1000).contains(&addr.as_u64())
    }

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64> {
        let addr = addr.as_u64() - Self::BASE;

        let mut lock = m_lock!(self.0);

        match addr {
            // data in
            0x40 => {
                let mut value: u8 = 0;

                if let Some((_, source)) = &mut process.detail.serial {
                    source.read(core::slice::from_mut(&mut value)); // TODO handle error?
                }

                Ok(value as u64)
            }
            // status register
            0x54 => {
                // assert_size(access.access_size, AccessSize::Word)?;
                let mut lsr = 0;
                lsr |= 1 << 5; // TxAvailable

                if let Some((_, source)) = &mut process.detail.serial {
                    if source.done_waiting() {
                        lsr |= 1; // DataReady
                    }
                }

                return Ok(lsr);
            }
            e if lock.by_addr(e).is_some() => {
                Ok(*lock.by_addr_mut(e).unwrap())
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }

    fn write(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr, val: u64) -> Result<()> {
        let addr = addr.as_u64() - Self::BASE;
        let now = process.current_cpu_time().as_micros() as u64;

        let mut lock = m_lock!(self.0);

        match addr {
            // data out
            0x40 => {

                if let Some((sink, _)) = &mut process.detail.serial {
                    sink.write(&[val as u8]); // TODO do something if we can't write?
                }

                Ok(())
            }
            e if lock.by_addr(e).is_some() => {
                *lock.by_addr_mut(e).unwrap() = val;
                Ok(())
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }

    fn update(&self, process: &mut HyperProcess) {
        let mut lock = m_lock!(self.0);

        let mut do_assert = false;

        let (send, receive) = ((lock.ier & 0b10) != 0, (lock.ier & 0b1) != 0);

        if let Some((sink, source)) = &mut process.detail.serial {

            if send && sink.done_waiting() {
                do_assert = true;
            }
            if receive && source.done_waiting() {
                do_assert = true;
            }
        }

        process.detail.irqs.set_asserted(IrqSource::Aux, do_assert);

    }
}


pub struct LocalPeripheralsImpl {
    pub virtual_counter_enable: [AtomicBool; 4],

}

impl LocalPeripheralsImpl {
    pub fn new() -> Self {
        Self {
            virtual_counter_enable: [AtomicBool::default(), AtomicBool::default(), AtomicBool::default(), AtomicBool::default()],
        }
    }
}

#[derive(Debug)]
pub struct LocalPeripherals();

impl LocalPeripherals {
    pub const BASE: u64 = 0x4000_0000;
    pub const LENGTH: u64 = 0x2_0000;

    pub fn new() -> Self {
        Self()
    }
}

impl VirtDevice for LocalPeripherals {
    fn is_mapped(&self, addr: VirtualAddr) -> bool {
        (Self::BASE..Self::BASE + Self::LENGTH).contains(&addr.as_u64())
    }

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64> {
        let addr = addr.as_u64() - Self::BASE;

        match addr {
            0x60 | 0x64 | 0x68 | 0x6C => {
                let mut addr = addr;
                // The FIQ registers start at 0x70 (+0x10 from IRQ registers).
                if addr < 0x70 {
                    addr += 0x10;
                }

                let mut value = unsafe { ((Self::BASE + addr) as *const u32).read_volatile() };
                let mut mask = 0;

                // Allowed irqs that get passed to guest.

                mask |= 1 << 1; // CNTPNS IRQ
                mask |= 1 << 3; // CNTV IRQ

                value &= mask;

                Ok(value as u64)
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }

    fn write(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr, val: u64) -> Result<()> {
        let addr = addr.as_u64() - Self::BASE;

        match addr {
            0x08 => {
                if val != 0x80000000 {
                    warn!("guest set timer prescaler to non-standard {:#x}, which is ignored.", val);
                }
                Ok(())
            }
            0x40 | 0x44 | 0x48 | 0x4C => {
                let core_id = (addr - 0x40) as usize / 4;
                let virtual_en = (val & (1 << 3)) != 0;
                process.detail.local_peripherals.virtual_counter_enable[core_id].store(virtual_en, Ordering::Relaxed);

                info!("virt counters: {:?}", process.detail.local_peripherals.virtual_counter_enable);

                Ok(())
            }
            _ => Err(DeviceError::NotImplemented),
        }
    }
}



