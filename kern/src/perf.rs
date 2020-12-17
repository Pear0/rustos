use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use hashbrown::HashMap;

use common::fmt::ByteSize;
use tracing;

use crate::{debug, NET, smp};
use crate::cls::{CoreLocal, CoreMutex};
use crate::iosync::{Global, Lazy};
use crate::net::ipv4;
use crate::process::KernProcessCtx;
use crate::traps::{HyperTrapFrame, IRQ_EL, IRQ_FP, IRQ_RECURSION_DEPTH, KernelTrapFrame};
use crate::traps::hyper::{TM_TOTAL_COUNT, TM_TOTAL_TIME};
use crate::kernel::{kernel_main, KERNEL_TIMER};
use crate::arm::VirtualCounter;
use gimli::AttributeValue::Virtuality;

static CORE_EVENTS: CoreLocal<Global<EventData>> = CoreLocal::new_global(|| EventData {
    initialized: false,
    sample_count: 0,
    event_count: 0,
    dropped_sample_count: 0,
    dropped_event_count: 0,
    events: VecDeque::new(),
});

static CORE_STATS: CoreLocal<Lazy<CoreStats>> = CoreLocal::new_lazy(|| CoreStats {
    total_tick_count: AtomicU64::new(0),
});

pub static PERF_EVENTS_ENABLED: AtomicBool = AtomicBool::new(true);

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

struct CoreStats {
    pub total_tick_count: AtomicU64,
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
    CORE_STATS.total_tick_count.fetch_add(1, Ordering::Relaxed);

    if !PERF_EVENTS_ENABLED.load(Ordering::Relaxed) {
        return false;
    }

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
    CORE_EVENTS.cross(0).critical(|events| {
        let perf_event = core::mem::size_of::<PerfEvent>();

        info!("current events: {}, event count: {} ({}), sample count: {}, dropped sample count: {}",
              events.events.len(), events.event_count, ByteSize::from(perf_event * events.event_count),
              events.sample_count, events.dropped_sample_count);

        info!("Event Storage: event={}, buffer: {} of {}",
              ByteSize::from(perf_event),
              ByteSize::from(perf_event * events.events.len()),
              ByteSize::from(perf_event * events.events.capacity()));

        info!("total perf ticks: {}", CORE_STATS.total_tick_count.load(Ordering::Relaxed));

    });
}

struct EventStreamer {
    /// Store events ready to be sent.
    event_queue: Vec<tracing::TraceEvent>,

    /// Store TraceEvent structures after they've
    /// been used so that the resources (frames vec)
    /// can be reused.
    event_cache: Vec<tracing::TraceEvent>,

    event_limit: usize,

    event_index: u64,
    build_id: u64,
}

impl EventStreamer {
    pub fn new(build_id: u64) -> Self {
        Self {
            event_queue: Vec::new(),
            event_cache: Vec::new(),
            event_limit: 10,
            event_index: 0,
            build_id,
        }
    }

    fn get_or_create_event(&mut self) -> Option<tracing::TraceEvent> {
        if self.event_cache.len() > 0 {
            return self.event_cache.pop();
        }

        if self.event_queue.len() + self.event_cache.len() < self.event_limit {
            // we can allocate one
            return Some(tracing::TraceEvent::default());
        }

        None
    }

    /// Try parse raw event data into event structs. Return true if we parsed an event.
    fn try_process_event(&mut self) -> bool {
        let mut event = match self.get_or_create_event() {
            Some(e) => e,
            None => return false
        };

        event.event_index = self.event_index;
        event.frames.clear();
        event.frames.reserve(100);

        let mut wrote_event = false;

        CORE_EVENTS.cross(0).critical(|events| {
            while let Some(header_ev) = events.events.pop_front() {
                let header = match header_ev {
                    PerfEvent::Header(h) => h,
                    e => {
                        error!("got unexpected event: {:?}", e);
                        continue;
                    }
                };

                event.timestamp = header.timestamp.as_nanos() as u64;

                while let Some(ev) = events.events.pop_front() {
                    match ev {
                        PerfEvent::Empty => error!("got PerfEvent::Empty"),
                        PerfEvent::Header(header) => {
                            events.events.push_front(PerfEvent::Header(header));
                            break;
                        }
                        PerfEvent::Exc(e) => {
                            if event.frames.len() < event.frames.capacity() {
                                event.frames.push(tracing::TraceFrame { pc: e.lr });
                            }
                        }
                        PerfEvent::Kernel(e) => {
                            if event.frames.len() < event.frames.capacity() {
                                event.frames.push(tracing::TraceFrame { pc: e.lr });
                            }
                        }
                        PerfEvent::Guest(e) => {
                            if event.frames.len() < event.frames.capacity() {
                                event.frames.push(tracing::TraceFrame { pc: e.lr });
                            }
                        }
                    }
                }

                wrote_event = true;
                break;
            }
        });

        if wrote_event {
            self.event_queue.push(event);
            true
        } else {
            event.frames.clear();
            event.event_index = 0;
            self.event_cache.push(event);
            false
        }
    }

    fn recycle_events(&mut self, count: usize) {
        for mut event in self.event_queue.drain(..count) {
            event.event_index = 0;
            event.timestamp = 0;
            event.frames.clear();
            self.event_cache.push(event);
        }
    }

    /// send some events over UDP. return true if more events to send.
    fn send_events(&mut self, address: ipv4::Address) -> bool {
        const MAX_SIZE: usize = 512;
        let mut send_buffer = [0u8; MAX_SIZE];

        if self.event_queue.is_empty() {
            return false;
        }

        for num_events in (1..=self.event_queue.len()).rev() {
            match tracing::encode_events(&mut send_buffer, self.build_id, &self.event_queue[..num_events]) {
                Ok(size) => {

                    // info!("{}", pretty_hex::pretty_hex(&&send_buffer[..size]));

                    let result = NET.critical(|net| {
                        net.send_datagram(address, 4005, 4000, &send_buffer[..size])
                    });

                    if let Err(e) = result {
                        info!("failed to send data: {:?}", e);
                    }

                    // remove events we sent
                    self.recycle_events(num_events);

                    // info!("sent {} events, with size {}", num_events, size);

                    // are there any more events?
                    return !self.event_queue.is_empty();
                }
                Err(size) => {
                    if num_events == 1 {
                        warn!("tried to send {} events, but size {} > {}", num_events, size, MAX_SIZE);
                    }
                }
            }
        }

        // if we are here, then we failed to send even 1 event.
        // in that case, lets drop the event in case it is too large to send.
        self.recycle_events(1);

        !self.event_queue.is_empty()
    }
}

#[inline(never)]
fn create_build_id(id: &[u8]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&id[..8]);
    u64::from_le_bytes(bytes)
}

pub fn perf_stream_proc(ctx: KernProcessCtx) {
    const MAGIC: u32 = 0x54445254;
    const VERSION: u16 = 1;
    const MAX_SIZE: usize = 512;

    info!("perf_stream_proc()");
    info!("BUILD_ID: {:?}", crate::debug::build_id());

    let build_id = create_build_id(crate::debug::build_id());

    info!("BUILD_ID: {:#x}", build_id);

    let address = "239.15.55.200".parse::<ipv4::Address>().unwrap();

    // wait for network...
    while !NET.is_initialized() {
        kernel_api::syscall::sleep(Duration::from_millis(200));
    }

    let mut streamer = EventStreamer::new(build_id);

    loop {
        let mut only_yield = false;
        while streamer.try_process_event() {
            only_yield = true;
        }

        while streamer.send_events(address) {
            only_yield = true;
        }

        if only_yield {
            kernel_api::syscall::sched_yield();
        } else {
            kernel_api::syscall::sleep(Duration::from_millis(10));
        }
    }
}

