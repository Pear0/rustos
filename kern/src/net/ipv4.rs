use alloc::boxed::Box;
use core::fmt;
use crate::mutex::m_lock;

use pi::types::BigU16;
use shim::const_assert_size;

use crate::console::kprintln;
use crate::net::{encode_struct, try_parse_struct, ether, arp, NetResult, NetErrorKind};
use crate::net::ether::{EthPayload, EthHeader};
use crate::net::util::ChecksumOnesComplement;
use alloc::sync::Arc;
use hashbrown::HashMap;
use crate::mutex::Mutex;
use core::str::FromStr;
use core::num::ParseIntError;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Address([u8; 4]);

impl From<&[u8]> for Address {
    fn from(buf: &[u8]) -> Self {
        assert_eq!(buf.len(), 4);
        let mut addr = Address([0; 4]);
        addr.0.copy_from_slice(buf);
        addr
    }
}

impl From<&[u8; 4]> for Address {
    fn from(buf: &[u8; 4]) -> Self {
        assert_eq!(buf.len(), 4);
        let mut addr = Address([0; 4]);
        addr.0.copy_from_slice(buf);
        addr
    }
}

impl FromStr for Address {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut addr = Address::default();
        if s.split('.').count() != 4 {
            return Err("wrong number of octets")
        }
        for (i, e) in s.split('.').into_iter().enumerate() {
            addr.0[i] = e.parse().or(Err("invalid octet"))?;
        }
        Ok(addr)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}.{}.{}.{}",
                                 self.0[0], self.0[1], self.0[2], self.0[3]))
    }
}

#[repr(C, packed)]
#[derive(Clone, Debug, Default)]
struct IPv4MinHeader {
    header: BigU16,
    total_length: BigU16,
    identification: BigU16,
    fragment: BigU16,
    ttl_protocol: BigU16,
    header_checksum: BigU16,
    source: Address,
    destination: Address,
}

const_assert_size!(IPv4MinHeader, 20);

#[derive(Clone, Debug)]
pub struct IPv4Header {
    pub version: u8,
    pub ihl: u8,
    pub dscp: u8,
    pub ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags: u8,
    pub fragment_offset: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub header_checksum: u16,
    pub source: Address,
    pub destination: Address,
}

impl IPv4Header {
    pub fn new(protocol: u8, from: Address, to: Address, payload_len: u16) -> Self {
        Self {
            version: 4,
            ihl: 5,
            dscp: 0,
            ecn: 0,
            total_length: payload_len + 20,
            identification: 0,
            flags: 0,
            fragment_offset: 0,
            ttl: 255,
            protocol,
            header_checksum: 0,
            source: from,
            destination: to,
        }
    }

    pub fn payload_len(&self) -> u16 {
        self.total_length - 4 * self.ihl as u16
    }

    fn min_header(&self) -> IPv4MinHeader {
        let mut h = IPv4MinHeader::default();
        h.header.or_mask(w(self.version as u16, 15, 12));
        h.header.or_mask(w(self.ihl as u16, 11, 8));
        h.header.or_mask(w(self.dscp as u16, 7, 2));
        h.header.or_mask(w(self.ecn as u16, 1, 0));
        h.total_length.set(self.total_length);
        h.identification.set(self.identification);
        h.fragment.or_mask(w(self.flags as u16, 15, 13));
        h.fragment.or_mask(w(self.fragment_offset as u16, 12, 0));
        h.ttl_protocol.or_mask(w(self.ttl as u16, 15, 8));
        h.ttl_protocol.or_mask(w(self.protocol as u16, 7, 0));

        h.source = self.source;
        h.destination = self.destination;

        h.header_checksum.set(0);

        let checksum = ChecksumOnesComplement::digest_sized(&h);
        h.header_checksum.set(checksum);

        h
    }
}

// num[high:low] bit extraction inclusive both sides
fn w(num: u16, high: usize, low: usize) -> u16 {
    ((num) & ((2u16 << (high - low)) - 1)) << low
}


// num[high:low] bit extraction inclusive both sides
fn r(num: u16, high: usize, low: usize) -> u16 {
    (num >> low) & ((2u16 << (high - low)) - 1)
}

impl From<IPv4MinHeader> for IPv4Header {
    fn from(min: IPv4MinHeader) -> Self {
        Self {
            version: r(min.header.get(), 15, 12) as u8,
            ihl: r(min.header.get(), 11, 8) as u8,
            dscp: r(min.header.get(), 7, 2) as u8,
            ecn: r(min.header.get(), 1, 0) as u8,
            total_length: min.total_length.get(),
            identification: min.identification.get(),
            flags: r(min.fragment.get(), 15, 13) as u8,
            fragment_offset: r(min.fragment.get(), 12, 0),
            ttl: r(min.ttl_protocol.get(), 15, 8) as u8,
            protocol: r(min.ttl_protocol.get(), 7, 0) as u8,
            header_checksum: min.header_checksum.get(),
            source: min.source,
            destination: min.destination,
        }
    }
}

pub trait IPv4Payload: Sized {
    const PROTOCOL_NUMBER: u8;

    fn try_parse(buf: &[u8]) -> Option<(Self, &[u8])> {
        try_parse_struct::<Self>(buf)
    }

    fn encode<'a>(&self, buf: &'a mut [u8], header: &IPv4Header) -> NetResult<&'a mut [u8]> {
        encode_struct::<Self>(buf, self).ok_or(NetErrorKind::EncodeFail)
    }
}


#[derive(Clone, Debug)]
pub struct IPv4Frame {
    pub header: IPv4Header,
    pub payload: Box<[u8]>,
}

impl EthPayload for IPv4Frame {
    const ETHER_TYPE: u16 = 0x800;

    fn try_parse(buf: &[u8]) -> Option<(Self, &[u8])> {
        let (head, mut buf) = try_parse_struct::<IPv4MinHeader>(buf)?;
        let head = IPv4Header::from(head);

        if head.ihl < 5 {
            return None;
        }

        // handle variable sized ipv4 header. we ignore options for now
        if head.ihl > 5 {
            buf = &(&buf)[4 * (head.ihl - 5) as usize..];
        }

        buf = &buf[..head.payload_len() as usize];

        let pay = IPv4Frame {
            header: head,
            payload: Box::from(buf),
        };

        Some((pay, &buf[..0]))
    }

    fn encode<'a>(&self, mut buf: &'a mut [u8]) -> NetResult<&'a mut [u8]> {
        let mut header = self.header.clone();
        header.version = 4;
        header.ihl = 5;
        header.ttl = 255;
        header.total_length = self.payload.len() as u16 + 20;

        buf = encode_struct(buf, &header.min_header()).ok_or(NetErrorKind::EncodeFail)?;

        buf[0..self.payload.len()].copy_from_slice(self.payload.as_ref());

        Ok(&mut buf[self.payload.len()..])
    }
}


pub type IpHandler<T> = Box<dyn FnMut(&mut Interface, &EthHeader, &IPv4Header, &mut T) + Send>;

type RawIpHandler = Box<dyn FnMut(&mut Interface, &EthHeader, &IPv4Header, &[u8]) + Send>;

pub struct Interface {
    eth: Arc<Mutex<ether::Interface>>,
    handlers: HashMap<u8, Option<RawIpHandler>>,
    address: Address,
    arp_resolver: Arc<dyn arp::ArpResolver>,


}

impl Interface {
    pub fn new(eth: Arc<Mutex<ether::Interface>>, address: Address, arp: Arc<dyn arp::ArpResolver>) -> Interface {
        Interface {
            eth,
            handlers: HashMap::new(),
            address,
            arp_resolver: arp,
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn maximum_packet_size(&self) -> usize {
        576 - 20 // maximum size minus 20 byte ip header. we never send larger ip headers
    }

    pub fn send<T: IPv4Payload>(&mut self, to: Address, payload: T) -> NetResult<()> {
        // this is significantly oversized to catch packet issues in a friendlier way.
        // we do not support IP fragmentation.
        let mut payload_buffer = [0u8; 1200];
        let full_buffer_len = payload_buffer.len();

        let mac = self.arp_resolver.resolve_or_request_address(arp::PROT_ADDR_IP, to, self.address(), self.eth.clone())?;

        let header = IPv4Header::new(T::PROTOCOL_NUMBER, self.address(), to, 0);

        let buf_left = payload.encode(&mut payload_buffer, &header)?;
        let remaining = buf_left.len();
        let mut payload_buffer = &mut payload_buffer[0..(full_buffer_len - remaining)];

        if payload_buffer.len() > self.maximum_packet_size() {
            kprintln!("Refusing to send protocol {} packet with size {}", T::PROTOCOL_NUMBER, payload_buffer.len());
            return Err(NetErrorKind::IpPacketTooLarge);
        }

        let frame = IPv4Frame {
            header: IPv4Header::new(T::PROTOCOL_NUMBER, self.address(), to, payload_buffer.len() as u16),
            payload: Box::from(payload_buffer.as_ref()),
        };

        let mut lock = m_lock!(self.eth);
        lock.send(mac, frame)
    }

    pub fn register<T: IPv4Payload + 'static>(&mut self, mut handler: IpHandler<T>) {
        self.handlers.insert(T::PROTOCOL_NUMBER, Some(Box::new(move |eth, eth_header, ip_header, buf| {
            match T::try_parse(buf) {
                Some((mut t, rest)) => {
                    if rest.len() != 0 {
                        kprintln!("packet protocol 0x{:x} left {} bytes unconsumed", T::PROTOCOL_NUMBER, rest.len());
                    }
                    handler(eth, eth_header, ip_header, &mut t);
                }
                None => kprintln!("malformed packet for protocol: 0x{:x} has size {} < {}",
                    T::PROTOCOL_NUMBER, buf.len(), core::mem::size_of::<T>()),
            };
        })));
    }

    pub fn receive_dispatch(&mut self, eth_header: &ether::EthHeader, frame: &IPv4Frame) -> Option<()> {
        let protocol = frame.header.protocol;

        // kprintln!("[ip] frame: protocol=0x{:x}", protocol);

        // we pass a mutable reference to self but also have a FnMut so we have to move
        // the FnMut out of &mut self while this happens.
        if self.handlers.contains_key(&protocol) {
            let handler = self.handlers.insert(protocol, None).unwrap();
            let mut handler = handler.expect("recursion???");

            handler(self, eth_header, &frame.header, frame.payload.as_ref());

            if let Some(None) = self.handlers.get(&protocol) {
                self.handlers.insert(protocol, Some(handler));
            }
        }

        Some(())
    }
}




