use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use core::fmt::Write;
use core::sync::atomic::Ordering;
use core::time::Duration;

use hashbrown::HashMap;

use crate::cls::{CoreLocal, CoreMutex};
use crate::iosync::Global;
use crate::traps::{HyperTrapFrame, IRQ_EL, IRQ_RECURSION_DEPTH, KernelTrapFrame};
use crate::traps::hyper::{TM_TOTAL_COUNT, TM_TOTAL_TIME};
use common::fmt::ByteSize;
use crate::process::KernProcessCtx;
use crate::NET;
use crate::net::ipv4;

static CORE_EVENTS: CoreLocal<Global<VecDeque<PerfEvent>>> = CoreLocal::new_global(|| VecDeque::new());

struct GuestEvent {
    tpidr: u64,
    lr: u64,
    is_kernel_thread: bool,
}

struct ExcEvent {
    lr: u64,
}

struct PerfEvent {
    timestamp: Duration,
    guest: GuestEvent,
    exc: Option<ExcEvent>,
}

impl PerfEvent {
    pub fn from_hyper_tf(tf: &mut HyperTrapFrame) -> Self {
        let timestamp = crate::timer::current_time();
        let is_exc = IRQ_RECURSION_DEPTH.get() > 1;

        let guest = GuestEvent {
            lr: (if is_exc { IRQ_EL.get() } else { tf.ELR_EL2 }),
            tpidr: tf.TPIDR_EL2,
            is_kernel_thread: false
        };

        let hyper = if is_exc {
            Some(ExcEvent { lr: tf.ELR_EL2 })
        } else {
            None
        };
        Self { timestamp, guest, exc: hyper }
    }

    pub fn from_kernel_tf(tf: &mut KernelTrapFrame) -> Self {
        let timestamp = crate::timer::current_time();
        let is_exc = IRQ_RECURSION_DEPTH.get() > 1;

        let guest = GuestEvent {
            lr: (if is_exc { IRQ_EL.get() } else { tf.ELR_EL1 }),
            tpidr: tf.TPIDR_EL0,
            is_kernel_thread: tf.is_el1(),
        };

        let hyper = if is_exc {
            Some(ExcEvent { lr: tf.ELR_EL1 })
        } else {
            None
        };
        Self { timestamp, guest, exc: hyper }
    }
}

pub fn prepare() {
    CORE_EVENTS.critical(|core| core.reserve(150_000));
}

pub fn record_event_hyper(tf: &mut HyperTrapFrame) -> bool {
    let event = PerfEvent::from_hyper_tf(tf);
    CORE_EVENTS.try_critical(|core| {
        if core.len() < core.capacity() {
            core.push_back(event);
            true
        } else {
            false
        }
    }).unwrap_or(false)
}

pub fn record_event_kernel(tf: &mut KernelTrapFrame) -> bool {
    let event = PerfEvent::from_kernel_tf(tf);
    CORE_EVENTS.try_critical(|core| {
        if core.len() < core.capacity() {
            core.push_back(event);
            true
        } else {
            false
        }
    }).unwrap_or(false)
}

pub fn dump_events() {
    CORE_EVENTS.critical(|events| {
        info!("Events: {}", events.len());

        let perf_event = core::mem::size_of::<PerfEvent>();
        info!("Event Storage: event={}, buffer: {} of {}",
              ByteSize::from(perf_event),
              ByteSize::from(perf_event * events.len()),
              ByteSize::from(perf_event * events.capacity()));


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
            let mut lr = None;
            if let Some(hyper) = &event.exc {
                name = String::from("<hyper>");
                lr = Some(hyper.lr);
            } else if event.guest.is_kernel_thread {
                lr = Some(event.guest.lr);
            }

            if let Some(kernel_lr) = lr {
                if let Ok(mut iter) = debug_info.context.find_frames(kernel_lr) {
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

        let tm_count = TM_TOTAL_COUNT.load(Ordering::Relaxed);
        if tm_count > 0 {
            let tm_time = Duration::from_micros(TM_TOTAL_TIME.load(Ordering::Relaxed));

            info!("TM Count: {}, TM Time: {:?}", tm_count, tm_time);
            info!("Per: {:?}", tm_time / (tm_count as u32));
        }
    });
}

pub fn perf_stream_proc(ctx: KernProcessCtx) {
    const MAGIC: u32 = 0x54445254;
    const VERSION: u16 = 1;
    const MAX_SIZE: usize = 512;

    info!("perf_stream_proc()");
    info!("BUILD_ID: {:?}", &crate::debug::BUILD_ID);

    let address = "239.15.55.200".parse::<ipv4::Address>().unwrap();

    // wait for network...
    while !NET.is_initialized() {
        kernel_api::syscall::sleep(Duration::from_millis(200));
    }

    let mut event_index: u64 = 0;

    let mut full_message: Vec<u8> = Vec::new();
    full_message.reserve(2000);
    let mut event_buf: Vec<u8> = Vec::new();
    event_buf.reserve(2000);

    loop {
        let mut num_events: u16 = 0;

        full_message.clear();
        full_message.extend_from_slice(&MAGIC.to_le_bytes());
        full_message.extend_from_slice(&VERSION.to_le_bytes());
        full_message.extend_from_slice(&0u16.to_le_bytes()); // num_events

        CORE_EVENTS.critical(|events| {
            while let Some(ev) = events.pop_front() {

                let timestamp = ev.timestamp.as_nanos() as u64;

                event_buf.clear();
                event_buf.extend_from_slice(&timestamp.to_le_bytes());
                event_buf.extend_from_slice(&event_index.to_le_bytes());
                event_buf.extend_from_slice(&1u16.to_le_bytes()); // num frames

                let mut lr: u64;
                if let Some(exc) = &ev.exc {
                    lr = exc.lr;
                } else {
                    lr = ev.guest.lr;
                }

                event_buf.extend_from_slice(&lr.to_le_bytes());

                if full_message.len() + event_buf.len() > MAX_SIZE || num_events > 100 {
                    // roll everything back...

                    // but, drop this event if it's the only one and it's too big
                    if num_events > 0 {
                        continue;
                    }

                    events.push_front(ev);
                    break;
                }

                // otherwise, we are good to add...
                full_message.append(&mut event_buf);
                num_events += 1;
                event_index += 1;
            }
        });

        // early exit with optimistically longer delay
        if num_events == 0 {
            kernel_api::syscall::sleep(Duration::from_millis(150));
            continue;
        }

        {
            let bytes = num_events.to_le_bytes();
            full_message[6] = bytes[0];
            full_message[7] = bytes[1];
        }

        assert!(full_message.len() <= MAX_SIZE);

        let result = NET.critical(|net| {
            let msg = "hello world!";
            net.send_datagram(address, 4005, 4000, full_message.as_slice())
        });

        if let Err(e) = result {
            info!("failed to send data: {:?}", e);
        }

        kernel_api::syscall::sleep(Duration::from_millis(5));
    }
}

