use alloc::boxed::Box;

use pi::interrupt::{Interrupt, CoreInterrupt};

use crate::mutex::Mutex;
use crate::traps::KernelTrapFrame;
use crate::smp;
use core::time::Duration;
use crate::process::ProcessImpl;

pub type IrqHandler<T> = Box<dyn FnMut(&mut T) + Send>;

#[derive(Copy, Clone, Default, Debug)]
pub struct IrqStats {
    pub count: u32,
}

struct IrqEntry<T> {
    handler: Option<IrqHandler<T>>,
    stats: IrqStats,
}

impl<T> IrqEntry<T> {
    fn new() -> Self {
        IrqEntry {
            handler: None,
            stats: Default::default(),
        }
    }

    fn record_stats(&mut self, _tf: &T) {
        self.stats.count = self.stats.count.wrapping_add(1);
    }

}

type IrqHandlers<T> = [IrqEntry<T>; Interrupt::MAX];
type CoreIrqHandlers<T> = [IrqEntry<T>; CoreInterrupt::MAX];

fn new_core_irqs<T>() -> CoreIrqHandlers<T> {
    [IrqEntry::new(), IrqEntry::new(), IrqEntry::new(), IrqEntry::new(),
        IrqEntry::new(), IrqEntry::new(), IrqEntry::new(), IrqEntry::new(),
        IrqEntry::new(), IrqEntry::new(), IrqEntry::new(), IrqEntry::new()]
}

struct CoreIrq<T> {
    handlers: [Mutex<Option<CoreIrqHandlers<T>>>; smp::MAX_CORES],
}

pub struct Irq<T: ProcessImpl>(Mutex<Option<IrqHandlers<T::Frame>>>, CoreIrq<T::Frame>);

impl<T: ProcessImpl> Irq<T> {
    pub const fn uninitialized() -> Irq<T> {
        Irq(mutex_new!(None), CoreIrq { handlers: [
            mutex_new!(None), mutex_new!(None),
            mutex_new!(None), mutex_new!(None)
        ] })
    }

    pub fn initialize(&self) {
        *m_lock!(self.0) = Some([
            IrqEntry::new(), IrqEntry::new(), IrqEntry::new(), IrqEntry::new(),
            IrqEntry::new(), IrqEntry::new(), IrqEntry::new(), IrqEntry::new(),
        ]);

        for core in self.1.handlers.iter() {
            *m_lock!(core) = Some(new_core_irqs());
        }

    }

    /// Register an irq handler for an interrupt.
    /// The caller should assure that `initialize()` has been called before calling this function.
    pub fn register(&self, int: Interrupt, handler: IrqHandler<T::Frame>) {
        m_lock!(self.0).as_mut().unwrap()[Interrupt::to_index(int)].handler = Some(handler);
    }

    pub fn register_core(&self, core: usize, int: CoreInterrupt, handler: IrqHandler<T::Frame>) {
        m_lock!(self.1.handlers[core]).as_mut().unwrap()[int as usize].handler = Some(handler);
    }

    /// Executes an irq handler for the givven interrupt.
    /// The caller should assure that `initialize()` has been called before calling this function.
    pub fn invoke(&self, int: Interrupt, tf: &mut T::Frame) -> bool {
        let lock = &mut m_lock!(self.0);
        let entry = &mut lock.as_mut().unwrap()[Interrupt::to_index(int)];
        entry.record_stats(tf);
        if let Some(handler) = &mut entry.handler {
            handler(tf);
            true
        } else {
            false
        }
    }

    pub fn invoke_core(&self, core: usize, int: CoreInterrupt, tf: &mut T::Frame) -> bool {
        let lock = &mut m_lock!(self.1.handlers[core]);
        let entry = &mut lock.as_mut().unwrap()[int as usize];
        entry.record_stats(tf);
        if let Some(handler) = &mut entry.handler {
            handler(tf);
            true
        } else {
            false
        }
    }

    pub fn get_stats(&self) -> Option<[IrqStats; Interrupt::MAX]> {
        let mut stats = [IrqStats::default(); Interrupt::MAX];
        for (i, entry) in m_lock_timeout!(self.0, Duration::from_millis(1))?.as_ref().unwrap().into_iter().enumerate() {
            stats[i] = entry.stats;
        }
        Some(stats)
    }

    pub fn get_stats_core(&self, core: usize) -> Option<[IrqStats; CoreInterrupt::MAX]> {
        let mut stats = [IrqStats::default(); CoreInterrupt::MAX];
        for (i, entry) in m_lock_timeout!(self.1.handlers[core], Duration::from_millis(1))?.as_ref().unwrap().into_iter().enumerate() {
            stats[i] = entry.stats;
        }
        Some(stats)
    }

}
