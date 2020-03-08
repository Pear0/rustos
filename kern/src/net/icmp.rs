use pi::types::{BigU16, BigU32};
use crate::net::ipv4::IPv4Payload;
use crate::net::{try_parse_struct, encode_struct, ipv4, NetResult, NetErrorKind};
use alloc::boxed::Box;
use crate::console::kprintln;
use crate::net::util::ChecksumOnesComplement;

#[repr(C, packed)]
#[derive(Clone, Debug, Default)]
pub struct Header {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: BigU16,
    pub header: BigU32,
}

#[derive(Debug, Clone)]
pub struct IcmpFrame {
    pub header: Header,
    pub payload: Box<[u8]>,
}

impl IcmpFrame {
    pub fn identifier(&self) -> u16 {
        (self.header.header.get() >> 16) as u16
    }

    pub fn sequence_number(&self) -> u16 {
        (self.header.header.get() & 0xFF_FF) as u16
    }
}

impl IPv4Payload for IcmpFrame {
    const PROTOCOL_NUMBER: u8 = 1;

    fn try_parse(buf: &[u8]) -> Option<(Self, &[u8])> {

        // kprintln!("parsing ping: {} {}", buf.len(), core::mem::size_of::<Header>());
        let (header, buf) = try_parse_struct::<Header>(buf)?;
        // kprintln!("parsed ping");


        let frame = IcmpFrame {
            header,
            payload: Box::from(buf),
        };

        Some((frame, &buf[..0]))
    }

    fn encode<'a>(&self, mut buf: &'a mut [u8], _header: &ipv4::IPv4Header) -> NetResult<&'a mut [u8]> {
        let mut header = self.header.clone();
        header.checksum.set(0);

        let mut check = ChecksumOnesComplement::new();
        check.ingest_sized(&header);
        check.ingest_u8_pad(self.payload.as_ref());
        header.checksum.set(check.get());

        buf = encode_struct(buf, &header).ok_or(NetErrorKind::EncodeFail)?;

        buf[0..self.payload.len()].copy_from_slice(self.payload.as_ref());

        Ok(&mut buf[self.payload.len()..])
    }


}

