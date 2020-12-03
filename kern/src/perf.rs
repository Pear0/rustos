use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use core::fmt::Write;
use core::sync::atomic::Ordering;
use core::time::Duration;

use hashbrown::HashMap;

use crate::cls::{CoreLocal, CoreMutex};
use crate::iosync::Global;
use crate::traps::{HyperTrapFrame, IRQ_EL, IRQ_RECURSION_DEPTH, KernelTrapFrame, IRQ_FP};
use crate::traps::hyper::{TM_TOTAL_COUNT, TM_TOTAL_TIME};
use common::fmt::ByteSize;
use crate::process::KernProcessCtx;
use crate::{NET, debug};
use crate::net::ipv4;

static CORE_EVENTS: CoreLocal<Global<EventData>> = CoreLocal::new_global(|| EventData {
    initialized: false,
    sample_count: 0,
    event_count: 0,
    dropped_sample_count: 0,
    dropped_event_count: 0,
    events: VecDeque::new(),
});

struct EventData {
    initialized: bool,

    // total counts
    sample_count: usize,
    event_count: usize,

    // failed to record counts
    dropped_sample_count: usize,
    dropped_event_count: usize,

    events: VecDeque<PerfEvent>,
}

impl EventData {
    pub fn record_sample(&mut self, events: &[PerfEvent]) -> bool {
        if !self.initialized {
            return false;
        }

        self.sample_count += 1;
        self.event_count += events.len();

        if self.events.len() + events.len() <= self.events.capacity() {
            for i in 0..events.len() {
                self.events.push_back(events[i]);
            }
            return true;
        }

        self.dropped_sample_count += 1;
        self.dropped_event_count += events.len();
        false
    }
}

#[derive(Clone, Copy, Debug)]
struct GuestEvent {
    lr: u64,
}

// kernel thread event
#[derive(Clone, Copy, Debug)]
struct KernelEvent {
    lr: u64,
}

#[derive(Clone, Copy, Debug)]
struct ExcEvent {
    lr: u64,
}

#[derive(Clone, Copy, Debug)]
struct HeaderEvent {
    timestamp: Duration,
    tpidr: u64,
}

#[derive(Clone, Copy, Debug)]
enum PerfEvent {
    Empty,
    Header(HeaderEvent),
    Exc(ExcEvent),
    Kernel(KernelEvent),
    Guest(GuestEvent),
}

impl PerfEvent {
    pub fn from_hyper_tf(tf: &mut HyperTrapFrame) -> Self {
        // let timestamp = crate::timer::current_time();
        // let is_exc = IRQ_RECURSION_DEPTH.get() > 1;
        //
        // let guest = GuestEvent {
        //     lr: (if is_exc { IRQ_EL.get() } else { tf.ELR_EL2 }),
        //     is_kernel_thread: false
        // };
        //
        // let hyper = if is_exc {
        //     Some(ExcEvent { lr: tf.ELR_EL2 })
        // } else {
        //     None
        // };
        // Self { timestamp, tpidr: tf.TPIDR_EL2, guest, exc: hyper }
        unimplemented!("unimplemented")
    }

    pub fn from_kernel_tf(tf: &mut KernelTrapFrame) -> Self {
        // let timestamp = crate::timer::current_time();
        // let is_exc = IRQ_RECURSION_DEPTH.get() > 1;
        //
        // let guest = GuestEvent {
        //     lr: (if is_exc { IRQ_EL.get() } else { tf.ELR_EL1 }),
        //     is_kernel_thread: tf.is_el1(),
        // };
        //
        // let hyper = if is_exc {
        //     Some(ExcEvent { lr: tf.ELR_EL1 })
        // } else {
        //     None
        // };
        // Self { timestamp, tpidr: tf.TPIDR_EL0, guest, exc: hyper }
        unimplemented!("unimplemented")
    }
}

pub fn prepare() {
    CORE_EVENTS.critical(|core| {
        core.initialized = true;
        core.events.reserve(150_000);
    });
}

pub fn record_event_hyper(tf: &mut HyperTrapFrame) -> bool {
    let event = PerfEvent::from_hyper_tf(tf);
    CORE_EVENTS.try_critical(|core| {
        core.record_sample(core::slice::from_ref(&event))
    }).unwrap_or(false)
}

pub fn record_event_kernel(tf: &mut KernelTrapFrame) -> bool {
    let mut events = [PerfEvent::Empty; 50];
    let mut events_len = 0;
    let mut append = |event: PerfEvent| {
        if events_len < events.len() {
            events[events_len] = event;
            events_len += 1;
        }
    };

    let timestamp = crate::timer::current_time();
    let is_exc = IRQ_RECURSION_DEPTH.get() > 1;
    let kern_thread = tf.is_el1();

    if !is_exc {
        return true;
    }

    append(PerfEvent::Header(HeaderEvent {
        timestamp,
        tpidr: tf.TPIDR_EL0,
    }));

    let mut proc_lr = tf.ELR_EL1;
    let mut proc_bp = tf.regs[29];

    if is_exc {
        append(PerfEvent::Exc(ExcEvent {
            lr: proc_lr,
        }));

        for frame in unsafe { debug::stack_walker_bp(proc_bp) } {
            append(PerfEvent::Exc(ExcEvent {
                lr: frame.link_register,
            }));
        }

        proc_lr = IRQ_EL.get();
        proc_bp = IRQ_FP.get();
    }

    if kern_thread {
        append(PerfEvent::Kernel(KernelEvent {
            lr: proc_lr,
        }));

        for frame in unsafe { debug::stack_walker_bp(proc_bp) } {
            append(PerfEvent::Kernel(KernelEvent {
                lr: frame.link_register,
            }));
        }
    } else {
        append(PerfEvent::Guest(GuestEvent {
            lr: proc_lr,
        }));

        // TODO stack walk guest???
    }

    CORE_EVENTS.try_critical(|core| {
        core.record_sample(&events[..events_len])
    }).unwrap_or(false)
}

pub fn dump_events() {
    CORE_EVENTS.critical(|events| {
        let perf_event = core::mem::size_of::<PerfEvent>();

        info!("current events: {}, event count: {} ({}), sample count: {}, dropped sample count: {}",
              events.events.len(), events.event_count, ByteSize::from(perf_event * events.event_count),
              events.sample_count, events.dropped_sample_count);

        info!("Event Storage: event={}, buffer: {} of {}",
              ByteSize::from(perf_event),
              ByteSize::from(perf_event * events.events.len()),
              ByteSize::from(perf_event * events.events.capacity()));

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
            while let Some(header_ev) = events.events.pop_front() {
                let header = match header_ev {
                    PerfEvent::Header(h) => h,
                    e => {
                        error!("got unexpected event: {:?}", e);
                        continue
                    }
                };

                let timestamp = header.timestamp.as_nanos() as u64;

                event_buf.clear();
                event_buf.extend_from_slice(&timestamp.to_le_bytes());
                event_buf.extend_from_slice(&event_index.to_le_bytes());
                event_buf.extend_from_slice(&0u16.to_le_bytes()); // num frames

                let mut num_frames = 0u16;

                while let Some(ev) = events.events.pop_front() {
                    match ev {
                        PerfEvent::Empty => error!("got PerfEvent::Empty"),
                        PerfEvent::Header(header) => {
                            events.events.push_front(PerfEvent::Header(header));
                            break;
                        }
                        PerfEvent::Exc(e) => {
                            event_buf.extend_from_slice(&e.lr.to_le_bytes());
                            num_frames += 1;
                        }
                        PerfEvent::Kernel(e) => {
                            event_buf.extend_from_slice(&e.lr.to_le_bytes());
                            num_frames += 1;
                        }
                        PerfEvent::Guest(e) => {
                            event_buf.extend_from_slice(&e.lr.to_le_bytes());
                            num_frames += 1;
                        }
                    }
                }

                {
                    let bytes = num_frames.to_le_bytes();
                    event_buf[16] = bytes[0];
                    event_buf[17] = bytes[1];
                }

                if full_message.len() + event_buf.len() > MAX_SIZE || num_events > 100 {
                    // roll everything back...

                    // but, drop this event if it's the only one and it's too big
                    if num_events > 0 {
                        continue;
                    }

                    events.events.push_front(header_ev);
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

