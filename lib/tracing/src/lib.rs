#![cfg_attr(not(test), no_std)]

#[allow(non_snake_case)]
#[allow(unused)]
pub(crate) mod tracestreaming_capnp {
    include!(concat!(env!("OUT_DIR"), "/tracestreaming_capnp.rs"));
    // include!("../target/debug/build/tracing-235b686e5ef4b759/out/tracestreaming_capnp.rs");
}

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;

#[derive(Default)]
pub struct TraceEvent {
    pub timestamp: u64,
    pub event_index: u64,
    pub frames: Vec<TraceFrame>,
}

#[derive(Default)]
pub struct TraceFrame {
    pub pc: u64,
}

struct SliceWriter<'a>(&'a mut [u8], usize);

impl<'a> capnp::io::Write for SliceWriter<'a> {
    fn write_all(&mut self, buf: &[u8]) -> capnp::Result<()> {
        self.1 += buf.len();
        if buf.len() > self.0.len() {
            return Err(capnp::Error::failed(String::from("buffer is not large enough")));
        }
        let amt = buf.len();
        let (a, b) = core::mem::replace(&mut self.0, &mut []).split_at_mut(amt);
        a.copy_from_slice(buf);
        self.0 = b;
        Ok(())
    }
}


pub fn encode_events(buffer: &mut [u8], build_id: u64, events: &[TraceEvent]) -> Result<usize, usize> {
    use tracestreaming_capnp::{trace_group};
    use capnp::serialize_packed;

    let mut message = ::capnp::message::Builder::new_default();

    let mut root = message.init_root::<trace_group::Builder>();

    root.set_build_id(build_id);

    let mut cap_events = root.init_events(events.len() as u32);
    for (i, event) in events.iter().enumerate() {
        let mut cap_ev = cap_events.reborrow().get(i as u32);
        cap_ev.set_timestamp(event.timestamp);
        cap_ev.set_event_index(event.event_index);

        let mut cap_frames = cap_ev.init_frames(event.frames.len() as u32);
        for (j, frame) in event.frames.iter().enumerate() {
            let mut cap_frame = cap_frames.reborrow().get(j as u32);
            cap_frame.set_pc(frame.pc);
        }
    }

    let mut writer = SliceWriter(buffer, 0);

    match serialize_packed::write_message(&mut writer, &message) {
        Ok(_) => Ok(writer.1),
        Err(_) => Err(writer.1),
    }
}

mod tests {
    #[test]
    fn foo() {
        use super::*;

        use tracestreaming_capnp::{trace_group};

        let mut message = ::capnp::message::Builder::new_default();

        let mut root = message.init_root::<trace_group::Builder>();

        root.set_build_id(5);

        let mut events = root.reborrow().init_events(2);
        events.reborrow().get(0).set_event_index(1);
        events.reborrow().get(1).set_event_index(2);

        println!("[1] event_index: {}", events.reborrow().get(1).get_event_index());

        drop(events);
        let mut events = root.init_events(3);

        println!("[1] event_index: {}", events.reborrow().get(1).get_event_index());


    }
}
