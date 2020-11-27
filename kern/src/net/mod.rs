use alloc::boxed::Box;
use alloc::sync::Arc;
use core::time::Duration;

use pi::mbox::MBox;
use pi::usb::{self, Usb};

use crate::mutex::Mutex;
use crate::net::arp::{ArpPacket, ArpTable, ArpResolver};
use crate::mbox::with_mbox;
use crate::net::icmp::IcmpFrame;
use crate::net::ipv4::IPv4Payload;
use core::ops::DerefMut;
use core::ops::Deref;
use shim::{io, newioerr};
use crate::net::physical::{VirtNIC, Physical};
use crate::{BootVariant, hw};
use crate::net::udp::UdpFrame;

pub mod arp;
pub mod buffer;
pub mod ether;
pub mod icmp;
pub mod ipv4;
pub mod physical;
pub mod tcp;
pub mod udp;
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

    pub fn into_io_err(self) -> io::Error {
        match self {
            NetErrorKind::ArpMiss => newioerr!(AddrNotAvailable, "ArpMiss"),
            NetErrorKind::IpPacketTooLarge => newioerr!(InvalidInput, "IpPacketTooLarge"),
            NetErrorKind::EthSendFail => newioerr!(Interrupted, "EthSendFail"),
            NetErrorKind::EncodeFail => newioerr!(InvalidInput, "EncodeFail"),
            NetErrorKind::BufferFull => newioerr!(WouldBlock, "BufferFull"),
        }
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
    pub usb: Arc<dyn Physical>,
    pub eth: Arc<ether::Interface>,
    pub arp: Arc<ArpTable>,
    pub ip: Arc<ipv4::Interface>,
    pub tcp: Arc<tcp::ConnectionManager>,
}

impl NetHandler {
    pub fn new(usb: Arc<dyn Physical>) -> Option<NetHandler> {

        // while !usb.ethernet_available() {
        //     info!("ethernet not available");
        //     pi::timer::spin_sleep(Duration::from_millis(2000));
        // }
        //
        // while !usb.ethernet_link_up() {
        //     debug!("eth DOWN");
        //     pi::timer::spin_sleep(Duration::from_millis(500));
        // }
        //
        // debug!("eth UP");

        let mac = ether::Mac::from(&[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);// get_mac_address()?;
        let my_ip = ipv4::Address::from(&[10, 45, 52, 130]);

        let eth = Arc::new(ether::Interface::new(usb.clone(), mac));

        let arp = Arc::new(ArpTable::new());

        let ip = Arc::new(ipv4::Interface::new(eth.clone(), my_ip, arp.clone()));

        let tcp = Arc::new(tcp::ConnectionManager::new(ip.clone()));

        let mut handler = NetHandler { usb, eth, arp, ip, tcp };
        // Eth
        handler.register_arp_responder();
        handler.register_ipv4_responder();

        // IPv4
        handler.register_ping_responder();


        // {
        //     handler.tcp.add_listening_port((my_ip, 100), Box::new(|sink, source| {
        //        
        //        
        //        
        //     }));
        // }

        // gratuitous ARP so neighbors know about us sooner.
        {
            let mut eth = &handler.eth;

            let mut packet = ArpPacket::default();
            packet.hw_address_space.set(arp::HW_ADDR_ETHER);
            packet.protocol_address_space.set(arp::PROT_ADDR_IP);
            packet.hw_address_len = 6;
            packet.protocol_address_len = 4;
            packet.set_op_code(arp::ArpOp::Reply);

            packet.hw_address_sender = eth.address();
            packet.protocol_address_sender = my_ip;

            if let Err(e) = eth.send(ether::Mac::broadcast(), packet) {
                debug!("gratuitous arp failure: {:?}", e);
            }
        }

        Some(handler)
    }

    fn register_arp_responder(&mut self) {
        let table = self.arp.clone();

        self.eth.register::<ArpPacket>(Box::new(move |eth, _header, arp_req, _| {

            let my_ip = ipv4::Address::from(&[10, 45, 52, 130]);

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
        let ip = self.ip.clone();

        self.eth.register::<ipv4::IPv4Frame>(Box::new(move |eth, header, req, buf| {
            ip.receive_dispatch(header, req);
        }));
    }

    fn register_ping_responder(&mut self) {
        let ip = self.ip.clone();

        self.ip.register::<icmp::IcmpFrame>(Box::new(move |eth, eth_header, ip_header, icmp| {

            // kprintln!("icmp: {:?}", icmp);
            // kprintln!("id: {}, seq: {}", icmp.identifier(), icmp.sequence_number());

            if icmp.header.icmp_type == 8 {
                let mut copy = icmp.clone();

                copy.header.icmp_type = 0; // ping reply

                ip.send(ip_header.source, &copy);
            }

        }));

        let tcp = self.tcp.clone();

        self.ip.register::<tcp::TcpFrame>(Box::new(move |eth, eth_header, ip_header, frame| {
            tcp.on_receive_packet(ip_header, frame);
        }));

    }

    pub fn arp_request(&self, addr: ipv4::Address) -> NetResult<ether::Mac> {
        let me = self.ip.address();

        self.arp.resolve_or_request_address(arp::PROT_ADDR_IP, addr, me, self.eth.clone())
    }

    pub fn send_datagram(&self, addr: ipv4::Address, src_port: u16, dst_port: u16, payload: &[u8]) -> NetResult<()> {
        let udp = UdpFrame {
            header: udp::Header::new(src_port, dst_port),
            payload: Box::from(payload),
        };

        self.ip.send(addr, &udp)
    }

    pub fn dispatch(&self) -> bool {
        let mut events = false;
        events |= self.eth.receive_dispatch().is_some();
        events |= self.tcp.process_events();

        events
    }

}

pub struct GlobalNetHandler(Mutex<Option<NetHandler>>);

impl GlobalNetHandler {
    pub const fn uninitialized() -> Self {
        Self(mutex_new!(None))
    }

    pub fn initialize_with(&self, usb: Arc<dyn Physical>) {
        let net = NetHandler::new(usb).expect("create net handler");

        info!("created net");

        m_lock!(self.0).replace(net);
    }

    pub unsafe fn initialize(&self) {
        // let usb = Usb::new().expect("failed to init usb");

        let phys: Arc<dyn Physical>;
        if BootVariant::kernel_in_hypervisor() {
            phys = Arc::new(VirtNIC());
        } else if BootVariant::kernel() {
            if hw::not_pi() || hw::is_qemu() {
                info!("choose dwmac net");

                let mac = crate::driver::net::dwmac::DwMac1000::open().expect("failed to initialize dwmac");
                phys = Arc::new(mac);
            } else if hw::is_qemu() {
                phys = Arc::new(physical::NilDevice());
            } else {
                let usb = unsafe { Usb::new() }.expect("failed to initialize usb");
                phys = Arc::new(physical::PhysicalUsb(usb));
            }
        } else {
            panic!("todo, net stack in hypervisor");
        }

        info!("created nic");

        self.initialize_with(phys);
    }

    pub fn is_initialized(&self) -> bool {
        m_lock!(self.0).is_some()
    }

    pub fn critical<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut NetHandler) -> R,
    {
        let mut guard = m_lock!(self.0);
        f(guard.as_mut().expect("net uninitialized"))
    }


}
