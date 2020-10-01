#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

/* Normal receive descriptor defines */

/* RDES0 */
pub const RDES0_PAYLOAD_CSUM_ERR: u32 = 1 << 0;
pub const RDES0_CRC_ERROR: u32 = 1 << 1;
pub const RDES0_DRIBBLING: u32 = 1 << 2;
pub const RDES0_MII_ERROR: u32 = 1 << 3;
pub const RDES0_RECEIVE_WATCHDOG: u32 = 1 << 4;
pub const RDES0_FRAME_TYPE: u32 = 1 << 5;
pub const RDES0_COLLISION: u32 = 1 << 6;
pub const RDES0_IPC_CSUM_ERROR: u32 = 1 << 7;
pub const RDES0_LAST_DESCRIPTOR: u32 = 1 << 8;
pub const RDES0_FIRST_DESCRIPTOR: u32 = 1 << 9;
pub const RDES0_VLAN_TAG: u32 = 1 << 10;
pub const RDES0_OVERFLOW_ERROR: u32 = 1 << 11;
pub const RDES0_LENGTH_ERROR: u32 = 1 << 12;
pub const RDES0_SA_FILTER_FAIL: u32 = 1 << 13;
pub const RDES0_DESCRIPTOR_ERROR: u32 = 1 << 14;
pub const RDES0_ERROR_SUMMARY: u32 = 1 << 15;
pub const RDES0_FRAME_LEN_MASK: u32 = GENMASK(29, 16);
pub const RDES0_FRAME_LEN_SHIFT: usize = 16;
pub const RDES0_DA_FILTER_FAIL: u32 = 1 << 30;
pub const RDES0_OWN: u32 = 1 << 31;
/* RDES1 */
pub const RDES1_BUFFER1_SIZE_MASK: u32 = GENMASK(10, 0);
pub const RDES1_BUFFER2_SIZE_MASK: u32 = GENMASK(21, 11);
pub const RDES1_BUFFER2_SIZE_SHIFT: usize = 11;
pub const RDES1_SECOND_ADDRESS_CHAINED: u32 = 1 << 24;
pub const RDES1_END_RING: u32 = 1 << 25;
pub const RDES1_DISABLE_IC: u32 = 1 << 31;

/* Enhanced receive descriptor defines */

/* RDES0 (similar to normal RDES) */
pub const ERDES0_RX_MAC_ADDR: u32 = 1 << 0;

/* RDES1: completely differ from normal desc definitions */
pub const ERDES1_BUFFER1_SIZE_MASK: u32 = GENMASK(12, 0);
pub const ERDES1_SECOND_ADDRESS_CHAINED: u32 = 1 << 14;
pub const ERDES1_END_RING: u32 = 1 << 15;
pub const ERDES1_BUFFER2_SIZE_MASK: u32 = GENMASK(28, 16);
pub const ERDES1_BUFFER2_SIZE_SHIFT: usize = 16;
pub const ERDES1_DISABLE_IC: u32 = 1 << 31;

/* Normal transmit descriptor defines */
/* TDES0 */
pub const TDES0_DEFERRED: u32 = 1 << 0;
pub const TDES0_UNDERFLOW_ERROR: u32 = 1 << 1;
pub const TDES0_EXCESSIVE_DEFERRAL: u32 = 1 << 2;
pub const TDES0_COLLISION_COUNT_MASK: u32 = GENMASK(6, 3);
pub const TDES0_VLAN_FRAME: u32 = 1 << 7;
pub const TDES0_EXCESSIVE_COLLISIONS: u32 = 1 << 8;
pub const TDES0_LATE_COLLISION: u32 = 1 << 9;
pub const TDES0_NO_CARRIER: u32 = 1 << 10;
pub const TDES0_LOSS_CARRIER: u32 = 1 << 11;
pub const TDES0_PAYLOAD_ERROR: u32 = 1 << 12;
pub const TDES0_FRAME_FLUSHED: u32 = 1 << 13;
pub const TDES0_JABBER_TIMEOUT: u32 = 1 << 14;
pub const TDES0_ERROR_SUMMARY: u32 = 1 << 15;
pub const TDES0_IP_HEADER_ERROR: u32 = 1 << 16;
pub const TDES0_TIME_STAMP_STATUS: u32 = 1 << 17;
pub const TDES0_OWN: u32 = 1 << 31;	/* silence sparse */
/* TDES1 */
pub const TDES1_BUFFER1_SIZE_MASK: u32 = GENMASK(10, 0);
pub const TDES1_BUFFER2_SIZE_MASK: u32 = GENMASK(21, 11);
pub const TDES1_BUFFER2_SIZE_SHIFT: usize = 11;
pub const TDES1_TIME_STAMP_ENABLE: u32 = 1 << 22;
pub const TDES1_DISABLE_PADDING: u32 = 1 << 23;
pub const TDES1_SECOND_ADDRESS_CHAINED: u32 = 1 << 24;
pub const TDES1_END_RING: u32 = 1 << 25;
pub const TDES1_CRC_DISABLE: u32 = 1 << 26;
pub const TDES1_CHECKSUM_INSERTION_MASK: u32 = GENMASK(28, 27);
pub const TDES1_CHECKSUM_INSERTION_SHIFT: usize = 27;
pub const TDES1_FIRST_SEGMENT: u32 = 1 << 29;
pub const TDES1_LAST_SEGMENT: u32 = 1 << 30;
pub const TDES1_INTERRUPT: u32 = 1 << 31;

/* Enhanced transmit descriptor defines */
/* TDES0 */
pub const ETDES0_DEFERRED: u32 = 1 << 0;
pub const ETDES0_UNDERFLOW_ERROR: u32 = 1 << 1;
pub const ETDES0_EXCESSIVE_DEFERRAL: u32 = 1 << 2;
pub const ETDES0_COLLISION_COUNT_MASK: u32 = GENMASK(6, 3);
pub const ETDES0_VLAN_FRAME: u32 = 1 << 7;
pub const ETDES0_EXCESSIVE_COLLISIONS: u32 = 1 << 8;
pub const ETDES0_LATE_COLLISION: u32 = 1 << 9;
pub const ETDES0_NO_CARRIER: u32 = 1 << 10;
pub const ETDES0_LOSS_CARRIER: u32 = 1 << 11;
pub const ETDES0_PAYLOAD_ERROR: u32 = 1 << 12;
pub const ETDES0_FRAME_FLUSHED: u32 = 1 << 13;
pub const ETDES0_JABBER_TIMEOUT: u32 = 1 << 14;
pub const ETDES0_ERROR_SUMMARY: u32 = 1 << 15;
pub const ETDES0_IP_HEADER_ERROR: u32 = 1 << 16;
pub const ETDES0_TIME_STAMP_STATUS: u32 = 1 << 17;
pub const ETDES0_SECOND_ADDRESS_CHAINED: u32 = 1 << 20;
pub const ETDES0_END_RING: u32 = 1 << 21;
pub const ETDES0_CHECKSUM_INSERTION_MASK: u32 = GENMASK(23, 22);
pub const ETDES0_CHECKSUM_INSERTION_SHIFT: usize = 22;
pub const ETDES0_TIME_STAMP_ENABLE: u32 = 1 << 25;
pub const ETDES0_DISABLE_PADDING: u32 = 1 << 26;
pub const ETDES0_CRC_DISABLE: u32 = 1 << 27;
pub const ETDES0_FIRST_SEGMENT: u32 = 1 << 28;
pub const ETDES0_LAST_SEGMENT: u32 = 1 << 29;
pub const ETDES0_INTERRUPT: u32 = 1 << 30;
pub const ETDES0_OWN: u32 = 1 << 31;	/* silence sparse */
/* TDES1 */
pub const ETDES1_BUFFER1_SIZE_MASK: u32 = GENMASK(12, 0);
pub const ETDES1_BUFFER2_SIZE_MASK: u32 = GENMASK(28, 16);
pub const ETDES1_BUFFER2_SIZE_SHIFT: usize = 16;

/* Extended Receive descriptor definitions */
pub const ERDES4_IP_PAYLOAD_TYPE_MASK: u32 = GENMASK(6, 2);
pub const ERDES4_IP_HDR_ERR: u32 = 1 << 3;
pub const ERDES4_IP_PAYLOAD_ERR: u32 = 1 << 4;
pub const ERDES4_IP_CSUM_BYPASSED: u32 = 1 << 5;
pub const ERDES4_IPV4_PKT_RCVD: u32 = 1 << 6;
pub const ERDES4_IPV6_PKT_RCVD: u32 = 1 << 7;
pub const ERDES4_MSG_TYPE_MASK: u32 = GENMASK(11, 8);
pub const ERDES4_PTP_FRAME_TYPE: u32 = 1 << 12;
pub const ERDES4_PTP_VER: u32 = 1 << 13;
pub const ERDES4_TIMESTAMP_DROPPED: u32 = 1 << 14;
pub const ERDES4_AV_PKT_RCVD: u32 = 1 << 16;
pub const ERDES4_AV_TAGGED_PKT_RCVD: u32 = 1 << 17;
pub const ERDES4_VLAN_TAG_PRI_VAL_MASK: u32 = GENMASK(20, 18);
pub const ERDES4_L3_FILTER_MATCH: u32 = 1 << 24;
pub const ERDES4_L4_FILTER_MATCH: u32 = 1 << 25;
pub const ERDES4_L3_L4_FILT_NO_MATCH_MASK: u32 = GENMASK(27, 26);

/* Extended RDES4 message type definitions */
pub const RDES_EXT_NO_PTP: usize = 0x0;
pub const RDES_EXT_SYNC: usize = 0x1;
pub const RDES_EXT_FOLLOW_UP: usize = 0x2;
pub const RDES_EXT_DELAY_REQ: usize = 0x3;
pub const RDES_EXT_DELAY_RESP: usize = 0x4;
pub const RDES_EXT_PDELAY_REQ: usize = 0x5;
pub const RDES_EXT_PDELAY_RESP: usize = 0x6;
pub const RDES_EXT_PDELAY_FOLLOW_UP: usize = 0x7;
pub const RDES_PTP_ANNOUNCE: usize = 0x8;
pub const RDES_PTP_MANAGEMENT: usize = 0x9;
pub const RDES_PTP_SIGNALING: usize = 0xa;
pub const RDES_PTP_PKT_RESERVED_TYPE: usize = 0xf;

/* Basic descriptor structure for normal and alternate descriptors */
// struct dma_desc {
//     __le32 des0;
//     __le32 des1;
//     __le32 des2;
//     __le32 des3;
// };

/* Extended descriptor structure (e.g. >= databook 3.50a) */
// struct dma_extended_desc {
// struct dma_desc basic;	/* Basic descriptors */
// __le32 des4;	/* Extended Status */
// __le32 des5;	/* Reserved */
// __le32 des6;	/* Tx/Rx Timestamp Low */
// __le32 des7;	/* Tx/Rx Timestamp High */
// };

/* Enhanced descriptor for TBS */
// struct dma_edesc {
// __le32 des4;
// __le32 des5;
// __le32 des6;
// __le32 des7;
// struct dma_desc basic;
// };

/* Transmit checksum insertion control */
pub const TX_CIC_FULL: u32 = 3;	/* Include IP header and pseudoheader */

const fn GENMASK(h: usize, l: usize) -> u32 {
    ((u32::max_value() - (1u32 << l) + 1) & (u32::max_value() >> (32 - 1 - h)))
}
