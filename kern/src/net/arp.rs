use pi::types::BigU16;
use crate::net::ether::EthPayload;
use shim::const_assert_size;
use crate::net::{ether, ipv4};

const MAC_ADDRESS_SIZE: usize = 6;
const IP_ADDRESS_SIZE: usize = 4;

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
    pub hw_address_space: BigU16, // ether = 1
    pub protocol_address_space: BigU16, // ipv4 = 0x800
    pub hw_address_len: u8, // ether = 6
    pub protocol_address_len: u8, // ipv4 = 4
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

