use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{self, Write};
use core::ops::AddAssign;
use core::slice::from_mut;
use core::time::Duration;

use dsx::sync::mutex::LockableMutex;
use hashbrown::{HashMap, HashSet};
use modular_bitfield::prelude::*;

use pi::timer;
use pi::types::{BigU16, BigU32};
use shim::io;

use crate::fs::handle::{Sink, Source};
use crate::iosync::{ConsoleSync, Global, ReadWrapper, SyncRead, SyncWrite};
use crate::mutex::Mutex;
use crate::net::{encode_struct, ipv4, NetErrorKind, NetResult, try_parse_struct};
use crate::net::buffer::BufferHandle;
use crate::net::ipv4::IPv4Payload;
use crate::net::util::ChecksumOnesComplement;
use crate::process::Process;
use crate::shell;

// Works with aliases - just for the showcase.
type Vitamin = B12;

/// Bitfield struct with 32 bits in total.
#[bitfield]
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Flags {
    ns: bool,
    _res: B3,
    data_offset: B4,

    fin: bool,
    syn: bool,
    rst: bool,
    psh: bool,
    ack: bool,
    urg: bool,
    ece: bool,
    cwr: bool,

}

#[repr(C, packed)]
#[derive(Clone, Debug, Default)]
pub struct Header {
    pub source_port: BigU16,
    pub destination_port: BigU16,
    pub sequence_number: BigU32,
    pub ack_number: BigU32,
    pub flags: Flags,

    pub window_size: BigU16,
    pub checksum: BigU16,
    pub urgent_pointer: BigU16,

}

#[derive(Debug, Clone)]
pub struct TcpFrame {
    pub header: Header,
    pub payload: Box<[u8]>,
}

impl TcpFrame {
    pub fn dump(&self) {
        let h = &self.header.flags;

        kprintln!("Packet:");
        kprint!("data_offset: {:?}, ", h.get_data_offset());
        kprint!("ns: {:?}, ", h.get_ns());
        kprint!("cwr: {:?}, ", h.get_cwr());
        kprint!("ece: {:?}, ", h.get_ece());
        kprint!("urg: {:?}, ", h.get_urg());
        kprint!("ack: {:?}, ", h.get_ack());
        kprint!("psh: {:?}, ", h.get_psh());
        kprint!("rst: {:?}, ", h.get_rst());
        kprint!("syn: {:?}, ", h.get_syn());
        kprint!("fin: {:?}, ", h.get_fin());
        kprintln!("");
    }

    fn total_length(&self) -> u16 {
        20 + self.payload.len() as u16
    }
}

impl IPv4Payload for TcpFrame {
    const PROTOCOL_NUMBER: u8 = 0x06;

    fn try_parse(buf: &[u8]) -> Option<(Self, &[u8])> {
        let (header, mut buf) = try_parse_struct::<Header>(buf)?;

        let header_len = 4 * header.flags.get_data_offset() as usize;

        if header_len < 20 {
            kprintln!("dropping tcp frame with header smaller than 20");
            return None;
        }

        if header_len > 20 {
            buf = &buf[header_len - 20..];
        }

        let frame = TcpFrame {
            header,
            payload: Box::from(buf),
        };

        Some((frame, &buf[..0]))
    }

    fn encode<'a>(&self, buf: &'a mut [u8], header: &ipv4::IPv4Header) -> NetResult<&'a mut [u8]> {
        // calculate checksum with pseudo header
        let mut check = ChecksumOnesComplement::new();
        check.ingest_sized(&header.source);
        check.ingest_sized(&header.destination);
        check.ingest_sized(&BigU16::new(Self::PROTOCOL_NUMBER as u16));
        check.ingest_sized(&BigU16::new(self.total_length()));

        let mut tcp_header = self.header.clone();

        // we don't support fancy stuff!
        tcp_header.flags.set_ece(false);
        tcp_header.flags.set_cwr(false);
        tcp_header.flags.set_data_offset(5);

        tcp_header.checksum.set(0);
        check.ingest_sized(&tcp_header);
        check.ingest_u8_pad(self.payload.as_ref());
        let check = check.get();
        // kprintln!("checksum: 0x{:04x}", check);
        tcp_header.checksum.set(check);

        let buf = encode_struct(buf, &tcp_header).ok_or(NetErrorKind::EncodeFail)?;

        let len = self.payload.len();
        buf[..len].copy_from_slice(self.payload.as_ref());

        Ok(&mut buf[len..])
    }
}

type Socket = (ipv4::Address, u16);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionKey {
    pub local: Socket,
    pub remote: Socket,
}

// ref. http://www.medianet.kent.edu/techreports/TR2005-07-22-tcp-EFSM.pdf

// struct ConnState {
//     seq_number: u32,
//     remote_seq_number: u32,
// }

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum State {
    Closed,
    Listen,

    // Connection Establishment
    C_SynSent,
    S_SynReceived,

    Established,

    // Close initiator
    FinWait1(Duration),
    // sent at
    FinWait2,
    TimeWait(Duration),

    // Close receiver
    CloseWait,
    LastAck,

    Broken,
    // custom lol
    Killed, // signal to the manager to remove this connection.
}

fn empty_payload() -> Box<[u8]> {
    let empty: [u8; 0] = [];
    let empty: &[u8] = &empty;
    Box::from(empty)
}

struct PendingPacket {
    queued: Duration,
    latest_attempt: Option<Duration>,
    // for unsent, last failed send. for unacked, last send
    dest: ipv4::Address,
    frame: TcpFrame,
}

impl PendingPacket {
    pub fn new(dest: ipv4::Address, frame: TcpFrame) -> Self {
        Self {
            queued: timer::current_time(),
            latest_attempt: None,
            dest,
            frame,
        }
    }
    pub fn update_attempt(mut self) -> Self {
        self.latest_attempt = Some(timer::current_time());
        self
    }
}

#[derive(Copy, Clone, Debug)]
struct SeqRing {
    value: u32,
}

impl fmt::Display for SeqRing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl SeqRing {
    pub fn new(value: u32) -> Self {
        Self {
            value,
        }
    }

    pub fn get(&self) -> u32 {
        self.value
    }

    pub fn add(&self, val: u32) -> Self {
        Self::new(self.value.wrapping_add(val))
    }

    pub fn is_recent(&self, val: u32) -> bool {
        self.value.wrapping_add(val) < 120_000
    }
}

impl AddAssign for SeqRing {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(rhs.get());
    }
}


struct TcpConnection {
    pub local: Socket,
    pub remote: Socket,
    state: State,
    seq_number: SeqRing,
    acked_number: SeqRing,
    // what we've acknowledged. the next sequence number we expect from remote
    remote_acked_number: u32, // the last ack number we know remote has

    recv: Sink,
    send: Source,

    // packets that failed to send for whatever reason. link down, whatever
    unsent_packets: VecDeque<PendingPacket>,

    // packets that have not yet been acknowledged and may need to be
    // resent.
    // TODO packets are never removed from this buffer.
    unacked_packets: Vec<PendingPacket>,
}

impl TcpConnection {
    pub fn new(local: Socket, remote: Socket, state: State, recv: Sink, send: Source) -> Self {
        Self {
            local,
            remote,
            state,
            seq_number: SeqRing::new(0),
            acked_number: SeqRing::new(0),
            remote_acked_number: 0,
            recv,
            send,
            unsent_packets: VecDeque::new(),
            unacked_packets: Vec::new(),
        }
    }

    fn send_packet(&mut self, manager: &ConnectionManager, flags: Flags, payload: Box<[u8]>) -> NetResult<()> {
        // we set most fields, header is provided for weird flags
        let dest = self.remote.0.clone();

        let mut header = Header::default();
        header.flags = flags.clone();

        header.destination_port.set(self.remote.1);
        header.source_port.set(self.local.1);
        header.ack_number.set(self.acked_number.get());
        header.sequence_number.set(self.seq_number.get());

        let mut window: u16 = 0xFF_FF;
        if let Some(w) = self.recv.estimate_free_capacity() {
            if w < window as usize {
                window = w as u16;
            }
        }

        header.window_size.set(window);

        let payload_len = payload.len();
        let frame = TcpFrame { header, payload };

        // resending? lol as if
        {
            let m = m_lock!(manager.inner);
            if let Err(e) = m.ip.send(dest, &frame) {
                if !e.is_spurious() {
                    return Err(e);
                }

                kprintln!("failed to send packet, putting in queue: {:?}", e);
                self.unsent_packets.push_back(PendingPacket::new(dest, frame).update_attempt());
                // no return here
            } else {
                // Ok
                self.unacked_packets.push(PendingPacket::new(dest, frame).update_attempt());
            }
        }

        // we sent successfully
        if flags.get_syn() {
            self.seq_number = self.seq_number.add(1);
        } else {
            self.seq_number = self.seq_number.add(payload_len as u32);
        }

        Ok(())
    }

    fn raw_send_bytes(&mut self, manager: &ConnectionManager, mut buf: &[u8]) -> NetResult<usize> {
        if buf.len() > 500 {
            buf = &buf[..500];
        }

        let mut flags = Flags::default();
        flags.set_psh(true);
        flags.set_ack(true);
        self.send_packet(manager, flags, Box::from(buf))?;

        Ok(buf.len())
    }

    fn handle_packet(&mut self, manager: &ConnectionManager, ip_header: &ipv4::IPv4Header, frame: &TcpFrame) {
        // kprintln!("TCP State: {:?}", self.state);
        match self.state {
            State::Listen => {
                // TODO we assume this is a SYN lol

                self.acked_number = SeqRing::new(frame.header.sequence_number.get()).add(1);
                self.state = State::S_SynReceived;

                let mut flags = Flags::default();
                flags.set_syn(true);
                flags.set_ack(true);

                self.send_packet(manager, flags, empty_payload());
            }
            State::C_SynSent => {}
            State::S_SynReceived => {
                if frame.header.sequence_number.get() != self.acked_number.get() {
                    trace!("Got packet with seq mismatch: {} != {}", frame.header.sequence_number.get(), self.acked_number);
                    return;
                }

                // self.acked_number += 1;

                if frame.header.flags.get_ack() {
                    self.state = State::Established;
                    self.remote_acked_number = frame.header.ack_number.get();
                }
            }
            state @ State::Established | state @ State::FinWait1(_) | state @ State::FinWait2 | state @ State::CloseWait => {

                // TODO not a proper connection close.
                if frame.header.flags.get_rst() {
                    debug!("got RST");
                    self.state = State::Killed;

                    let mut flags = Flags::default();
                    flags.set_ack(true);
                    flags.set_rst(true);
                    self.send_packet(manager, flags, empty_payload());

                    return;
                }

                self.remote_acked_number = frame.header.ack_number.get();

                if frame.header.sequence_number.get() != self.acked_number.get() {
                    trace!("Got packet with seq mismatch: {} != {:?}", frame.header.sequence_number.get(), self.acked_number);
                    return;
                }

                // TODO drop packets from send queue covered byt this ack

                // TODO packets arriving out of order. an handling dropped ACKs we've sent.

                if frame.payload.len() != 0 {
                    use crate::iosync::SyncWrite;
                    match self.recv.write(frame.payload.as_ref()) {
                        Err(e) => {
                            // ack no bytes

                            warn!("failed to write but acked: {}", e);
                        }
                        Ok(n) if n != frame.payload.len() => {
                            trace!("trunc'd ack because {} bytes could not be written", frame.payload.len() - n);

                            // Ack only the bytes we were able to write into the buffer
                            if n != 0 {
                                self.acked_number = self.acked_number.add(n as u32);
                                let mut flags = Flags::default();
                                flags.set_ack(true);
                                self.send_packet(manager, flags, empty_payload());
                            }
                        }
                        _ => {

                            // Ack all the bytes
                            self.acked_number = self.acked_number.add(frame.payload.len() as u32);
                            let mut flags = Flags::default();
                            flags.set_ack(true);
                            self.send_packet(manager, flags, empty_payload());
                        }
                    };
                }

                if frame.header.flags.get_fin() {
                    let now = timer::current_time();
                    self.state = match state {
                        State::Established => State::CloseWait,
                        State::FinWait1(_) if frame.header.flags.get_ack() => State::TimeWait(now),
                        State::FinWait1(_) => state, // we need an ack for the FIN we sent.
                        State::FinWait2 => State::TimeWait(now),
                        State::CloseWait => state,
                        _ => panic!("invalid state"),
                    };

                    let mut flags = Flags::default();
                    flags.set_ack(true);
                    self.send_packet(manager, flags, empty_payload());
                } else if (match state {
                    State::FinWait1(_) => true,
                    _ => false
                }) && frame.header.flags.get_ack() {
                    self.state = State::FinWait2;
                }
            }
            State::LastAck => {
                if frame.header.flags.get_ack() {
                    self.state = State::Closed;
                    return;
                }
            }
            State::TimeWait(_) | State::Closed | State::Broken | State::Killed => {}
        }
    }

    fn send_some_data(&mut self, manager: &ConnectionManager) -> bool {
        use crate::iosync::SyncRead;
        let mut buf = [0u8; 500];

        match self.send.read(&mut buf) {
            Ok(n) if n > 0 => {
                self.raw_send_bytes(manager, &buf[0..n]);
                true
            }
            Err(e) => {
                debug!("failed to read source: {}", e);
                false
            }
            _ => {
                false
            }
        }
    }

    fn resend_packets(&mut self, manager: &ConnectionManager) -> bool {
        'resend_loop: for _ in 0..self.unsent_packets.len() {
            match self.unsent_packets.pop_front() {
                Some(pending) => {
                    debug!("resending packet");
                    match m_lock!(manager.inner).ip.send(pending.dest, &pending.frame) {
                        Ok(_) => {
                            debug!("successfully resent packet");
                            self.unacked_packets.push(PendingPacket::new(pending.dest, pending.frame).update_attempt());
                        }
                        Err(e) => {
                            if e.is_spurious() {
                                debug!("failed to resend packet, error: {:?}", e);
                                self.unsent_packets.push_back(pending.update_attempt());
                                break 'resend_loop;
                            } else {
                                debug!("failed to send packet: {:?}", e);
                            }
                        }
                    }
                }
                None => break 'resend_loop,
            }
        }
        //
        // self.unacked_packets.swap_remove()
        //
        // for (dest, frame) in self.unsent_packets.iter() {}


        return false;
    }

    pub fn process_events(&mut self, manager: &ConnectionManager) -> bool {
        // TODO resend packets, handle time outs, etc

        self.resend_packets(manager);

        if self.state == State::Established {
            return self.send_some_data(manager);
        }

        if self.state == State::CloseWait {
            self.send_some_data(manager);

            // wait longer for our data to be acknowledge before closing
            if self.remote_acked_number != self.seq_number.get() {
                return false;
            }

            let mut flags = Flags::default();
            flags.set_fin(true);
            self.send_packet(manager, flags, empty_payload());

            self.state = State::LastAck;
            return true;
        }


        false
    }

    pub fn key(&self) -> ConnectionKey {
        ConnectionKey { local: self.local, remote: self.remote }
    }
}

type ConnectionAcceptor = Box<dyn FnMut(Sink, Source) -> io::Result<()> + Send>;

struct ConnectionManagerImpl {
    pub ip: Arc<ipv4::Interface>,
    connections: Option<HashMap<ConnectionKey, TcpConnection>>,
    pub listening_ports: HashMap<Socket, ConnectionAcceptor>,
}

pub struct ConnectionManager {
    inner: Mutex<ConnectionManagerImpl>
}

impl ConnectionManager {
    pub fn new(ip: Arc<ipv4::Interface>) -> Self {
        Self {
            inner: mutex_new!(ConnectionManagerImpl {
                ip,
                connections: Some(HashMap::new()),
                listening_ports: HashMap::new(),
            })
        }
    }

    pub fn process_events(&self) -> bool {
        let mut events = false;

        let mut connections = m_lock!(self.inner).connections.take().unwrap();

        for (_, conn) in connections.iter_mut() {
            events |= conn.process_events(self);
        }

        m_lock!(self.inner).connections.replace(connections);

        events
    }

    pub fn add_listening_port(&self, socket: Socket, func: ConnectionAcceptor) {
        m_lock!(self.inner).listening_ports.insert(socket, func);
    }

    pub fn on_receive_packet(&self, ip_header: &ipv4::IPv4Header, frame: &TcpFrame) {
        let remote_sock: Socket = (ip_header.source, frame.header.source_port.get());
        let local_sock: Socket = (ip_header.destination, frame.header.destination_port.get());
        let key = ConnectionKey { local: local_sock, remote: remote_sock };

        {
            let mut lock = m_lock!(self.inner);
            if !lock.connections.as_mut().unwrap().contains_key(&key) {
                match lock.listening_ports.get_mut(&local_sock) {
                    Some(func) => {
                        let outgoing = BufferHandle::new();
                        let incoming = BufferHandle::new();

                        if let Err(e) = func(Sink::Buffer(outgoing.clone()), Source::Buffer(incoming.clone())) {
                            kprintln!("::accept() error: {:?} for socket: {:?}", e, local_sock);
                            return;
                        }

                        let conn = TcpConnection::new(
                            local_sock, remote_sock, State::Listen,
                            Sink::Buffer(incoming), Source::Buffer(outgoing));

                        lock.connections.as_mut().unwrap().insert(key.clone(), conn);
                    }
                    None => {
                        kprintln!("Dropping unknown packet to {:?}", local_sock);
                        return;
                    }
                }
            }
        }

        let mut conn = m_lock!(self.inner).connections.as_mut().unwrap().remove(&key).unwrap();
        conn.handle_packet(self, ip_header, frame);

        if conn.state != State::Killed {
            m_lock!(self.inner).connections.as_mut().unwrap().insert(key.clone(), conn);
        } else {
            kprintln!("Connection {:?} killed", key);
        }
    }

    pub fn print_info(&self) -> String {
        let mut result = String::new();

        let lock = m_lock!(self.inner);

        writeln!(result, "Listening Ports:").unwrap();

        for (socket, _) in lock.listening_ports.iter() {
            writeln!(result, "  {}:{}", socket.0, socket.1).unwrap();
        }

        writeln!(result, "\nConnections:").unwrap();

        for (_, conn) in lock.connections.as_ref().unwrap().iter() {
            writeln!(result, "  {}:{} -> {}:{} {:?} unsent:{}, unacked:{}",
                     conn.local.0, conn.local.1, conn.remote.0, conn.remote.1,
                     conn.state, conn.unsent_packets.len(), conn.unacked_packets.len()).unwrap();
        }

        result
    }
}

