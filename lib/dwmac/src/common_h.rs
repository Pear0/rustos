#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

/* Synopsys Core versions */
pub const DWMAC_CORE_3_40: usize = 0x34;
pub const DWMAC_CORE_3_50: usize = 0x35;
pub const DWMAC_CORE_4_00: usize = 0x40;
pub const DWMAC_CORE_4_10: usize = 0x41;
pub const DWMAC_CORE_5_00: usize = 0x50;
pub const DWMAC_CORE_5_10: usize = 0x51;
pub const DWXGMAC_CORE_2_10: usize = 0x21;
pub const DWXLGMAC_CORE_2_00: usize = 0x20;

/* Device ID */
pub const DWXGMAC_ID: usize = 0x76;
pub const DWXLGMAC_ID: usize = 0x27;

pub const STMMAC_CHAN0: usize = 0;	/* Always supported and default for all chips */

/* These need to be power of two, and >= 4 */
pub const DMA_TX_SIZE: usize = 512;
pub const DMA_RX_SIZE: usize = 512;

pub fn STMMAC_GET_ENTRY(x: u32, size: u32) -> u32 {
    ((x + 1) & (size - 1))
}

/* CSR Frequency Access Defines*/
pub const CSR_F_35M: usize = 35000000;
pub const CSR_F_60M: usize = 60000000;
pub const CSR_F_100M: usize = 100000000;
pub const CSR_F_150M: usize = 150000000;
pub const CSR_F_250M: usize = 250000000;
pub const CSR_F_300M: usize = 300000000;

pub const MAC_CSR_H_FRQ_MASK: usize = 0x20;

pub const HASH_TABLE_SIZE: usize = 64;
pub const PAUSE_TIME: usize = 0xffff;

/* Flow Control defines */
pub const FLOW_OFF: usize = 0;
pub const FLOW_RX: usize = 1;
pub const FLOW_TX: usize = 2;
pub const FLOW_AUTO: usize = FLOW_TX | FLOW_RX;

/* PCS defines */
pub const STMMAC_PCS_RGMII: u32 =	(1 << 0);
pub const STMMAC_PCS_SGMII: u32 =	(1 << 1);
pub const STMMAC_PCS_TBI: u32 =		(1 << 2);
pub const STMMAC_PCS_RTBI: u32 =	(1 << 3);

pub const SF_DMA_MODE: usize = 1;		/* DMA STORE-AND-FORWARD Operation Mode */

/* DAM HW feature register fields */
pub const DMA_HW_FEAT_MIISEL: u32 = 0x00000001;	/* 10/100 Mbps Support */
pub const DMA_HW_FEAT_GMIISEL: u32 = 0x00000002;	/* 1000 Mbps Support */
pub const DMA_HW_FEAT_HDSEL: u32 = 0x00000004;	/* Half-Duplex Support */
pub const DMA_HW_FEAT_EXTHASHEN: u32 = 0x00000008;	/* Expanded DA Hash Filter */
pub const DMA_HW_FEAT_HASHSEL: u32 = 0x00000010;	/* HASH Filter */
pub const DMA_HW_FEAT_ADDMAC: u32 = 0x00000020;	/* Multiple MAC Addr Reg */
pub const DMA_HW_FEAT_PCSSEL: u32 = 0x00000040;	/* PCS registers */
pub const DMA_HW_FEAT_L3L4FLTREN: u32 = 0x00000080;	/* Layer 3 & Layer 4 Feature */
pub const DMA_HW_FEAT_SMASEL: u32 = 0x00000100;	/* SMA(MDIO) Interface */
pub const DMA_HW_FEAT_RWKSEL: u32 = 0x00000200;	/* PMT Remote Wakeup */
pub const DMA_HW_FEAT_MGKSEL: u32 = 0x00000400;	/* PMT Magic Packet */
pub const DMA_HW_FEAT_MMCSEL: u32 = 0x00000800;	/* RMON Module */
pub const DMA_HW_FEAT_TSVER1SEL: u32 = 0x00001000;	/* Only IEEE 1588-2002 */
pub const DMA_HW_FEAT_TSVER2SEL: u32 = 0x00002000;	/* IEEE 1588-2008 PTPv2 */
pub const DMA_HW_FEAT_EEESEL: u32 = 0x00004000;	/* Energy Efficient Ethernet */
pub const DMA_HW_FEAT_AVSEL: u32 = 0x00008000;	/* AV Feature */
pub const DMA_HW_FEAT_TXCOESEL: u32 = 0x00010000;	/* Checksum Offload in Tx */
pub const DMA_HW_FEAT_RXTYP1COE: u32 = 0x00020000;	/* IP COE (Type 1) in Rx */
pub const DMA_HW_FEAT_RXTYP2COE: u32 = 0x00040000;	/* IP COE (Type 2) in Rx */
pub const DMA_HW_FEAT_RXFIFOSIZE: u32 = 0x00080000;	/* Rx FIFO > 2048 Bytes */
pub const DMA_HW_FEAT_RXCHCNT: u32 = 0x00300000;	/* No. additional Rx Channels */
pub const DMA_HW_FEAT_TXCHCNT: u32 = 0x00c00000;	/* No. additional Tx Channels */
pub const DMA_HW_FEAT_ENHDESSEL: u32 = 0x01000000;	/* Alternate Descriptor */
/* Timestamping with Internal System Time */
pub const DMA_HW_FEAT_INTTSEN: u32 = 0x02000000;
pub const DMA_HW_FEAT_FLEXIPPSEN: u32 = 0x04000000;	/* Flexible PPS Output */
pub const DMA_HW_FEAT_SAVLANINS: u32 = 0x08000000;	/* Source Addr or VLAN */
pub const DMA_HW_FEAT_ACTPHYIF: u32 = 0x70000000;	/* Active/selected PHY iface */
pub const DEFAULT_DMA_PBL: usize = 8;

/* PCS status and mask defines */
pub const PCS_ANE_IRQ: u32 = 1 << 2;	/* PCS Auto-Negotiation */
pub const PCS_LINK_IRQ: u32 = 1 << 1;	/* PCS Link */
pub const PCS_RGSMIIIS_IRQ: u32 = 1 << 0;	/* RGMII or SMII Interrupt */

/* Max/Min RI Watchdog Timer count value */
pub const MAX_DMA_RIWT: usize = 0xff;
pub const MIN_DMA_RIWT: usize = 0x10;
pub const DEF_DMA_RIWT: usize = 0xa0;
/* Tx coalesce parameters */
pub const STMMAC_COAL_TX_TIMER: usize = 1000;
pub const STMMAC_MAX_COAL_TX_TICK: usize = 100000;
pub const STMMAC_TX_MAX_FRAMES: usize = 256;
pub const STMMAC_TX_FRAMES: usize = 25;
pub const STMMAC_RX_FRAMES: usize = 0;

/* Packets types */
enum packets_types {
    PACKET_AVCPQ = 0x1, /* AV Untagged Control packets */
    PACKET_PTPQ = 0x2, /* PTP Packets */
    PACKET_DCBCPQ = 0x3, /* DCB Control Packets */
    PACKET_UPQ = 0x4, /* Untagged Packets */
    PACKET_MCBCQ = 0x5, /* Multicast & Broadcast Packets */
}

/* Rx IPC status */
enum rx_frame_status {
    good_frame = 0x0,
    discard_frame = 0x1,
    csum_none = 0x2,
    llc_snap = 0x4,
    dma_own = 0x8,
    rx_not_ls = 0x10,
}

/* Tx status */
enum tx_frame_status {
    tx_done = 0x0,
    tx_not_ls = 0x1,
    tx_err = 0x2,
    tx_dma_own = 0x4,
}

enum dma_irq_status {
    tx_hard_error = 0x1,
    tx_hard_error_bump_tc = 0x2,
    handle_rx = 0x4,
    handle_tx = 0x8,
}

/* EEE and LPI defines */
// #define	CORE_IRQ_TX_PATH_IN_LPI_MODE	(1 << 0)
// #define	CORE_IRQ_TX_PATH_EXIT_LPI_MODE	(1 << 1)
// #define	CORE_IRQ_RX_PATH_IN_LPI_MODE	(1 << 2)
// #define	CORE_IRQ_RX_PATH_EXIT_LPI_MODE	(1 << 3)

pub const CORE_IRQ_MTL_RX_OVERFLOW: u32 = 1 << 8;

/* Physical Coding Sublayer */
// struct rgmii_adv {
//     unsigned int pause;
//     unsigned int duplex;
//     unsigned int lp_pause;
//     unsigned int lp_duplex;
// };

pub const STMMAC_PCS_PAUSE: usize = 1;
pub const STMMAC_PCS_ASYM_PAUSE: usize = 2;

/* DMA HW capabilities */
// struct dma_features {
//     unsigned int mbps_10_100;
//     unsigned int mbps_1000;
//     unsigned int half_duplex;
//     unsigned int hash_filter;
//     unsigned int multi_addr;
//     unsigned int pcs;
//     unsigned int sma_mdio;
//     unsigned int pmt_remote_wake_up;
//     unsigned int pmt_magic_frame;
//     unsigned int rmon;
//     /* IEEE 1588-2002 */
//     unsigned int time_stamp;
//     /* IEEE 1588-2008 */
//     unsigned int atime_stamp;
//     /* 802.3az - Energy-Efficient Ethernet (EEE) */
//     unsigned int eee;
//     unsigned int av;
//     unsigned int hash_tb_sz;
//     unsigned int tsoen;
//     /* TX and RX csum */
//     unsigned int tx_coe;
//     unsigned int rx_coe;
//     unsigned int rx_coe_type1;
//     unsigned int rx_coe_type2;
//     unsigned int rxfifo_over_2048;
//     /* TX and RX number of channels */
//     unsigned int number_rx_channel;
//     unsigned int number_tx_channel;
//     /* TX and RX number of queues */
//     unsigned int number_rx_queues;
//     unsigned int number_tx_queues;
//     /* PPS output */
//     unsigned int pps_out_num;
//     /* Alternate (enhanced) DESC mode */
//     unsigned int enh_desc;
//     /* TX and RX FIFO sizes */
//     unsigned int tx_fifo_size;
//     unsigned int rx_fifo_size;
//     /* Automotive Safety Package */
//     unsigned int asp;
//     /* RX Parser */
//     unsigned int frpsel;
//     unsigned int frpbs;
//     unsigned int frpes;
//     unsigned int addr64;
//     unsigned int rssen;
//     unsigned int vlhash;
//     unsigned int sphen;
//     unsigned int vlins;
//     unsigned int dvlan;
//     unsigned int l3l4fnum;
//     unsigned int arpoffsel;
//     /* TSN Features */
//     unsigned int estwid;
//     unsigned int estdep;
//     unsigned int estsel;
//     unsigned int fpesel;
//     unsigned int tbssel;
// };

/* RX Buffer size must be multiple of 4/8/16 bytes */
pub const BUF_SIZE_16KiB: usize = 16368;
pub const BUF_SIZE_8KiB: usize = 8188;
pub const BUF_SIZE_4KiB: usize = 4096;
pub const BUF_SIZE_2KiB: usize = 2048;

/* Power Down and WOL */
pub const PMT_NOT_SUPPORTED: usize = 0;
pub const PMT_SUPPORTED: usize = 1;

/* Common MAC defines */
pub const MAC_CTRL_REG: u32 = 0x00000000;	/* MAC Control */
pub const MAC_ENABLE_TX: u32 = 0x00000008;	/* Transmitter Enable */
pub const MAC_ENABLE_RX: u32 = 0x00000004;	/* Receiver Enable */

/* Default LPI timers */
pub const STMMAC_DEFAULT_LIT_LS: usize = 0x3E8;
pub const STMMAC_DEFAULT_TWT_LS: usize = 0x1E;

pub const STMMAC_CHAIN_MODE: u32 = 0x1;
pub const STMMAC_RING_MODE: u32 = 0x2;

pub const JUMBO_LEN: usize = 9000;

/* Receive Side Scaling */
pub const STMMAC_RSS_HASH_KEY_SIZE: usize = 40;
pub const STMMAC_RSS_MAX_TABLE_SIZE: usize = 256;

/* VLAN */
pub const STMMAC_VLAN_NONE: usize = 0x0;
pub const STMMAC_VLAN_REMOVE: usize = 0x1;
pub const STMMAC_VLAN_INSERT: usize = 0x2;
pub const STMMAC_VLAN_REPLACE: usize = 0x3;

