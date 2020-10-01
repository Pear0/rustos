#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

// converted from linux/drivers/net/ethernet/stmicro/stmmac/dwmac1000.h


pub const fn GENMASK(h: usize, l: usize) -> u32 {
    ((u32::max_value() - (1u32 << l) + 1) & (u32::max_value() >> (32 - 1 - h)))
}

pub const GMAC_HI_REG_AE: u32 = 0x80000000;

pub const GMAC_CONTROL: usize = 0x00000000;	/* Configuration */
pub const GMAC_FRAME_FILTER: usize = 0x00000004;	/* Frame Filter */
pub const GMAC_HASH_HIGH: usize = 0x00000008;	/* Multicast Hash Table High */
pub const GMAC_HASH_LOW: usize = 0x0000000c;	/* Multicast Hash Table Low */
pub const GMAC_MII_ADDR: usize = 0x00000010;	/* MII Address */
pub const GMAC_MII_DATA: usize = 0x00000014;	/* MII Data */
pub const GMAC_FLOW_CTRL: usize = 0x00000018;	/* Flow Control */
pub const GMAC_VLAN_TAG: usize = 0x0000001c;	/* VLAN Tag */
pub const GMAC_DEBUG: usize = 0x00000024;	/* GMAC debug register */
pub const GMAC_WAKEUP_FILTER: usize = 0x00000028;	/* Wake-up Frame Filter */

pub const GMAC_INT_STATUS: usize = 0x00000038;	/* interrupt status register */
pub const GMAC_INT_STATUS_PMT: u32 = 1 << 3;
pub const GMAC_INT_STATUS_MMCIS: u32 = 1 << 4;
pub const GMAC_INT_STATUS_MMCRIS: u32 = 1 << 5;
pub const GMAC_INT_STATUS_MMCTIS: u32 = 1 << 6;
pub const GMAC_INT_STATUS_MMCCSUM: u32 = 1 << 7;
pub const GMAC_INT_STATUS_TSTAMP: u32 = 1 << 9;
pub const GMAC_INT_STATUS_LPIIS: u32 = 1 << 10;

/* interrupt mask register */
pub const GMAC_INT_MASK: usize = 0x0000003c;
pub const GMAC_INT_DISABLE_RGMII: u32 = 1 << 0;
pub const GMAC_INT_DISABLE_PCSLINK: u32 = 1 << 1;
pub const GMAC_INT_DISABLE_PCSAN: u32 = 1 << 2;
pub const GMAC_INT_DISABLE_PMT: u32 = 1 << 3;
pub const GMAC_INT_DISABLE_TIMESTAMP: u32 = 1 << 9;
pub const GMAC_INT_DISABLE_PCS: u32 = GMAC_INT_DISABLE_RGMII |
    GMAC_INT_DISABLE_PCSLINK |
    GMAC_INT_DISABLE_PCSAN;
pub const GMAC_INT_DEFAULT_MASK: u32 = GMAC_INT_DISABLE_TIMESTAMP |
    GMAC_INT_DISABLE_PCS;


/* PMT Control and Status */
pub const GMAC_PMT: usize = 0x0000002c;
#[repr(u32)]
pub enum power_event {
    pointer_reset = 0x80000000,
    global_unicast = 0x00000200,
    wake_up_rx_frame = 0x00000040,
    magic_frame = 0x00000020,
    wake_up_frame_en = 0x00000004,
    magic_pkt_en = 0x00000002,
    power_down = 0x00000001,
}

/* Energy Efficient Ethernet (EEE)
 *
 * LPI status, timer and control register offset
 */
pub const LPI_CTRL_STATUS: usize = 0x0030;
pub const LPI_TIMER_CTRL: usize = 0x0034;

/* LPI control and status defines */
pub const LPI_CTRL_STATUS_LPITXA: u32 = 0x00080000;	/* Enable LPI TX Automate */
pub const LPI_CTRL_STATUS_PLSEN: u32 = 0x00040000;	/* Enable PHY Link Status */
pub const LPI_CTRL_STATUS_PLS: u32 = 0x00020000;	/* PHY Link Status */
pub const LPI_CTRL_STATUS_LPIEN: u32 = 0x00010000;	/* LPI Enable */
pub const LPI_CTRL_STATUS_RLPIST: u32 = 0x00000200;	/* Receive LPI state */
pub const LPI_CTRL_STATUS_TLPIST: u32 = 0x00000100;	/* Transmit LPI state */
pub const LPI_CTRL_STATUS_RLPIEX: u32 = 0x00000008;	/* Receive LPI Exit */
pub const LPI_CTRL_STATUS_RLPIEN: u32 = 0x00000004;	/* Receive LPI Entry */
pub const LPI_CTRL_STATUS_TLPIEX: u32 = 0x00000002;	/* Transmit LPI Exit */
pub const LPI_CTRL_STATUS_TLPIEN: u32 = 0x00000001;	/* Transmit LPI Entry */

/* GMAC HW ADDR regs */

pub const fn GMAC_ADDR_HIGH(reg: usize) -> usize {
    (if reg > 15 { 0x00000800 } else { 0x00000040 }) + (reg * 8)
}

pub const fn GMAC_ADDR_LOW(reg: usize) -> usize {
    (if reg > 15 { 0x00000804 } else { 0x00000044 }) + (reg * 8)
}

pub const GMAC_MAX_PERFECT_ADDRESSES: usize = 1;

pub const GMAC_PCS_BASE: usize = 0x000000c0;	/* PCS register base */
pub const GMAC_RGSMIIIS: usize = 0x000000d8;	/* RGMII/SMII status */

/* SGMII/RGMII status register */
pub const GMAC_RGSMIIIS_LNKMODE: u32 = 1 << 0;
pub const GMAC_RGSMIIIS_SPEED: u32 = GENMASK(2, 1);
pub const GMAC_RGSMIIIS_SPEED_SHIFT: usize = 1;
pub const GMAC_RGSMIIIS_LNKSTS: u32 = 1 << 3;
pub const GMAC_RGSMIIIS_JABTO: u32 = 1 << 4;
pub const GMAC_RGSMIIIS_FALSECARDET: u32 = 1 << 5;
pub const GMAC_RGSMIIIS_SMIDRXS: u32 = 1 << 16;
/* LNKMOD */
pub const GMAC_RGSMIIIS_LNKMOD_MASK: usize = 0x1;
/* LNKSPEED */
pub const GMAC_RGSMIIIS_SPEED_125: usize = 0x2;
pub const GMAC_RGSMIIIS_SPEED_25: usize = 0x1;
pub const GMAC_RGSMIIIS_SPEED_2_5: usize = 0x0;

/* GMAC Configuration defines */
pub const GMAC_CONTROL_2K: u32 = 0x08000000;	/* IEEE 802.3as 2K packets */
pub const GMAC_CONTROL_TC: u32 = 0x01000000;	/* Transmit Conf. in RGMII/SGMII */
pub const GMAC_CONTROL_WD: u32 = 0x00800000;	/* Disable Watchdog on receive */
pub const GMAC_CONTROL_JD: u32 = 0x00400000;	/* Jabber disable */
pub const GMAC_CONTROL_BE: u32 = 0x00200000;	/* Frame Burst Enable */
pub const GMAC_CONTROL_JE: u32 = 0x00100000;	/* Jumbo frame */

#[repr(u32)]
pub enum inter_frame_gap {
    GMAC_CONTROL_IFG_88 = 0x00040000,
    GMAC_CONTROL_IFG_80 = 0x00020000,
    GMAC_CONTROL_IFG_40 = 0x000e0000,
}
pub const GMAC_CONTROL_DCRS: u32 = 0x00010000;	/* Disable carrier sense */
pub const GMAC_CONTROL_PS: u32 = 0x00008000;	/* Port Select 0:GMI 1:MII */
pub const GMAC_CONTROL_FES: u32 = 0x00004000;	/* Speed 0:10 1:100 */
pub const GMAC_CONTROL_DO: u32 = 0x00002000;	/* Disable Rx Own */
pub const GMAC_CONTROL_LM: u32 = 0x00001000;	/* Loop-back mode */
pub const GMAC_CONTROL_DM: u32 = 0x00000800;	/* Duplex Mode */
pub const GMAC_CONTROL_IPC: u32 = 0x00000400;	/* Checksum Offload */
pub const GMAC_CONTROL_DR: u32 = 0x00000200;	/* Disable Retry */
pub const GMAC_CONTROL_LUD: u32 = 0x00000100;	/* Link up/down */
pub const GMAC_CONTROL_ACS: u32 = 0x00000080;	/* Auto Pad/FCS Stripping */
pub const GMAC_CONTROL_DC: u32 = 0x00000010;	/* Deferral Check */
pub const GMAC_CONTROL_TE: u32 = 0x00000008;	/* Transmitter Enable */
pub const GMAC_CONTROL_RE: u32 = 0x00000004;	/* Receiver Enable */

pub const GMAC_CORE_INIT: u32 = GMAC_CONTROL_JD | GMAC_CONTROL_PS | GMAC_CONTROL_ACS |
    GMAC_CONTROL_BE | GMAC_CONTROL_DCRS;


/* GMAC Frame Filter defines */
pub const GMAC_FRAME_FILTER_PR: u32 = 0x00000001;	/* Promiscuous Mode */
pub const GMAC_FRAME_FILTER_HUC: u32 = 0x00000002;	/* Hash Unicast */
pub const GMAC_FRAME_FILTER_HMC: u32 = 0x00000004;	/* Hash Multicast */
pub const GMAC_FRAME_FILTER_DAIF: u32 = 0x00000008;	/* DA Inverse Filtering */
pub const GMAC_FRAME_FILTER_PM: u32 = 0x00000010;	/* Pass all multicast */
pub const GMAC_FRAME_FILTER_DBF: u32 = 0x00000020;	/* Disable Broadcast frames */
pub const GMAC_FRAME_FILTER_PCF: u32 = 0x00000080;	/* Pass Control frames */
pub const GMAC_FRAME_FILTER_SAIF: u32 = 0x00000100;	/* Inverse Filtering */
pub const GMAC_FRAME_FILTER_SAF: u32 = 0x00000200;	/* Source Address Filter */
pub const GMAC_FRAME_FILTER_HPF: u32 = 0x00000400;	/* Hash or perfect Filter */
pub const GMAC_FRAME_FILTER_RA: u32 = 0x80000000;	/* Receive all mode */
/* GMII ADDR  defines */
pub const GMAC_MII_ADDR_WRITE: u32 = 0x00000002;	/* MII Write */
pub const GMAC_MII_ADDR_BUSY: u32 = 0x00000001;	/* MII Busy */
/* GMAC FLOW CTRL defines */
pub const GMAC_FLOW_CTRL_PT_MASK: u32 = 0xffff0000;	/* Pause Time Mask */
pub const GMAC_FLOW_CTRL_PT_SHIFT: usize = 16;
pub const GMAC_FLOW_CTRL_UP: u32 = 0x00000008;	/* Unicast pause frame enable */
pub const GMAC_FLOW_CTRL_RFE: u32 = 0x00000004;	/* Rx Flow Control Enable */
pub const GMAC_FLOW_CTRL_TFE: u32 = 0x00000002;	/* Tx Flow Control Enable */
pub const GMAC_FLOW_CTRL_FCB_BPA: u32 = 0x00000001;	/* Flow Control Busy ... */

/* DEBUG Register defines */
/* MTL TxStatus FIFO */
pub const GMAC_DEBUG_TXSTSFSTS: u32 = 1 << 25;	/* MTL TxStatus FIFO Full Status */
pub const GMAC_DEBUG_TXFSTS: u32 = 1 << 24; /* MTL Tx FIFO Not Empty Status */
pub const GMAC_DEBUG_TWCSTS: u32 = 1 << 22; /* MTL Tx FIFO Write Controller */
/* MTL Tx FIFO Read Controller Status */
pub const GMAC_DEBUG_TRCSTS_MASK: u32 = GENMASK(21, 20);
pub const GMAC_DEBUG_TRCSTS_SHIFT: usize = 20;
pub const GMAC_DEBUG_TRCSTS_IDLE: usize = 0;
pub const GMAC_DEBUG_TRCSTS_READ: usize = 1;
pub const GMAC_DEBUG_TRCSTS_TXW: usize = 2;
pub const GMAC_DEBUG_TRCSTS_WRITE: usize = 3;
pub const GMAC_DEBUG_TXPAUSED: u32 = 1 << 19; /* MAC Transmitter in PAUSE */
/* MAC Transmit Frame Controller Status */
pub const GMAC_DEBUG_TFCSTS_MASK: u32 = GENMASK(18, 17);
pub const GMAC_DEBUG_TFCSTS_SHIFT: usize = 17;
pub const GMAC_DEBUG_TFCSTS_IDLE: usize = 0;
pub const GMAC_DEBUG_TFCSTS_WAIT: usize = 1;
pub const GMAC_DEBUG_TFCSTS_GEN_PAUSE: usize = 2;
pub const GMAC_DEBUG_TFCSTS_XFER: usize = 3;
/* MAC GMII or MII Transmit Protocol Engine Status */
pub const GMAC_DEBUG_TPESTS: u32 = 1 << 16;
pub const GMAC_DEBUG_RXFSTS_MASK: u32 = GENMASK(9, 8); /* MTL Rx FIFO Fill-level */
pub const GMAC_DEBUG_RXFSTS_SHIFT: usize = 8;
pub const GMAC_DEBUG_RXFSTS_EMPTY: usize = 0;
pub const GMAC_DEBUG_RXFSTS_BT: usize = 1;
pub const GMAC_DEBUG_RXFSTS_AT: usize = 2;
pub const GMAC_DEBUG_RXFSTS_FULL: usize = 3;
pub const GMAC_DEBUG_RRCSTS_MASK: u32 = GENMASK(6, 5); /* MTL Rx FIFO Read Controller */
pub const GMAC_DEBUG_RRCSTS_SHIFT: usize = 5;
pub const GMAC_DEBUG_RRCSTS_IDLE: usize = 0;
pub const GMAC_DEBUG_RRCSTS_RDATA: usize = 1;
pub const GMAC_DEBUG_RRCSTS_RSTAT: usize = 2;
pub const GMAC_DEBUG_RRCSTS_FLUSH: usize = 3;
pub const GMAC_DEBUG_RWCSTS: u32 = 1 << 4; /* MTL Rx FIFO Write Controller Active */
/* MAC Receive Frame Controller FIFO Status */
pub const GMAC_DEBUG_RFCFCSTS_MASK: u32 = GENMASK(2, 1);
pub const GMAC_DEBUG_RFCFCSTS_SHIFT: usize = 1;
/* MAC GMII or MII Receive Protocol Engine Status */
pub const GMAC_DEBUG_RPESTS: u32 = 1 << 0;

/*--- DMA BLOCK defines ---*/
/* DMA Bus Mode register defines */
pub const DMA_BUS_MODE_DA: u32 = 0x00000002;	/* Arbitration scheme */
pub const DMA_BUS_MODE_DSL_MASK: u32 = 0x0000007c;	/* Descriptor Skip Length */
pub const DMA_BUS_MODE_DSL_SHIFT: usize = 2;		/*   (in DWORDS)      */
/* Programmable burst length (passed thorugh platform)*/
pub const DMA_BUS_MODE_PBL_MASK: u32 = 0x00003f00;	/* Programmable Burst Len */
pub const DMA_BUS_MODE_PBL_SHIFT: usize = 8;
pub const DMA_BUS_MODE_ATDS: u32 = 0x00000080;	/* Alternate Descriptor Size */

#[repr(u32)]
pub enum rx_tx_priority_ratio {
    double_ratio = 0x00004000,	/* 2:1 */
    triple_ratio = 0x00008000,	/* 3:1 */
    quadruple_ratio = 0x0000c000,	/* 4:1 */
}

pub const DMA_BUS_MODE_FB: u32 = 0x00010000;	/* Fixed burst */
pub const DMA_BUS_MODE_MB: u32 = 0x04000000;	/* Mixed burst */
pub const DMA_BUS_MODE_RPBL_MASK: u32 = 0x007e0000;	/* Rx-Programmable Burst Len */
pub const DMA_BUS_MODE_RPBL_SHIFT: usize = 17;
pub const DMA_BUS_MODE_USP: u32 = 0x00800000;
pub const DMA_BUS_MODE_MAXPBL: u32 = 0x01000000;
pub const DMA_BUS_MODE_AAL: u32 = 0x02000000;

/* DMA CRS Control and Status Register Mapping */
pub const DMA_HOST_TX_DESC: u32 = 0x00001048;	/* Current Host Tx descriptor */
pub const DMA_HOST_RX_DESC: u32 = 0x0000104c;	/* Current Host Rx descriptor */
/*  DMA Bus Mode register defines */
pub const DMA_BUS_PR_RATIO_MASK: u32 = 0x0000c000;	/* Rx/Tx priority ratio */
pub const DMA_BUS_PR_RATIO_SHIFT: usize = 14;
pub const DMA_BUS_FB: u32 = 0x00010000;	/* Fixed Burst */

/* DMA operation mode defines (start/stop tx/rx are placed in common header)*/
/* Disable Drop TCP/IP csum error */
pub const DMA_CONTROL_DT: u32 = 0x04000000;
pub const DMA_CONTROL_RSF: u32 = 0x02000000;	/* Receive Store and Forward */
pub const DMA_CONTROL_DFF: u32 = 0x01000000;	/* Disaable flushing */
/* Threshold for Activating the FC */
#[repr(u32)]
pub enum rfa {
    act_full_minus_1 = 0x00800000,
    act_full_minus_2 = 0x00800200,
    act_full_minus_3 = 0x00800400,
    act_full_minus_4 = 0x00800600,
}
/* Threshold for Deactivating the FC */
#[repr(u32)]
pub enum rfd {
    deac_full_minus_1 = 0x00400000,
    deac_full_minus_2 = 0x00400800,
    deac_full_minus_3 = 0x00401000,
    deac_full_minus_4 = 0x00401800,
}
pub const DMA_CONTROL_TSF: u32 = 0x00200000;	/* Transmit  Store and Forward */

#[repr(u32)]
pub enum ttc_control {
    DMA_CONTROL_TTC_64 = 0x00000000,
    DMA_CONTROL_TTC_128 = 0x00004000,
    DMA_CONTROL_TTC_192 = 0x00008000,
    DMA_CONTROL_TTC_256 = 0x0000c000,
    DMA_CONTROL_TTC_40 = 0x00010000,
    DMA_CONTROL_TTC_32 = 0x00014000,
    DMA_CONTROL_TTC_24 = 0x00018000,
    DMA_CONTROL_TTC_16 = 0x0001c000,
}
pub const DMA_CONTROL_TC_TX_MASK: u32 = 0xfffe3fff;

pub const DMA_CONTROL_EFC: u32 = 0x00000100;
pub const DMA_CONTROL_FEF: u32 = 0x00000080;
pub const DMA_CONTROL_FUF: u32 = 0x00000040;

/* Receive flow control activation field
 * RFA field in DMA control register, bits 23,10:9
 */
pub const DMA_CONTROL_RFA_MASK: u32 = 0x00800600;

/* Receive flow control deactivation field
 * RFD field in DMA control register, bits 22,12:11
 */
pub const DMA_CONTROL_RFD_MASK: u32 = 0x00401800;

/* RFD and RFA fields are encoded as follows
 *
 *   Bit Field
 *   0,00 - Full minus 1KB (only valid when rxfifo >= 4KB and EFC enabled)
 *   0,01 - Full minus 2KB (only valid when rxfifo >= 4KB and EFC enabled)
 *   0,10 - Full minus 3KB (only valid when rxfifo >= 4KB and EFC enabled)
 *   0,11 - Full minus 4KB (only valid when rxfifo > 4KB and EFC enabled)
 *   1,00 - Full minus 5KB (only valid when rxfifo > 8KB and EFC enabled)
 *   1,01 - Full minus 6KB (only valid when rxfifo > 8KB and EFC enabled)
 *   1,10 - Full minus 7KB (only valid when rxfifo > 8KB and EFC enabled)
 *   1,11 - Reserved
 *
 * RFD should always be > RFA for a given FIFO size. RFD == RFA may work,
 * but packet throughput performance may not be as expected.
 *
 * Be sure that bit 3 in GMAC Register 6 is set for Unicast Pause frame
 * detection (IEEE Specification Requirement, Annex 31B, 31B.1, Pause
 * Description).
 *
 * Be sure that DZPA (bit 7 in Flow Control Register, GMAC Register 6),
 * is set to 0. This allows pause frames with a quanta of 0 to be sent
 * as an XOFF message to the link peer.
 */

pub const RFA_FULL_MINUS_1K: u32 = 0x00000000;
pub const RFA_FULL_MINUS_2K: u32 = 0x00000200;
pub const RFA_FULL_MINUS_3K: u32 = 0x00000400;
pub const RFA_FULL_MINUS_4K: u32 = 0x00000600;
pub const RFA_FULL_MINUS_5K: u32 = 0x00800000;
pub const RFA_FULL_MINUS_6K: u32 = 0x00800200;
pub const RFA_FULL_MINUS_7K: u32 = 0x00800400;

pub const RFD_FULL_MINUS_1K: u32 = 0x00000000;
pub const RFD_FULL_MINUS_2K: u32 = 0x00000800;
pub const RFD_FULL_MINUS_3K: u32 = 0x00001000;
pub const RFD_FULL_MINUS_4K: u32 = 0x00001800;
pub const RFD_FULL_MINUS_5K: u32 = 0x00400000;
pub const RFD_FULL_MINUS_6K: u32 = 0x00400800;
pub const RFD_FULL_MINUS_7K: u32 = 0x00401000;

#[repr(u32)]
pub enum rtc_control {
    DMA_CONTROL_RTC_64 = 0x00000000,
    DMA_CONTROL_RTC_32 = 0x00000008,
    DMA_CONTROL_RTC_96 = 0x00000010,
    DMA_CONTROL_RTC_128 = 0x00000018,
}
pub const DMA_CONTROL_TC_RX_MASK: u32 = 0xffffffe7;

pub const DMA_CONTROL_OSF: u32 = 0x00000004;	/* Operate on second frame */

/* MMC registers offset */
pub const GMAC_MMC_CTRL: usize = 0x100;
pub const GMAC_MMC_RX_INTR: usize = 0x104;
pub const GMAC_MMC_TX_INTR: usize = 0x108;
pub const GMAC_MMC_RXFRMCNT_GB: usize = 0x180;
pub const GMAC_MMC_RXOCTETCNT_G: usize = 0x188;
pub const GMAC_MMC_RX_CSUM_OFFLOAD: usize = 0x208;
pub const GMAC_EXTHASH_BASE: usize = 0x500;

