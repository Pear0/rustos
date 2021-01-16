use pi::types::{BigU16, BigU32};
use crate::net::ipv4::IPv4Payload;
use crate::net::{try_parse_struct, encode_struct, ipv4, NetResult, NetErrorKind};
use alloc::boxed::Box;
use crate::net::util::ChecksumOnesComplement;

#[repr(C, packed)]
#[derive(Clone, Debug, Default)]
pub struct Header {
    pub source_port: BigU16,
    pub destination_port: BigU16,
    pub length: BigU16,
    pub checksum: BigU16,
}

impl Header {
    pub fn new(source: u16, dest: u16) -> Header {
        Self {
            source_port: BigU16::new(source),
            destination_port: BigU16::new(dest),
            length: BigU16::default(),
            checksum: BigU16::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UdpFrame {
    pub header: Header,
    pub payload: Box<[u8]>,
}

impl UdpFrame {

    fn total_length(&self) -> u16 {
        8 + self.payload.len() as u16
    }
}

impl IPv4Payload for UdpFrame {
    const PROTOCOL_NUMBER: u8 = 17;

    fn try_parse(buf: &[u8]) -> Option<(Self, &[u8])> {

        // kprintln!("parsing ping: {} {}", buf.len(), core::mem::size_of::<Header>());
        let (header, buf) = try_parse_struct::<Header>(buf)?;
        // kprintln!("parsed ping");


        let frame = UdpFrame {
            header,
            payload: Box::from(buf),
        };

        Some((frame, &buf[..0]))
    }

    fn encode<'a>(&self, buf: &'a mut [u8], ip_header: &ipv4::IPv4Header) -> NetResult<&'a mut [u8]> {
        // calculate checksum with pseudo header
        let mut check = ChecksumOnesComplement::new();
        check.ingest_sized(&ip_header.source);
        check.ingest_sized(&ip_header.destination);
        check.ingest_sized(&BigU16::new(Self::PROTOCOL_NUMBER as u16));
        check.ingest_sized(&BigU16::new(self.total_length()));

        let mut udp_header = self.header.clone();

        udp_header.checksum.set(0);
        udp_header.length.set(self.total_length() as u16);

        check.ingest_sized(&udp_header);
        check.ingest_u8_pad(self.payload.as_ref());
        let check = check.get();
        // kprintln!("checksum: 0x{:04x}", check);
        udp_header.checksum.set(check);

        let buf = encode_struct(buf, &udp_header).ok_or(NetErrorKind::EncodeFail)?;

        let len = self.payload.len();
        buf[..len].copy_from_slice(self.payload.as_ref());

        Ok(&mut buf[len..])
    }

}

