use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use core::time::Duration;

use hashbrown::HashMap;

use crate::cls::{CoreLocal, CoreMutex};
use crate::iosync::Global;
use crate::traps::{HyperTrapFrame, IRQ_EL, IRQ_RECURSION_DEPTH};

static CORE_EVENTS: CoreLocal<Global<Vec<PerfEvent>>> = CoreLocal::new_global(|| Vec::new());

struct GuestEvent {
    tpidr: u64,
    lr: u64,
}

struct HyperEvent {
    lr: u64,
}

struct PerfEvent {
    timestamp: Duration,
    guest: GuestEvent,
    hyper: Option<HyperEvent>,
}

impl PerfEvent {
    pub fn from_tf(tf: &mut HyperTrapFrame) -> Self {
        let timestamp = crate::timer::current_time();
        let is_exc = IRQ_RECURSION_DEPTH.get() > 1;

        let guest = GuestEvent { lr: (if is_exc { IRQ_EL.get() } else { tf.ELR_EL2 }), tpidr: tf.TPIDR_EL2 };

        let hyper = if is_exc {
            Some(HyperEvent { lr: tf.ELR_EL2 })
        } else {
            None
        };
        Self { timestamp, guest, hyper }
    }
}

pub fn prepare() {
    CORE_EVENTS.critical(|core| core.reserve(150_000));
}

pub fn record_event(tf: &mut HyperTrapFrame) -> bool {
    let event = PerfEvent::from_tf(tf);
    CORE_EVENTS.critical(|core| {
        if core.len() < core.capacity() {
            core.push(event);
            true
        } else {
            false
        }
    })
}

pub fn dump_events() {
    CORE_EVENTS.critical(|events| {
        info!("Events: {}", events.len());

        let debug_info = match crate::debug::debug_ref() {
            Some(d) => d,
            None => {
                info!("Cannot parse profiling info, not loaded.");
                return;
            }
        };

        let mut aggregate: HashMap<String, usize> = HashMap::new();

        for event in events.iter() {
            let mut name = String::from("<guest>");

            if let Some(hyper) = &event.hyper {
                name = String::from("<hyper>");

                if let Ok(mut iter) = debug_info.context.find_frames(hyper.lr) {
                    name = String::new();

                    let mut first = true;
                    loop {
                        let frame = match iter.next() {
                            Ok(Some(frame)) => frame,
                            _ => break,
                        };

                        if !first {
                            name += "->";
                        }
                        first = false;

                        if let Some(func) = frame.function {
                            let mangled = func.raw_name().unwrap();

                            if let Some(s2) = addr2line::demangle(mangled.as_ref(), gimli::DW_LANG_Rust) {
                                name += s2.as_str();
                            } else {
                                name += mangled.as_ref();
                            }
                        } else {
                            name += "???";
                        }

                        if let Some(location) = frame.location {
                            if let Some(line) = location.line {
                                name += "[:";
                                name.write_fmt(format_args!("{}", line));
                                name += "]";
                            }
                        }
                    }

                    if name.len() == 0 {
                        name = String::from("<unknown>");
                    }
                }
            }

            if let Some(item) = aggregate.get_mut(&name) {
                *item += 1;
            } else {
                aggregate.insert(name, 1);
            }
        }

        let mut functions: Vec<(String, usize)> = Vec::new();

        for (s, n) in aggregate.drain() {
            functions.push((s, n));
        }

        functions.sort_by_key(|(_, n)| usize::max_value() - *n);

        let mut displayed = 0;

        for (s, n) in functions.iter().take(50) {
            displayed += *n;
            if let Some(s2) = addr2line::demangle(s.as_str(), gimli::DW_LANG_Rust) {
                info!(" {:6}: {:?}", *n, s2);
            } else {
                info!(" {:6}: {:?}", *n, s);
            }
        }

        info!("{} other events", events.len() - displayed);
    });
}



