use alloc::vec::Vec;

use crate::cls::{CoreLocal, CoreMutex};
use crate::iosync::Global;
use crate::traps::{HyperTrapFrame, IRQ_RECURSION_DEPTH, IRQ_EL};

static CORE_EVENTS: CoreLocal<Global<Vec<PerfEvent>>> = CoreLocal::new_global(|| Vec::new());

struct GuestEvent {
    tpidr: u64,
    lr: u64,
}

struct HyperEvent {
    lr: u64,
}

struct PerfEvent {
    guest: GuestEvent,
    hyper: Option<HyperEvent>,
}

impl PerfEvent {
    pub fn from_tf(tf: &mut HyperTrapFrame) -> Self {
        let is_exc = IRQ_RECURSION_DEPTH.get() > 1;

        let guest = GuestEvent { lr: (if is_exc { IRQ_EL.get() } else { tf.ELR_EL2 }), tpidr: tf.TPIDR_EL2 };

        let hyper = if is_exc {
            Some(HyperEvent { lr: tf.ELR_EL2 })
        } else {
            None
        };
        Self { guest, hyper }
    }
}

pub fn prepare() {
    CORE_EVENTS.critical(|core| core.reserve(50_000));
}

pub fn record_event(tf: &mut HyperTrapFrame) {
    let event = PerfEvent::from_tf(tf);
    CORE_EVENTS.critical(|core| {
        if core.len() < core.capacity() {
            core.push(event);
        }
    });
}

pub fn dump_events() {

    CORE_EVENTS.critical(|events| {

        info!("Events: {}", events.len());

    });

}



