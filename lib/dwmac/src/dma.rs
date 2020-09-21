use crate::common_h::*;
use crate::dma_h::*;
use crate::dwmac_h::*;

#[derive(Debug, Clone, Default)]
pub struct DmaFeatures {
    pub mbps_10_100: u32,
    pub mbps_1000: u32,
    pub half_duplex: u32,
    pub hash_filter: u32,
    pub multi_addr: u32,
    pub pcs: u32,
    pub sma_mdio: u32,
    pub pmt_remote_wake_up: u32,
    pub pmt_magic_frame: u32,
    pub rmon: u32,
    /* IEEE 1588-2002 */
    pub time_stamp: u32,
    /* IEEE 1588-2008 */
    pub atime_stamp: u32,
    /* 802.3az - Energy-Efficient Ethernet (EEE) */
    pub eee: u32,
    pub av: u32,
    pub hash_tb_sz: u32,
    pub tsoen: u32,
    /* TX and RX csum */
    pub tx_coe: u32,
    pub rx_coe: u32,
    pub rx_coe_type1: u32,
    pub rx_coe_type2: u32,
    pub rxfifo_over_2048: u32,
    /* TX and RX number of channels */
    pub number_rx_channel: u32,
    pub number_tx_channel: u32,
    /* TX and RX number of queues */
    pub number_rx_queues: u32,
    pub number_tx_queues: u32,
    /* PPS output */
    pub pps_out_num: u32,
    /* Alternate (enhanced) DESC mode */
    pub enh_desc: u32,
    /* TX and RX FIFO sizes */
    pub tx_fifo_size: u32,
    pub rx_fifo_size: u32,
    /* Automotive Safety Package */
    pub asp: u32,
    /* RX Parser */
    pub frpsel: u32,
    pub frpbs: u32,
    pub frpes: u32,
    pub addr64: u32,
    pub rssen: u32,
    pub vlhash: u32,
    pub sphen: u32,
    pub vlins: u32,
    pub dvlan: u32,
    pub l3l4fnum: u32,
    pub arpoffsel: u32,
    /* TSN Features */
    pub estwid: u32,
    pub estdep: u32,
    pub estsel: u32,
    pub fpesel: u32,
    pub tbssel: u32,
}

impl From<u32> for DmaFeatures {
    fn from(hw_cap: u32) -> Self {
        let mut dma_cap = DmaFeatures::default();

        dma_cap.mbps_10_100 = (hw_cap & DMA_HW_FEAT_MIISEL);
        dma_cap.mbps_1000 = (hw_cap & DMA_HW_FEAT_GMIISEL) >> 1;
        dma_cap.half_duplex = (hw_cap & DMA_HW_FEAT_HDSEL) >> 2;
        dma_cap.hash_filter = (hw_cap & DMA_HW_FEAT_HASHSEL) >> 4;
        dma_cap.multi_addr = (hw_cap & DMA_HW_FEAT_ADDMAC) >> 5;
        dma_cap.pcs = (hw_cap & DMA_HW_FEAT_PCSSEL) >> 6;
        dma_cap.sma_mdio = (hw_cap & DMA_HW_FEAT_SMASEL) >> 8;
        dma_cap.pmt_remote_wake_up = (hw_cap & DMA_HW_FEAT_RWKSEL) >> 9;
        dma_cap.pmt_magic_frame = (hw_cap & DMA_HW_FEAT_MGKSEL) >> 10;
        /* MMC */
        dma_cap.rmon = (hw_cap & DMA_HW_FEAT_MMCSEL) >> 11;
        /* IEEE 1588-2002 */
        dma_cap.time_stamp =
            (hw_cap & DMA_HW_FEAT_TSVER1SEL) >> 12;
        /* IEEE 1588-2008 */
        dma_cap.atime_stamp = (hw_cap & DMA_HW_FEAT_TSVER2SEL) >> 13;
        /* 802.3az - Energy-Efficient Ethernet (EEE) */
        dma_cap.eee = (hw_cap & DMA_HW_FEAT_EEESEL) >> 14;
        dma_cap.av = (hw_cap & DMA_HW_FEAT_AVSEL) >> 15;
        /* TX and RX csum */
        dma_cap.tx_coe = (hw_cap & DMA_HW_FEAT_TXCOESEL) >> 16;
        dma_cap.rx_coe_type1 = (hw_cap & DMA_HW_FEAT_RXTYP1COE) >> 17;
        dma_cap.rx_coe_type2 = (hw_cap & DMA_HW_FEAT_RXTYP2COE) >> 18;
        dma_cap.rxfifo_over_2048 = (hw_cap & DMA_HW_FEAT_RXFIFOSIZE) >> 19;
        /* TX and RX number of channels */
        dma_cap.number_rx_channel = (hw_cap & DMA_HW_FEAT_RXCHCNT) >> 20;
        dma_cap.number_tx_channel = (hw_cap & DMA_HW_FEAT_TXCHCNT) >> 22;
        /* Alternate (enhanced) DESC mode */
        dma_cap.enh_desc = (hw_cap & DMA_HW_FEAT_ENHDESSEL) >> 24;

        dma_cap
    }
}

