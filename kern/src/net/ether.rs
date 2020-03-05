use core::fmt;
use pi::types::BigU16;
use pi::usb::Usb;
use alloc::boxed::Box;
use crate::console::kprintln;

use hashbrown::HashMap;
use crate::net::try_parse_struct;
use pi::usb;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Mac([u8; 6]);

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


pub trait EthPayload {
    const ETHER_TYPE: u16;
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

pub type EthHandler<T: EthPayload> = Box<dyn FnMut(&mut Interface, &EthHeader, &mut T) + Send>;

type RawEthHandler = Box<dyn FnMut(&mut Interface, &EthHeader, &[u8]) + Send>;

pub struct Interface<'a> {
    usb: &'a Usb,
    handlers: HashMap<u16, Option<RawEthHandler>>,
    address: Mac,
}

impl Interface<'_> {
    pub fn new(usb: &Usb, address: Mac) -> Interface {
        Interface {
            usb,
            handlers: HashMap::new(),
            address,
        }
    }

    pub fn send<T: EthPayload>(&mut self, to: Mac, payload: T) -> Option<()> {
        let response = EthFrame {
            eth: EthHeader {
                mac_sender: self.address,
                mac_receiver: to,
                protocol_type: BigU16::new(T::ETHER_TYPE),
            },
            payload,
        };

        use fat32::util::SliceExt;
        let response = [response];
        let buf: &[u8] = unsafe { response.cast() };

        unsafe { self.usb.send_frame(buf) }
    }

    pub fn register<T: EthPayload + 'static>(&mut self, mut handler: EthHandler<T>) {
        self.handlers.insert(T::ETHER_TYPE, Some(Box::new(move |eth, header, buf| {
            match try_parse_struct::<T>(buf) {
                Some((mut t, _)) => {
                      handler(eth, header, &mut t);
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

        kprintln!("[eth] frame: protocol=0x{:x}", protocol);

        // we pass a mutable reference to self but also have a FnMut so we have to move
        // the FnMut out of &mut self while this happens.
        if self.handlers.contains_key(&protocol) {
            let mut handler = self.handlers.insert(protocol, None).unwrap();
            let mut handler = handler.expect("recursion???");

            handler(self, &eth, frame);

            if let Some(None) = self.handlers.get(&protocol) {
                self.handlers.insert(protocol, Some(handler));
            }
        }

        Some(())
    }


}


