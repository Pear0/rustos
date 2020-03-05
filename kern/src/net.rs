use alloc::boxed::Box;
use alloc::sync::Arc;
use core::time::Duration;

use pi::mbox::MBox;
use pi::usb::{self, Usb};

use crate::console::kprintln;
use crate::mutex::Mutex;
use crate::net::arp::{ArpPacket, ArpTable};

pub mod arp;
pub mod ether;
pub mod ipv4;

pub fn try_parse_struct<T: core::marker::Sized>(buf: &[u8]) -> Option<(T, &[u8])> where T: core::marker::Sized {
    use core::mem::size_of;
    if buf.len() < size_of::<T>() {
        return None;
    }

    // maybe there's a better way to do this...
    let mut header: T = unsafe { core::mem::zeroed() };
    {
        let header = unsafe { core::slice::from_raw_parts_mut((&mut header) as *mut T as *mut u8, size_of::<T>()) };
        header.copy_from_slice(&buf[0..size_of::<T>()]);
    }

    Some((header, &buf[size_of::<T>()..]))
}

fn get_mac_address() -> Option<ether::Mac> {
    let raw = MBox::mac_address()?;
    let raw: [u8; 8] = unsafe { core::mem::transmute(raw) };
    Some(ether::Mac::from(&raw[..6]))
}

pub struct NetHandler {
    usb: Arc<usb::Usb>,
    eth: Arc<Mutex<ether::Interface>>,
    arp: Arc<ArpTable>,
}

impl NetHandler {
    pub unsafe fn new(usb: usb::Usb) -> Option<NetHandler> {
        let usb = Arc::new(usb);

        if !usb.ethernet_available() {
            kprintln!("ethernet not available");
            return None;
        }

        while !usb.ethernet_link_up() {
            kprintln!("eth DOWN");
            pi::timer::spin_sleep(Duration::from_millis(500));
        }

        kprintln!("eth UP");

        let mac = get_mac_address()?;
        let eth = Arc::new(Mutex::new(ether::Interface::new(usb.clone(), mac)));

        let mut handler = NetHandler { usb, eth, arp: Arc::new(ArpTable::new()) };
        handler.register_arp_responder();

        Some(handler)
    }

    fn register_arp_responder(&mut self) {
        let mut eth = self.eth.lock();
        let table = self.arp.clone();

        eth.register::<ArpPacket>(Box::new(move |eth, _header, arp_req| {

            let my_ip = ipv4::Address::from(&[169, 254, 78, 130]);

            if arp_req.hw_address_space.get() != arp::HW_ADDR_ETHER
                || arp_req.protocol_address_space.get() != arp::PROT_ADDR_IP
                || arp_req.hw_address_len != 6
                || arp_req.protocol_address_len != 4
                || arp_req.op_code() != arp::ArpOp::Request {
                return;
            }

            kprintln!("Valid ARP: {:?}", arp_req);

            table.insert(0x800, arp_req.protocol_address_sender, arp_req.hw_address_sender);

            if arp_req.protocol_address_target == my_ip {
                kprintln!("responding...");

                let mut response = arp_req.clone();

                response.protocol_address_target = response.protocol_address_sender;
                response.hw_address_target = response.hw_address_sender;

                response.set_op_code(arp::ArpOp::Reply);

                response.hw_address_sender = eth.address();
                response.protocol_address_sender = my_ip;

                eth.send(response.hw_address_target, response).unwrap();

            }
        }));
    }

    pub fn dispatch(&mut self) -> bool {
        self.eth.lock().receive_dispatch().is_some()
    }

}

pub struct GlobalNetHandler(Mutex<Option<NetHandler>>);

impl GlobalNetHandler {
    pub const fn uninitialized() -> Self {
        Self(Mutex::new(None))
    }

    pub unsafe fn initialize(&self) {
        let usb = Usb::new().expect("failed to init usb");

        kprintln!("created usb");

        let net = NetHandler::new(usb).expect("create net handler");

        kprintln!("created net");

        self.0.lock().replace(net);
    }

    pub fn critical<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut NetHandler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }


}

pub fn do_stuff() {
    unsafe {
        let usb = Usb::new().expect("failed to init usb");

        let mut net = NetHandler::new(usb).expect("create net handler");

        loop {
            net.dispatch();
        }

    }
}