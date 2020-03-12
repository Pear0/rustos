use alloc::boxed::Box;
use alloc::sync::Arc;
use core::slice::from_mut;
use crate::mutex::m_lock;

use hashbrown::{HashMap, HashSet};
use modular_bitfield::prelude::*;

use pi::types::{BigU16, BigU32};

use crate::console::{kprint, kprintln};
use crate::io::{ConsoleSync, SyncRead, SyncWrite, Global, ReadWrapper};
use crate::mutex::Mutex;
use crate::net::{encode_struct, ipv4, NetErrorKind, NetResult, try_parse_struct};
use crate::net::ipv4::IPv4Payload;
use crate::net::util::ChecksumOnesComplement;
use crate::shell;
use crate::process::Process;
use crate::net::buffer::BufferHandle;

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

    Broken,
    // custom lol
    Killed, // signal to the manager to remove this connection.
}

fn empty_payload() -> Box<[u8]> {
    let empty: [u8; 0] = [];
    let empty: &[u8] = &empty;
    Box::from(empty)
}

struct TcpConnection {
    pub local: Socket,
    pub remote: Socket,
    state: State,
    seq_number: u32,
    acked_number: u32, // the next sequence number we expect from remote

    recv: Arc<dyn SyncWrite>,
    send: Arc<dyn SyncRead>,

}

pub static SHELL_READ: Global<BufferHandle> = Global::new(|| BufferHandle::new());
pub static SHELL_WRITE: Global<BufferHandle> = Global::new(|| BufferHandle::new());

impl TcpConnection {
    pub fn new(local: Socket, remote: Socket, state: State) -> Self {
        Self {
            local,
            remote,
            state,
            seq_number: 0,
            acked_number: 0,
            recv: Arc::new(SHELL_READ.get()),
            send: Arc::new(SHELL_WRITE.get()),
        }
    }

    fn send_packet(&mut self, manager: &ConnectionManager, flags: Flags, payload: Box<[u8]>) -> NetResult<()> {
        // we set most fields, header is provided for weird flags
        let dest = self.remote.0.clone();

        let mut header = Header::default();
        header.flags = flags.clone();

        header.destination_port.set(self.remote.1);
        header.source_port.set(self.local.1);
        header.ack_number.set(self.acked_number);
        header.sequence_number.set(self.seq_number);

        header.window_size.set(0xFF_FF);

        let payload_len = payload.len();
        let frame = TcpFrame { header, payload };

        // resending? lol as if
        {
            let mut ip = m_lock!(manager.ip);
            ip.send(dest, frame)?;
        }

        // we sent successfully
        if flags.get_syn() {
            self.seq_number = self.seq_number.wrapping_add(1);
        } else {
            self.seq_number = self.seq_number.wrapping_add(payload_len as u32);
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

    fn handle_packet(&mut self, manager: &mut ConnectionManager, ip_header: &ipv4::IPv4Header, frame: &TcpFrame) {
        // kprintln!("TCP State: {:?}", self.state);
        match self.state {
            State::Listen => {
                // TODO we assume this is a SYN lol

                self.acked_number = frame.header.sequence_number.get().wrapping_add(1);
                self.state = State::S_SynReceived;

                let mut flags = Flags::default();
                flags.set_syn(true);
                flags.set_ack(true);

                self.send_packet(manager, flags, empty_payload());
            }
            State::C_SynSent => {}
            State::S_SynReceived => {
                if frame.header.sequence_number.get() != self.acked_number {
                    kprintln!("Got packet with seq mismatch: {} != {}", frame.header.sequence_number.get(), self.acked_number);
                    return;
                }

                // self.acked_number += 1;

                if frame.header.flags.get_ack() {
                    self.state = State::Established;
                }
            }
            State::Established => {

                // TODO not a proper connection close.
                if frame.header.flags.get_rst() || frame.header.flags.get_fin() {
                    kprintln!("got RST or FIN");
                    self.state = State::Killed;

                    let mut flags = Flags::default();
                    flags.set_ack(true);
                    flags.set_fin(true);
                    self.send_packet(manager, flags, empty_payload());

                    return;
                }

                if frame.header.sequence_number.get() != self.acked_number {
                    kprintln!("Got packet with seq mismatch: {} != {}", frame.header.sequence_number.get(), self.acked_number);
                    return;
                }

                // TODO drop packets from send queue covered byt this ack

                // TODO packets arriving out of order. an handling dropped ACKs we've sent.

                if frame.payload.len() != 0 {
                    // Ack the bytes.
                    self.acked_number = self.acked_number.wrapping_add(frame.payload.len() as u32);
                    let mut flags = Flags::default();
                    flags.set_ack(true);
                    self.send_packet(manager, flags, empty_payload());

                    // kprintln!("got packet with data: {:?}", frame.payload.as_ref());

                    // self.raw_send_bytes(manager, frame.payload.as_ref());

                    use crate::io::SyncWrite;
                    match self.recv.write(frame.payload.as_ref()) {
                        Err(e) => {
                            kprintln!("failed to write but acked: {}", e);
                        }
                        Ok(n) if n != frame.payload.len() => {
                            kprintln!("failed to write but acked {} bytes", frame.payload.len() - n);
                        }
                        _ => {}
                    };

                }
            }
            State::Closed | State::Broken | State::Killed => {}
        }
    }

    pub fn process_events(&mut self, manager: &mut ConnectionManager) -> bool {
        // TODO resend packets, handle time outs, etc

        if self.state == State::Established {
            use crate::io::SyncRead;
            let mut buf = [0u8; 500];

            match self.send.read(&mut buf) {
                Ok(n) if n > 0 => {
                    self.raw_send_bytes(manager, &buf[0..n]);
                    true
                }
                Err(e) => {
                    kprintln!("failed to read source: {}", e);
                    false
                }
                _ => {
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn key(&self) -> ConnectionKey {
        ConnectionKey { local: self.local, remote: self.remote }
    }
}

pub struct ConnectionManager {
    pub ip: Arc<Mutex<ipv4::Interface>>,
    connections: Option<HashMap<ConnectionKey, TcpConnection>>,
    pub listening_ports: HashSet<Socket>,
}

impl ConnectionManager {
    pub fn new(ip: Arc<Mutex<ipv4::Interface>>) -> Self {
        Self {
            ip,
            connections: Some(HashMap::new()),
            listening_ports: HashSet::new(),
        }
    }

    pub fn process_events(&mut self) -> bool {
        let mut events = false;

        let mut connections = self.connections.take().unwrap();

        for (_, conn) in connections.iter_mut() {
            events |= conn.process_events(self);
        }

        self.connections.replace(connections);

        events
    }

    pub fn on_receive_packet(&mut self, ip_header: &ipv4::IPv4Header, frame: &TcpFrame) {
        let remote_sock: Socket = (ip_header.source, frame.header.source_port.get());
        let local_sock: Socket = (ip_header.destination, frame.header.destination_port.get());
        let key = ConnectionKey { local: local_sock, remote: remote_sock };

        if !self.connections.as_mut().unwrap().contains_key(&key) {
            if !self.listening_ports.contains(&local_sock) {
                kprintln!("Dropping unknown packet to {:?}", local_sock);
                return;
            }

            self.connections.as_mut().unwrap().insert(key.clone(), TcpConnection::new(local_sock, remote_sock, State::Listen));
        }

        let mut conn = self.connections.as_mut().unwrap().remove(&key).unwrap();
        conn.handle_packet(self, ip_header, frame);

        if conn.state != State::Killed {
            self.connections.as_mut().unwrap().insert(key.clone(), conn);
        } else {
            kprintln!("Connection {:?} killed", key);
        }
    }
}

