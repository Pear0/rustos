
use crate::dwmac_h::*;

const MII_BUSY: u32 = 0x00000001;
const MII_WRITE: u32 = 0x00000002;
const MII_DATA_MASK: u32 = GENMASK(15, 0);

pub struct MiiRegs {
    addr: u32,
    data: u32,
    addr_shift: u32,
    reg_shift: u32,
    addr_mask: u32,
    reg_mask: u32,
    clk_csr_shift: u32,
    clk_csr_mask: u32,
}

pub static MY_MII: MiiRegs = MiiRegs {
    addr: GMAC_MII_ADDR as u32,
    data: GMAC_MII_DATA as u32,
    addr_shift: 11,
    addr_mask: 0x0000F800,
    reg_shift: 6,
    reg_mask: 0x000007C0,
    clk_csr_shift: 2,
    clk_csr_mask: GENMASK(5, 2),
};


pub fn mdio_write(io_base: usize, mii_hw: &MiiRegs, phy_addr: u32, phy_reg: u32, phy_data: u16) {
    let mut value = MII_BUSY | MII_WRITE;
    let clk_csr = 0;

    value |= (phy_addr << mii_hw.addr_shift) & mii_hw.addr_mask;
    value |= (phy_reg << mii_hw.reg_shift) & mii_hw.reg_mask;
    value |= (clk_csr << mii_hw.clk_csr_shift) & mii_hw.clk_csr_mask;

    crate::read_u32_poll(io_base + mii_hw.addr as usize, None, |v| (v & MII_BUSY) == 0);

    crate::write_u32(io_base + mii_hw.data as usize, phy_data as u32);
    crate::write_u32(io_base + mii_hw.addr as usize, value);

    crate::read_u32_poll(io_base + mii_hw.addr as usize, None, |v| (v & MII_BUSY) == 0);
}

pub fn mdio_read(io_base: usize, mii_hw: &MiiRegs, phy_addr: u32, phy_reg: u32) -> u16 {
    let mut value = MII_BUSY;
    let clk_csr = 0;

    value |= (phy_addr << mii_hw.addr_shift) & mii_hw.addr_mask;
    value |= (phy_reg << mii_hw.reg_shift) & mii_hw.reg_mask;
    value |= (clk_csr << mii_hw.clk_csr_shift) & mii_hw.clk_csr_mask;

    crate::read_u32_poll(io_base + mii_hw.addr as usize, None, |v| (v & MII_BUSY) == 0);

    crate::write_u32(io_base + mii_hw.data as usize, 0);
    crate::write_u32(io_base + mii_hw.addr as usize, value);

    crate::read_u32_poll(io_base + mii_hw.addr as usize, None, |v| (v & MII_BUSY) == 0);

    (crate::read_u32(io_base + mii_hw.data as usize) & MII_DATA_MASK) as u16
}
