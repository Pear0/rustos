use alloc::boxed::Box;
use alloc::sync::Arc;
use core::time::Duration;

use pi::mbox::MBox;
use pi::usb::{self, Usb};

use crate::console::kprintln;
use crate::mutex::Mutex;
use crate::net::arp::{ArpPacket, ArpTable, ArpResolver};
use crate::mbox::with_mbox;
use crate::net::icmp::IcmpFrame;
use crate::net::ipv4::IPv4Payload;
use core::ops::DerefMut;
use core::ops::Deref;

pub mod arp;
pub mod buffer;
pub mod ether;
pub mod icmp;
pub mod ipv4;
pub mod tcp;
pub mod util;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NetErrorKind {
    ArpMiss,
    IpPacketTooLarge,
    EthSendFail,
    EncodeFail,
    BufferFull,
}

impl NetErrorKind {
    pub fn is_spurious(&self) -> bool {
        use NetErrorKind::*;
        [ArpMiss, EthSendFail, BufferFull].contains(self)
    }
}

pub type NetResult<T> = Result<T, NetErrorKind>;

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

pub fn encode_struct<'a, T>(buf: &'a mut [u8], value: &T) -> Option<&'a mut [u8]> where T: core::marker::Sized {
    use core::mem::size_of;

    use fat32::util::SliceExt;
    let response = core::slice::from_ref(value);
    let buf2: &[u8] = unsafe { response.cast() };
    assert_eq!(buf2.len(), size_of::<T>());
    assert!(buf.len() >= buf2.len());

    buf[0..buf2.len()].copy_from_slice(buf2);
    Some(&mut buf[buf2.len()..])
}

fn get_mac_address() -> Option<ether::Mac> {
    let raw = with_mbox(|mbox| mbox.mac_address())?;
    let raw: [u8; 8] = unsafe { core::mem::transmute(raw) };
    Some(ether::Mac::from(&raw[..6]))
}

pub struct NetHandler {
    pub usb: Arc<usb::Usb>,
    pub eth: Arc<Mutex<ether::Interface>>,
    pub arp: Arc<ArpTable>,
    pub ip: Arc<Mutex<ipv4::Interface>>,
    pub tcp: Arc<Mutex<tcp::ConnectionManager>>,
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
        let my_ip = ipv4::Address::from(&[169, 254, 78, 130]);

        let eth = Arc::new(Mutex::new(ether::Interface::new(usb.clone(), mac)));

        let arp = Arc::new(ArpTable::new());

        let ip = Arc::new(Mutex::new(ipv4::Interface::new(eth.clone(), my_ip, arp.clone())));

        let tcp = Arc::new(Mutex::new(tcp::ConnectionManager::new(ip.clone())));

        let mut handler = NetHandler { usb, eth, arp, ip, tcp };
        // Eth
        handler.register_arp_responder();
        handler.register_ipv4_responder();

        // IPv4
        handler.register_ping_responder();


        {
            let mut tcp = handler.tcp.lock();
            tcp.listening_ports.insert((my_ip, 100));
        }

        // gratuitous ARP so neighbors know about us sooner.
        {
            let mut eth = handler.eth.lock();

            let mut packet = ArpPacket::default();
            packet.hw_address_space.set(arp::HW_ADDR_ETHER);
            packet.protocol_address_space.set(arp::PROT_ADDR_IP);
            packet.hw_address_len = 6;
            packet.protocol_address_len = 4;
            packet.set_op_code(arp::ArpOp::Reply);

            packet.hw_address_sender = eth.address();
            packet.protocol_address_sender = my_ip;

            if let Err(e) = eth.send(ether::Mac::broadcast(), packet) {
                kprintln!("gratuitous arp failure: {:?}", e);
            }
        }

        Some(handler)
    }

    fn register_arp_responder(&mut self) {
        let mut eth = self.eth.lock();
        let table = self.arp.clone();

        eth.register::<ArpPacket>(Box::new(move |eth, _header, arp_req, _| {

            let my_ip = ipv4::Address::from(&[169, 254, 78, 130]);

            if arp_req.hw_address_space.get() != arp::HW_ADDR_ETHER
                || arp_req.protocol_address_space.get() != arp::PROT_ADDR_IP
                || arp_req.hw_address_len != 6
                || arp_req.protocol_address_len != 4 {
                return;
            }

            // kprintln!("Valid ARP: {:?}", arp_req);

            if arp_req.protocol_address_sender != ipv4::Address::from(&[0; 4]) {
                table.insert(arp::PROT_ADDR_IP, arp_req.protocol_address_sender, arp_req.hw_address_sender);
            }

            if arp_req.op_code() != arp::ArpOp::Request {
                return;
            }

            if arp_req.protocol_address_target == my_ip {
                // kprintln!("responding...");

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

    fn register_ipv4_responder(&mut self) {
        let mut eth = self.eth.lock();
        let ip = self.ip.clone();

        eth.register::<ipv4::IPv4Frame>(Box::new(move |eth, header, req, buf| {
            let mut ip = ip.lock();
            ip.receive_dispatch(header, req);
        }));
    }

    fn register_ping_responder(&mut self) {
        let mut _ip = self.ip.lock();
        let ip = self.ip.clone();

        _ip.register::<icmp::IcmpFrame>(Box::new(move |eth, eth_header, ip_header, icmp| {

            // kprintln!("icmp: {:?}", icmp);
            // kprintln!("id: {}, seq: {}", icmp.identifier(), icmp.sequence_number());

            if icmp.header.icmp_type == 8 {
                let mut copy = icmp.clone();

                copy.header.icmp_type = 0; // ping reply

                let mut ip = ip.lock();
                ip.send(ip_header.source, copy);
            }

        }));

        let ip = self.ip.clone();
        let tcp = self.tcp.clone();

        _ip.register::<tcp::TcpFrame>(Box::new(move |eth, eth_header, ip_header, frame| {

            // kprintln!("tcp: {:?}", frame);
            // frame.dump();

            let mut tcp = tcp.lock();
            tcp.on_receive_packet(ip_header, frame);

            //
            // let mut ip = ip.lock();
            //
            // let mut tcp = tcp.clone();
            //
            // core::mem::swap(&mut tcp.header.source_port, &mut tcp.header.destination_port);
            //
            // tcp.header.flags.set_ack(true);
            // tcp.header.ack_number.set(tcp.header.sequence_number.get() + 1);
            // tcp.header.sequence_number.set(0);
            //
            // ip.send(ip_header.source, tcp);
            //


            // kprintln!("id: {}, seq: {}", icmp.identifier(), icmp.sequence_number());

        }));

    }

    pub fn arp_request(&mut self, addr: ipv4::Address) -> NetResult<ether::Mac> {
        let me = self.ip.lock().address();

        self.arp.resolve_or_request_address(arp::PROT_ADDR_IP, addr, me, self.eth.clone())
    }

    pub fn dispatch(&mut self) -> bool {
        let mut events = false;
        events |= self.eth.lock().receive_dispatch().is_some();
        events |= self.tcp.lock().process_events();

        events
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

    pub fn is_initialized(&self) -> bool {
        self.0.lock().is_some()
    }

    pub fn critical<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut NetHandler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("net uninitialized"))
    }


}
