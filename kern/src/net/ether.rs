use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt;

use hashbrown::HashMap;

use pi::types::BigU16;
use pi::usb;
use pi::usb::Usb;

use crate::net::{encode_struct, try_parse_struct, NetResult, NetErrorKind};
use crate::mutex::Mutex;
use crate::net::physical::{Physical, Frame};

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mac([u8; 6]);

impl Mac {
    pub fn broadcast() -> Mac {
        Mac([0xFF; 6])
    }

    pub fn is_broadcast(&self) -> bool {
        *self == Self::broadcast()
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

pub type EthHandler<T> = Box<dyn FnMut(&Interface, &EthHeader, &mut T, &[u8]) + Send>;

type RawEthHandler = Box<dyn FnMut(&Interface, &EthHeader, &[u8]) + Send>;

struct InterfaceImpl {
    physical: Arc<dyn Physical>,
    handlers: HashMap<u16, Option<RawEthHandler>>,
    address: Mac,
}

pub struct Interface {
    inner: Mutex<InterfaceImpl>
}

impl Interface {
    pub fn new(physical: Arc<dyn Physical>, address: Mac) -> Interface {
        Interface {
            inner: Mutex::new(InterfaceImpl {
                physical,
                handlers: HashMap::new(),
                address,
            })
        }
    }

    pub fn address(&self) -> Mac {
        m_lock!(self.inner).address
    }

    pub fn send<T: EthPayload>(&self, to: Mac, payload: T) -> NetResult<()> {
        let mut frame = Frame::default();
        let full_buf_len = frame.0.len();
        let mut buf: &mut [u8] = &mut frame.0;

        let header = EthHeader {
            mac_sender: self.address(),
            mac_receiver: to,
            protocol_type: BigU16::new(T::ETHER_TYPE),
        };

        buf = encode_struct(buf, &header).ok_or(NetErrorKind::EncodeFail)?;
        buf = payload.encode(buf)?;

        // buf is our write pointer, we need to turn it into the valid buffer.
        frame.1 = full_buf_len - buf.len();

        m_lock!(self.inner).physical.send_frame(&frame).ok_or(NetErrorKind::EthSendFail)
    }

    pub fn register<T: EthPayload + 'static>(&self, mut handler: EthHandler<T>) {
        m_lock!(self.inner).handlers.insert(T::ETHER_TYPE, Some(Box::new(move |eth, header, buf| {
            match T::try_parse(buf) {
                Some((mut t, rest)) => {
                    handler(eth, header, &mut t, rest);
                }
                None => kprintln!("malformed packet for protocol: 0x{:x} has size {} < {}",
                    header.protocol_type.get(), buf.len(), core::mem::size_of::<T>()),
            };
        })));
    }

    pub fn receive_dispatch(&self) -> Option<()> {
        let mut frame = Frame::default();
        unsafe { m_lock!(self.inner).physical.receive_frame(&mut frame) }?;

        let (eth, frame) = try_parse_struct::<EthHeader>(frame.as_slice())?;
        let protocol = eth.protocol_type.get();

        // kprintln!("[eth] frame: protocol=0x{:x}", protocol);

        // we pass a mutable reference to self but also have a FnMut so we have to move
        // the FnMut out of &mut self while this happens.

        // this code side-effects by inserting a None into the handlers for unknown protocols
        let handler = m_lock!(self.inner).handlers.insert(protocol, None)?;
        let mut handler = handler?;

        handler(self, &eth, frame);

        if let Some(f) = m_lock!(self.inner).handlers.get_mut(&protocol) {
            if f.is_none() {
                f.replace(handler);
            }
        }

        Some(())
    }
}


