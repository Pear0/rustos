#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

/* DMA CRS Control and Status Register Mapping */
pub const DMA_BUS_MODE: usize = 0x00001000;	/* Bus Mode */
pub const DMA_XMT_POLL_DEMAND: usize = 0x00001004;	/* Transmit Poll Demand */
pub const DMA_RCV_POLL_DEMAND: usize = 0x00001008;	/* Received Poll Demand */
pub const DMA_RCV_BASE_ADDR: usize = 0x0000100c;	/* Receive List Base */
pub const DMA_TX_BASE_ADDR: usize = 0x00001010;	/* Transmit List Base */
pub const DMA_STATUS: usize = 0x00001014;	/* Status Register */
pub const DMA_CONTROL: usize = 0x00001018;	/* Ctrl (Operational Mode) */
pub const DMA_INTR_ENA: usize = 0x0000101c;	/* Interrupt Enable */
pub const DMA_MISSED_FRAME_CTR: usize = 0x00001020;	/* Missed Frame Counter */

/* SW Reset */
pub const DMA_BUS_MODE_SFT_RESET: u32 = 0x00000001;	/* Software Reset */

/* Rx watchdog register */
pub const DMA_RX_WATCHDOG: usize = 0x00001024;

/* AXI Master Bus Mode */
pub const DMA_AXI_BUS_MODE: usize = 0x00001028;

pub const DMA_AXI_EN_LPI: u32 = 1 << 31;
pub const DMA_AXI_LPI_XIT_FRM: u32 = 1 << 30;
pub const DMA_AXI_WR_OSR_LMT: u32 = GENMASK(23, 20);
pub const DMA_AXI_WR_OSR_LMT_SHIFT: usize = 20;
pub const DMA_AXI_WR_OSR_LMT_MASK: usize = 0xf;
pub const DMA_AXI_RD_OSR_LMT: u32 = GENMASK(19, 16);
pub const DMA_AXI_RD_OSR_LMT_SHIFT: usize = 16;
pub const DMA_AXI_RD_OSR_LMT_MASK: usize = 0xf;

pub const DMA_AXI_OSR_MAX: usize = 0xf;
pub const DMA_AXI_MAX_OSR_LIMIT: usize = ((DMA_AXI_OSR_MAX << DMA_AXI_WR_OSR_LMT_SHIFT) |
(DMA_AXI_OSR_MAX << DMA_AXI_RD_OSR_LMT_SHIFT));
pub const DMA_AXI_1KBBE: u32 = 1 << 13;
pub const DMA_AXI_AAL: u32 = 1 << 12;
pub const DMA_AXI_BLEN256: u32 = 1 << 7;
pub const DMA_AXI_BLEN128: u32 = 1 << 6;
pub const DMA_AXI_BLEN64: u32 = 1 << 5;
pub const DMA_AXI_BLEN32: u32 = 1 << 4;
pub const DMA_AXI_BLEN16: u32 = 1 << 3;
pub const DMA_AXI_BLEN8: u32 = 1 << 2;
pub const DMA_AXI_BLEN4: u32 = 1 << 1;
pub const  DMA_BURST_LEN_DEFAULT: u32 =	(DMA_AXI_BLEN256 | DMA_AXI_BLEN128 |
DMA_AXI_BLEN64 | DMA_AXI_BLEN32 |
DMA_AXI_BLEN16 | DMA_AXI_BLEN8 |
DMA_AXI_BLEN4);

pub const DMA_AXI_UNDEF: u32 = 1 << 0;

pub const DMA_AXI_BURST_LEN_MASK: u32 = 0x000000FE;

pub const DMA_CUR_TX_BUF_ADDR: usize = 0x00001050;	/* Current Host Tx Buffer */
pub const DMA_CUR_RX_BUF_ADDR: usize = 0x00001054;	/* Current Host Rx Buffer */
pub const DMA_HW_FEATURE: usize = 0x00001058;	/* HW Feature Register */

/* DMA Control register defines */
pub const DMA_CONTROL_ST: u32 = 0x00002000;	/* Start/Stop Transmission */
pub const DMA_CONTROL_SR: u32 = 0x00000002;	/* Start/Stop Receive */

/* DMA Normal interrupt */
pub const DMA_INTR_ENA_NIE: u32 = 0x00010000;	/* Normal Summary */
pub const DMA_INTR_ENA_TIE: u32 = 0x00000001;	/* Transmit Interrupt */
pub const DMA_INTR_ENA_TUE: u32 = 0x00000004;	/* Transmit Buffer Unavailable */
pub const DMA_INTR_ENA_RIE: u32 = 0x00000040;	/* Receive Interrupt */
pub const DMA_INTR_ENA_ERE: u32 = 0x00004000;	/* Early Receive */

pub const DMA_INTR_NORMAL: u32 = (DMA_INTR_ENA_NIE | DMA_INTR_ENA_RIE |
DMA_INTR_ENA_TIE);

/* DMA Abnormal interrupt */
pub const DMA_INTR_ENA_AIE: u32 = 0x00008000;	/* Abnormal Summary */
pub const DMA_INTR_ENA_FBE: u32 = 0x00002000;	/* Fatal Bus Error */
pub const DMA_INTR_ENA_ETE: u32 = 0x00000400;	/* Early Transmit */
pub const DMA_INTR_ENA_RWE: u32 = 0x00000200;	/* Receive Watchdog */
pub const DMA_INTR_ENA_RSE: u32 = 0x00000100;	/* Receive Stopped */
pub const DMA_INTR_ENA_RUE: u32 = 0x00000080;	/* Receive Buffer Unavailable */
pub const DMA_INTR_ENA_UNE: u32 = 0x00000020;	/* Tx Underflow */
pub const DMA_INTR_ENA_OVE: u32 = 0x00000010;	/* Receive Overflow */
pub const DMA_INTR_ENA_TJE: u32 = 0x00000008;	/* Transmit Jabber */
pub const DMA_INTR_ENA_TSE: u32 = 0x00000002;	/* Transmit Stopped */

pub const DMA_INTR_ABNORMAL: u32 = (DMA_INTR_ENA_AIE | DMA_INTR_ENA_FBE |
DMA_INTR_ENA_UNE);

/* DMA default interrupt mask */
pub const DMA_INTR_DEFAULT_MASK: u32 = (DMA_INTR_NORMAL | DMA_INTR_ABNORMAL);
pub const DMA_INTR_DEFAULT_RX: u32 = 	(DMA_INTR_ENA_RIE);
pub const DMA_INTR_DEFAULT_TX: u32 = 	(DMA_INTR_ENA_TIE);

/* DMA Status register defines */
pub const DMA_STATUS_GLPII: u32 = 0x40000000;	/* GMAC LPI interrupt */
pub const DMA_STATUS_GPI: u32 = 0x10000000;	/* PMT interrupt */
pub const DMA_STATUS_GMI: u32 = 0x08000000;	/* MMC interrupt */
pub const DMA_STATUS_GLI: u32 = 0x04000000;	/* GMAC Line interface int */
pub const DMA_STATUS_EB_MASK: u32 = 0x00380000;	/* Error Bits Mask */
pub const DMA_STATUS_EB_TX_ABORT: u32 = 0x00080000;	/* Error Bits - TX Abort */
pub const DMA_STATUS_EB_RX_ABORT: u32 = 0x00100000;	/* Error Bits - RX Abort */
pub const DMA_STATUS_TS_MASK: u32 = 0x00700000;	/* Transmit Process State */
pub const DMA_STATUS_TS_SHIFT: usize = 20;
pub const DMA_STATUS_RS_MASK: u32 = 0x000e0000;	/* Receive Process State */
pub const DMA_STATUS_RS_SHIFT: usize = 17;
pub const DMA_STATUS_NIS: u32 = 0x00010000;	/* Normal Interrupt Summary */
pub const DMA_STATUS_AIS: u32 = 0x00008000;	/* Abnormal Interrupt Summary */
pub const DMA_STATUS_ERI: u32 = 0x00004000;	/* Early Receive Interrupt */
pub const DMA_STATUS_FBI: u32 = 0x00002000;	/* Fatal Bus Error Interrupt */
pub const DMA_STATUS_ETI: u32 = 0x00000400;	/* Early Transmit Interrupt */
pub const DMA_STATUS_RWT: u32 = 0x00000200;	/* Receive Watchdog Timeout */
pub const DMA_STATUS_RPS: u32 = 0x00000100;	/* Receive Process Stopped */
pub const DMA_STATUS_RU: u32 = 0x00000080;	/* Receive Buffer Unavailable */
pub const DMA_STATUS_RI: u32 = 0x00000040;	/* Receive Interrupt */
pub const DMA_STATUS_UNF: u32 = 0x00000020;	/* Transmit Underflow */
pub const DMA_STATUS_OVF: u32 = 0x00000010;	/* Receive Overflow */
pub const DMA_STATUS_TJT: u32 = 0x00000008;	/* Transmit Jabber Timeout */
pub const DMA_STATUS_TU: u32 = 0x00000004;	/* Transmit Buffer Unavailable */
pub const DMA_STATUS_TPS: u32 = 0x00000002;	/* Transmit Process Stopped */
pub const DMA_STATUS_TI: u32 = 0x00000001;	/* Transmit Interrupt */
pub const DMA_CONTROL_FTF: u32 = 0x00100000;	/* Flush transmit FIFO */

pub const NUM_DWMAC100_DMA_REGS: usize = 9;
pub const NUM_DWMAC1000_DMA_REGS: usize = 23;


const fn GENMASK(h: usize, l: usize) -> u32 {
    ((u32::max_value() - (1u32 << l) + 1) & (u32::max_value() >> (32 - 1 - h)))
}