use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt;

use hashbrown::HashMap;

use pi::types::BigU16;
use pi::usb;
use pi::usb::Usb;

use crate::console::kprintln;
use crate::net::{encode_struct, try_parse_struct, NetResult, NetErrorKind};

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mac([u8; 6]);

impl Mac {
    pub fn broadcast() -> Mac {
        Mac([0xFF; 6])
    }
}

impl From<&[u8]> for Mac {
    fn from(buf: &[u8]) -> Self {
        assert_eq!(buf.len(), 6);
        let mut mac = Mac([0; 6]);
        mac.0.copy_from_slice(buf);
        mac
    }
}

impl From<&[u8; 6]> for Mac {
    fn from(buf: &[u8; 6]) -> Self {
        assert_eq!(buf.len(), 6);
        let mut mac = Mac([0; 6]);
        mac.0.copy_from_slice(buf);
        mac
    }
}

impl fmt::Display for Mac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                                 self.0[0], self.0[1], self.0[2],
                                 self.0[3], self.0[4], self.0[5]))
    }
}

pub trait EthPayload: Sized {
    const ETHER_TYPE: u16;

    fn try_parse(buf: &[u8]) -> Option<(Self, &[u8])> {
        try_parse_struct::<Self>(buf)
    }

    fn encode<'a>(&self, buf: &'a mut [u8]) -> NetResult<&'a mut [u8]> {
        encode_struct::<Self>(buf, self).ok_or(NetErrorKind::EncodeFail)
    }
}

#[repr(C, packed)]
pub struct EthHeader {
    pub mac_receiver: Mac,
    pub mac_sender: Mac,
    pub protocol_type: BigU16,
}

#[repr(C, packed)]
pub struct EthFrame<T: EthPayload> {
    pub eth: EthHeader,
    pub payload: T,
}

pub type EthHandler<T> = Box<dyn FnMut(&mut Interface, &EthHeader, &mut T, &[u8]) + Send>;

type RawEthHandler = Box<dyn FnMut(&mut Interface, &EthHeader, &[u8]) + Send>;

pub struct Interface {
    usb: Arc<Usb>,
    handlers: HashMap<u16, Option<RawEthHandler>>,
    address: Mac,
}

impl Interface {
    pub fn new(usb: Arc<Usb>, address: Mac) -> Interface {
        Interface {
            usb,
            handlers: HashMap::new(),
            address,
        }
    }

    pub fn address(&self) -> Mac {
        self.address
    }

    pub fn send<T: EthPayload>(&mut self, to: Mac, payload: T) -> NetResult<()> {
        let mut full_buf = usb::new_frame_buffer();
        let full_buf_len = full_buf.len();
        let mut buf: &mut [u8] = &mut full_buf;

        let header = EthHeader {
            mac_sender: self.address,
            mac_receiver: to,
            protocol_type: BigU16::new(T::ETHER_TYPE),
        };

        buf = encode_struct(buf, &header).ok_or(NetErrorKind::EncodeFail)?;
        buf = payload.encode(buf)?;

        // buf is our write pointer, we need to turn it into the valid buffer.
        let buf_len = full_buf_len - buf.len();
        let buf = &mut full_buf[0..buf_len];

        unsafe { self.usb.send_frame(buf) }.ok_or(NetErrorKind::EthSendFail)
    }

    pub fn register<T: EthPayload + 'static>(&mut self, mut handler: EthHandler<T>) {
        self.handlers.insert(T::ETHER_TYPE, Some(Box::new(move |eth, header, buf| {
            match T::try_parse(buf) {
                Some((mut t, rest)) => {
                    handler(eth, header, &mut t, rest);
                }
                None => kprintln!("malformed packet for protocol: 0x{:x} has size {} < {}",
                    header.protocol_type.get(), buf.len(), core::mem::size_of::<T>()),
            };
        })));
    }

    pub fn receive_dispatch(&mut self) -> Option<()> {
        let mut frame_buf = usb::new_frame_buffer();
        let frame = unsafe { self.usb.receive_frame(&mut frame_buf) }?;

        let (eth, frame) = try_parse_struct::<EthHeader>(frame)?;
        let protocol = eth.protocol_type.get();

        // kprintln!("[eth] frame: protocol=0x{:x}", protocol);

        // we pass a mutable reference to self but also have a FnMut so we have to move
        // the FnMut out of &mut self while this happens.
        if self.handlers.contains_key(&protocol) {
            let handler = self.handlers.insert(protocol, None).unwrap();
            let mut handler = handler.expect("recursion???");

            handler(self, &eth, frame);

            if let Some(None) = self.handlers.get(&protocol) {
                self.handlers.insert(protocol, Some(handler));
            }
        }

        Some(())
    }
}


