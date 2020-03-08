use alloc::sync::Arc;
use core::time::Duration;

use hashbrown::HashMap;

use pi::timer;
use pi::types::BigU16;
use shim::const_assert_size;

use crate::mutex::Mutex;
use crate::net::{ether, ipv4, NetErrorKind, NetResult};
use crate::net::ether::{EthPayload, Interface, Mac};
use crate::net::ipv4::Address;

pub const HW_ADDR_ETHER: u16 = 1;
pub const PROT_ADDR_IP: u16 = 0x800;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ArpOp {
    Request,
    Reply,
    Other(u16),
}

impl From<ArpOp> for u16 {
    fn from(num: ArpOp) -> Self {
        match num {
            ArpOp::Request => 1,
            ArpOp::Reply => 2,
            ArpOp::Other(e) => e,
        }
    }
}

impl From<u16> for ArpOp {
    fn from(num: u16) -> Self {
        match num {
            1 => ArpOp::Request,
            2 => ArpOp::Reply,
            e => ArpOp::Other(e),
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Debug, Default)]
pub struct ArpPacket {
    pub hw_address_space: BigU16,
    // ether = 1
    pub protocol_address_space: BigU16,
    // ipv4 = 0x800
    pub hw_address_len: u8,
    // ether = 6
    pub protocol_address_len: u8,
    // ipv4 = 4
    op_code: BigU16,

    pub hw_address_sender: ether::Mac,
    pub protocol_address_sender: ipv4::Address,
    pub hw_address_target: ether::Mac,
    pub protocol_address_target: ipv4::Address,
}

const_assert_size!(ArpPacket, 28);

impl ArpPacket {
    pub fn op_code(&self) -> ArpOp {
        self.op_code.get().into()
    }

    pub fn set_op_code(&mut self, op: ArpOp) {
        self.op_code.set(op.into())
    }
}

impl EthPayload for ArpPacket {
    const ETHER_TYPE: u16 = 0x806;
}

pub trait ArpResolver: Send + Sync {
    fn resolve_address(&self, protocol: u16, addr: ipv4::Address) -> NetResult<ether::Mac>;

    fn resolve_or_request_address(&self, protocol: u16, addr: ipv4::Address, my_addr: ipv4::Address, eth: Arc<Mutex<ether::Interface>>) -> NetResult<ether::Mac>;
}

struct ReqInfo {
    first_sent_at: Option<Duration>,
    send_count: u32,
}

/// Thread safe ARP table
pub struct ArpTable {
    table: Mutex<HashMap<(u16, ipv4::Address), ether::Mac>>,
    pending_requests: Mutex<HashMap<(u16, ipv4::Address), ReqInfo>>,
}

impl ArpTable {
    pub fn new() -> Self {
        ArpTable {
            table: Mutex::new(HashMap::new()),
            pending_requests: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, protocol: u16, ip: ipv4::Address, mac: ether::Mac) {
        {
            let mut lock = self.table.lock();
            lock.insert((protocol, ip), mac);
        }

        {
            let mut lock = self.pending_requests.lock();
            lock.remove(&(protocol, ip));
        }
    }

    pub fn get(&self, protocol: u16, ip: ipv4::Address) -> Option<ether::Mac> {
        let lock = self.table.lock();
        lock.get(&(protocol, ip)).map(|x| x.clone())
    }

    pub fn copy_table(&self) -> HashMap<(u16, ipv4::Address), ether::Mac> {
        let lock = self.table.lock();
        lock.clone()
    }
}

impl ArpResolver for ArpTable {
    fn resolve_address(&self, protocol: u16, addr: ipv4::Address) -> NetResult<ether::Mac> {
        self.get(protocol, addr).ok_or(NetErrorKind::ArpMiss)
    }

    fn resolve_or_request_address(&self, protocol: u16, addr: Address, my_addr: ipv4::Address, eth: Arc<Mutex<Interface>>) -> NetResult<Mac> {
        if let Ok(mac) = self.resolve_address(protocol, addr) {
            return Ok(mac);
        }

        {
            let mut requests = self.pending_requests.lock();
            if !requests.contains_key(&(protocol, addr)) {
                requests.insert((protocol, addr), ReqInfo { first_sent_at: None, send_count: 0 });
            }
        }

        {
            let mut eth = eth.lock();

            let mut packet = ArpPacket::default();
            packet.hw_address_space.set(HW_ADDR_ETHER);
            packet.protocol_address_space.set(PROT_ADDR_IP);
            packet.hw_address_len = 6;
            packet.protocol_address_len = 4;
            packet.op_code.set(ArpOp::Request.into());

            packet.hw_address_sender = eth.address();
            packet.protocol_address_sender = my_addr;

            packet.protocol_address_target = addr;

            eth.send(Mac::broadcast(), packet)?;
        }

        {
            let mut requests = self.pending_requests.lock();
            // we released the lock, so a remote ARP may have filled our table entry.
            if let Some(req) = requests.get_mut(&(protocol, addr)) {
                if req.first_sent_at.is_none() {
                    req.first_sent_at.replace(timer::current_time());
                }
                req.send_count += 1;
            }
        }

        self.resolve_address(protocol, addr)
    }
}

