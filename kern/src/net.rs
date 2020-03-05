pub mod arp;
pub mod ether;
pub mod ipv4;

use pi::usb::{self, Usb, FrameBuffer};
use core::time::Duration;
use pi::types::BigU16;
use crate::console::kprintln;
use shim::const_assert_size;
use crate::net::arp::ArpPacket;
use crate::net::ether::{Mac, EthPayload};
use pi::mbox::MBox;
use alloc::boxed::Box;

const MAC_ADDRESS_SIZE: usize = 6;
const IP_ADDRESS_SIZE: usize = 4;

#[repr(C, packed)]
struct ArpFrame {
    eth: ether::EthHeader,
    arp: ArpPacket,
}

const header_size: usize = core::mem::size_of::<ether::EthHeader>();

pub fn try_parse_struct<T: core::marker::Sized>(buf: &[u8]) -> Option<(T, &[u8])> where T: core::marker::Sized {
    use core::mem::size_of;
    if buf.len() < size_of::<T>() {
        return None;
    }

    // maybe there's a better way to do this...
    let mut header: T = unsafe { core::mem::zeroed() };
    {
        let mut header = unsafe { core::slice::from_raw_parts_mut((&mut header) as *mut T as *mut u8, size_of::<T>()) };
        header.copy_from_slice(&buf[0..size_of::<T>()]);
    }

    Some((header, &buf[size_of::<T>()..]))
}

fn get_mac_address() -> Option<ether::Mac> {
    let raw = MBox::mac_address()?;
    let raw: [u8; 8] = unsafe { core::mem::transmute(raw) };
    Some(ether::Mac::from(&raw[..6]))
}

pub fn do_stuff() {
    unsafe {
        let usb = Usb::new().expect("failed to init usb");

        if !usb.ethernet_available() {
            kprintln!("ethernet not available");
            return;
        }

        while !usb.ethernet_link_up() {
            kprintln!("eth DOWN");
            pi::timer::spin_sleep(Duration::from_millis(500));
        }

        kprintln!("eth UP");

        let my_ip = ipv4::Address::from(&[169, 254, 78, 130]);
        let my_mac = get_mac_address().expect("failed to get mac address");

        let mut e = ether::Interface::new(&usb, get_mac_address().unwrap());

        e.register::<ArpPacket>(Box::new(|eth, header, arp_req| {

            if arp_req.hw_address_space.get() != arp::HW_ADDR_ETHER
                || arp_req.protocol_address_space.get() != arp::PROT_ADDR_IP
                || arp_req.hw_address_len != 6
                || arp_req.protocol_address_len != 4
                || arp_req.op_code() != arp::ArpOp::Request {
                return;
            }

            kprintln!("Valid ARP: {:?}", arp_req);

            if arp_req.protocol_address_target == my_ip {
                kprintln!("responding...");

                let mut response = arp_req.clone();

                response.protocol_address_target = response.protocol_address_sender;
                response.hw_address_target = response.hw_address_sender;

                response.set_op_code(arp::ArpOp::Reply);

                response.hw_address_sender = my_mac;
                response.protocol_address_sender = my_ip;

                eth.send(response.hw_address_target, response).unwrap();

            }

        }));

        loop {
            e.receive_dispatch();

        }
    }
}