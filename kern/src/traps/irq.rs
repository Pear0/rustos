use alloc::boxed::Box;

use pi::interrupt::Interrupt;

use crate::mutex::Mutex;
use crate::traps::TrapFrame;

pub type IrqHandler = Box<dyn FnMut(&mut TrapFrame) + Send>;

#[derive(Copy, Clone, Default, Debug)]
pub struct IrqStats {
    pub count: u32,
}

struct IrqEntry {
    handler: Option<IrqHandler>,
    stats: IrqStats,
}

impl IrqEntry {
    fn new() -> IrqEntry {
        IrqEntry {
            handler: None,
            stats: Default::default(),
        }
    }

    fn record_stats(&mut self, _tf: &TrapFrame) {
        self.stats.count = self.stats.count.wrapping_add(1);
    }

}

type IrqHandlers = [IrqEntry; Interrupt::MAX];

pub struct Irq(Mutex<Option<IrqHandlers>>);

impl Irq {
    pub const fn uninitialized() -> Irq {
        Irq(Mutex::new(None))
    }

    pub fn initialize(&self) {
        *self.0.lock() = Some([
            IrqEntry::new(), IrqEntry::new(), IrqEntry::new(), IrqEntry::new(),
            IrqEntry::new(), IrqEntry::new(), IrqEntry::new(), IrqEntry::new(),
        ]);
    }

    /// Register an irq handler for an interrupt.
    /// The caller should assure that `initialize()` has been called before calling this function.
    pub fn register(&self, int: Interrupt, handler: IrqHandler) {
        self.0.lock().as_mut().unwrap()[Interrupt::to_index(int)].handler = Some(handler);
    }

    /// Executes an irq handler for the givven interrupt.
    /// The caller should assure that `initialize()` has been called before calling this function.
    pub fn invoke(&self, int: Interrupt, tf: &mut TrapFrame) {
        let lock = &mut self.0.lock();
        let entry = &mut lock.as_mut().unwrap()[Interrupt::to_index(int)];
        entry.record_stats(tf);
        if let Some(handler) = &mut entry.handler {
            handler(tf);
        }
    }

    pub fn get_stats(&self) -> [IrqStats; Interrupt::MAX] {
        let mut stats = [IrqStats::default(); Interrupt::MAX];
        for (i, entry) in self.0.lock().as_ref().unwrap().into_iter().enumerate() {
            stats[i] = entry.stats;
        }
        stats
    }

}
