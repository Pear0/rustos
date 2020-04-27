use crate::mutex::Mutex;
use crate::process::{HyperProcess, Process};
use crate::virtualization::{DataAccess, DeviceError, IrqSource, Result, VirtDevice};
use crate::vm::VirtualAddr;

#[derive(Debug)]
pub struct Interrupts();

impl Interrupts {
    const BASE: u64 = 0x3F00B000;

    pub fn new() -> Self {
        Self()
    }
}

impl VirtDevice for Interrupts {
    fn is_mapped(&self, addr: VirtualAddr) -> bool {
        (Self::BASE..Self::BASE + 0x1000).contains(&addr.as_u64())
    }

    fn read(&self, process: &mut HyperProcess, access: &DataAccess, addr: VirtualAddr) -> Result<u64> {
        let addr = addr.as_u64() - Self::BASE;

        let irqs = process.detail.irqs.irq_bitmap();

        match addr {
            0x204 => {
                Ok(irqs & u32::max_value() as u64)
            }
            0x208 => {
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
            0x210 => {
                let mut mask = process.detail.irqs.get_mask();
                // mask &= !low_mask;
                mask |= val & LOW_MASK;
                process.detail.irqs.set_mask(mask);

                Ok(())
            }
            0x214 => {
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






