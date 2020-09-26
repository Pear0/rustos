#![no_std]
#![feature(const_if_match)]

extern crate alloc;
#[macro_use]
extern crate log;

use core::time::Duration;

use common_h::*;
use descs_h::*;
use dma_h::*;
use dwmac_h::*;
use mac_h::*;
use crate::dma::DmaFeatures;
use crate::mdio::{MY_MII, mdio_read, mdio_write};
use alloc::vec::Vec;

mod common_h;
mod descs_h;
mod dma;
mod dma_h;
mod dwmac_h;
mod mac_h;
mod mdio;
mod meson8b;

pub(crate) fn read_u32(addr: usize) -> u32 {
    unsafe { (addr as *const u32).read_volatile() }
}

pub(crate) fn write_u32(addr: usize, value: u32) {
    unsafe { (addr as *mut u32).write_volatile(value) }
}

pub(crate) fn read_u32_poll<F>(addr: usize, mut val: Option<&mut u32>, break_fn: F) where F: Fn(u32) -> bool {
    loop {
        let v  = read_u32(addr);
        if let Some(p) = &mut val {
            **p = v;
        }
        if break_fn(v) {
            break
        }
    }
}

#[repr(align(1024))]
struct MyArp(pub [u8; 42]);

const MY_ARP_PACKET: MyArp = MyArp([
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // destination
    0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, // source
    0x08, 0x06, // ethertype
    // ARP
    0x00, 0x01, // hardware type
    0x08, 0x00, // protocol type
    0x06, // hardware len
    0x04, // protocol len
    0x00, 0x01, // request
    0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, // sender hardware address
    10, 45, 52, 30, // sender protocol address
    0x00, 0x00, 0x00, 0x00, 0x0, 0x00, // target hardware address
    10, 45, 52, 34, // target protocol address
    // 0, 0, 0, 0, // eth crc
]);

const BASE: usize = 0xff3f0000;

const DMA_TX_SIZE: usize = 512;

#[derive(Copy, Clone, Debug)]
pub enum FlushType {
    /// Flush dirty cache lines to RAM (does not drop cache lines)
    Clean,
    /// Drop (potentially dirty) cache lines without writing back to RAM
    Invalidate,
    /// Clean then Drop
    CleanAndInvalidate,
}

pub trait Hooks {
    fn sleep(dur: Duration);

    fn memory_barrier();

    fn flush_cache(addr: u64, len: u64, flush: FlushType);
}

pub fn do_stuff<H: Hooks>() {
    H::sleep(Duration::from_millis(3000));

    info!("dwmac");

    meson8b::init();
    H::sleep(Duration::from_millis(10));

    let mut dev = Dev::default();

    // DMA cap
    {
        let value = read_u32(BASE + DMA_HW_FEATURE);
        dev.dma_features = DmaFeatures::from(value);

        info!("dma cap: {:#08x}", value);
        info!("dma cap: {:?}", &dev.dma_features);
    }

    open::<H>(&mut dev);

    info!("done....");

    // Read MAC Address
    {
        let low = read_u32(BASE + GMAC_ADDR_LOW(0));
        let high = read_u32(BASE + GMAC_ADDR_HIGH(0));
        info!("MAC addr: {:#08x} {:#08x}", low, high);
    }

    {
        let dma_stat = read_u32(BASE + DMA_STATUS);
        info!("rx dma status: {:#x}", (dma_stat & DMA_STATUS_RS_MASK) >> DMA_STATUS_RS_SHIFT);
        info!("tx dma status: {:#x}", (dma_stat & DMA_STATUS_TS_MASK) >> DMA_STATUS_TS_SHIFT);
    }

    for i in 0..32 {
        {
            let q = dev.tx_queues[0].as_mut().unwrap();
            let packet = &MY_ARP_PACKET.0;
            H::flush_cache(packet.as_ptr() as u64, packet.len() as u64, FlushType::Clean);

            q.dma_tx[i].prepare(true, true, false, STMMAC_RING_MODE, packet.len());
            q.dma_tx[i].set_address(packet.as_ptr() as usize);

            q.dma_tx[i].set_owner();
            q.dma_tx[i].dump();

            H::flush_cache(&q.dma_tx[i] as *const DmaDesc as u64, core::mem::size_of::<DmaDesc>() as u64, FlushType::Clean);
            H::memory_barrier();

            write_u32(BASE + DMA_XMT_POLL_DEMAND, 1);
        }


        loop {
            let dma_stat = read_u32(BASE + DMA_STATUS);
            let tx_status = (dma_stat & DMA_STATUS_TS_MASK) >> DMA_STATUS_TS_SHIFT;
            info!("tx dma status: {:#x} {:#x}", dma_stat, (dma_stat & DMA_STATUS_TS_MASK) >> DMA_STATUS_TS_SHIFT);
            // H::sleep(Duration::from_millis(1));
            if tx_status == 6 {
                break
            }
        }

        let q = dev.tx_queues[0].as_ref().unwrap();
        H::flush_cache(&q.dma_tx[i] as *const DmaDesc as u64, core::mem::size_of::<DmaDesc>() as u64, FlushType::Invalidate);

        q.dma_tx[i].dump();

        let stat = read_u32(BASE + DMA_STATUS);
        write_u32(BASE + DMA_STATUS, stat & 0x1ffff);
    }

    info!("dwmac::do_stuff() done");

}

struct Platform {
    rx_queues_to_use: usize,
    tx_queues_to_use: usize,
    rx_queues_cfg: [RxQueueCfg; MTL_MAX_RX_QUEUES],
    tx_queues_cfg: [TxQueueCfg; MTL_MAX_TX_QUEUES],
    rx_sched_algorithm: u32,
    tx_sched_algorithm: u32,
}

impl Default for Platform {
    fn default() -> Self {
        Self {
            rx_queues_to_use: 1,
            tx_queues_to_use: 1,
            rx_queues_cfg: Default::default(),
            tx_queues_cfg: Default::default(),
            rx_sched_algorithm: MTL_RX_ALGORITHM_SP,
            tx_sched_algorithm: MTL_TX_ALGORITHM_SP,
        }
    }
}


struct RxQueueCfg {
    mode_to_use: u32,
    chan: u8,
    prio: u32,
    use_prio: bool,
    pkt_route: u32,
}

impl Default for RxQueueCfg {
    fn default() -> Self {
        Self {
            mode_to_use: MTL_QUEUE_DCB,
            chan: 0,
            use_prio: false,
            prio: 0,
            pkt_route: 0,
        }
    }
}

struct TxQueueCfg {
    mode_to_use: u32,
    prio: u32,
    use_prio: bool,
    weight: u32,
}

impl Default for TxQueueCfg {
    fn default() -> Self {
        Self {
            mode_to_use: MTL_QUEUE_DCB,
            weight: 0x10, // + index
            use_prio: false,
            prio: 0,
        }
    }
}

#[derive(Clone, Default)]
#[repr(C, packed)]
struct DmaDesc {
    des0: u32,
    des1: u32,
    des2: u32, // address
    des3: u32,
}

impl DmaDesc {
    pub fn clear(&mut self) {
        self.des2 = 0; // only des2 needs to be zeroed to clear it for the dma.
    }

    pub fn init_tx(&mut self, end: bool) {
        self.des0 &= !TDES0_OWN;
        // assuming ring mode
        if end {
            self.des1 |= TDES1_END_RING;
        } else {
            self.des1 &= !TDES1_END_RING;
        }
    }

    pub fn set_address(&mut self, addr: usize) {
        assert!(addr < u32::max_value() as usize);
        self.des2 = addr as u32;
    }

    pub fn set_owner(&mut self) {
        self.des0 |= TDES0_OWN;
    }

    pub fn clear_owner(&mut self) {
        self.des0 &= !TDES0_OWN;
    }

    pub fn prepare(&mut self, first_segment: bool, last_segment: bool, insert_checksum: bool, mode: u32, len: usize) {
        let mut des1 = self.des1;
        if first_segment {
            des1 |= TDES1_FIRST_SEGMENT;
        } else {
            des1 &= !TDES1_FIRST_SEGMENT;
        }
        if insert_checksum {
            des1 |= TX_CIC_FULL << TDES1_CHECKSUM_INSERTION_SHIFT;
        } else {
            des1 &= !(TX_CIC_FULL << TDES1_CHECKSUM_INSERTION_SHIFT);
        }
        if last_segment {
            des1 |= TDES1_LAST_SEGMENT;
        } else {
            des1 &= !TDES1_LAST_SEGMENT;
        }
        self.des1 = des1;

        assert_eq!(mode, STMMAC_RING_MODE);
        self.des1 |= (len as u32) & TDES1_BUFFER1_SIZE_MASK;
    }

    pub fn dump(&self) {
        info!("des0: {:#08x}", self.des0);
        info!("des1: {:#08x}", self.des1);
        info!("des2: {:#08x}", self.des2);
        info!("des3: {:#08x}", self.des3);
    }

}

struct TxQueue {
    index: usize,
    dma_tx: Vec<DmaDesc>,
    dirty_tx: usize,
    cur_tx: usize,
    mss: usize,
}

impl TxQueue {
    pub fn new(index: usize) -> Self {
        let mut dma_tx = Vec::<DmaDesc>::new();
        dma_tx.reserve_exact(DMA_TX_SIZE);
        dma_tx.resize(DMA_TX_SIZE, DmaDesc::default());
        Self {
            index,
            dma_tx,
            dirty_tx: 0,
            cur_tx: 0,
            mss: 0,
        }
    }
}

#[derive(Default)]
struct Dev {
    platform: Platform,
    dma_features: DmaFeatures,
    buf_size: usize,
    rx_copybreak: u32,
    extend_desc: bool, // todo should be true?
    speed: usize,
    tx_queues: [Option<TxQueue>; MTL_MAX_TX_QUEUES],
}

// http://10.45.1.22/source/xref/linux/drivers/net/ethernet/stmicro/stmmac/stmmac_main.c?r=77b28983#2758
fn open<H: Hooks>(dev: &mut Dev) {

    // init_phy()
    init_phy::<H>(dev);

    info!("done init phy");

    // set_16kib_bfsize()

    // set_bfsize()
    let bfsize = 1536; // DEFAULT_BUFSIZE
    dev.buf_size = bfsize;

    // rx_copybreak
    dev.rx_copybreak = 256; // STMMAC_RX_COPYBREAK

    // TBS = time based scheduling
    // TBS chec? skip for now?

    info!("alloc_dma_tx_desc_resources");

    // alloc_dma_desc_resources()
    alloc_dma_tx_desc_resources(dev);

    info!("init_dma_tx_desc_rings");

    // init_dma_desc_rings()
    init_dma_tx_desc_rings(dev);

    info!("clear_tx_descriptors");

    clear_tx_descriptors(dev);

    info!("hw_setup");

    // hw_setup()
    hw_setup(dev);

    // TODO init_coalesce()

    // phy_start()
    {
        mdio_write(BASE, &MY_MII, 0, 0, 1 << 15);
        H::sleep(Duration::from_secs(2));

        for i in 4..=8 {
            info!("mdio reg {} : {:#04x}", i, mdio_read(BASE, &MY_MII, 0, i));
        }
    }

    // phy_speed_up()

    // setup irq

    // enable_all_queues()

    // start_all_queues()

}

fn init_phy<H: Hooks>(dev: &mut Dev) {

    mdio_write(BASE, &MY_MII, 0, 0, 1 << 15);

    H::sleep(Duration::from_millis(10));

    mdio_write(BASE, &MY_MII, 0, 0, 1 << 9);

    H::sleep(Duration::from_millis(10));

    let control = mdio_read(BASE, &MY_MII, 0, 0);
    let status = mdio_read(BASE, &MY_MII, 0, 1);
    info!("mdio control:{:#04x}, status:{:#04x}", control, status);

    let status_15 = mdio_read(BASE, &MY_MII, 0, 15);
    info!("mdio status_15:{:#04x}", status_15);



}

fn alloc_dma_tx_desc_resources(dev: &mut Dev) {
    let tx_count = dev.platform.tx_queues_to_use;
    for queue in 0..tx_count {
        dev.tx_queues[queue].replace(TxQueue::new(queue));
    }

}

fn init_dma_tx_desc_rings(dev: &mut Dev) {
    let tx_count = dev.platform.tx_queues_to_use;
    for queue in 0..tx_count {
        let mut q = &mut dev.tx_queues[queue].as_mut().unwrap();

        for e in q.dma_tx.iter_mut() {
            e.clear();
        }

        q.dirty_tx = 0;
        q.cur_tx = 0;
        q.mss = 0;
    }
}

fn clear_tx_descriptors(dev: &mut Dev) {
    let tx_count = dev.platform.tx_queues_to_use;
    for queue in 0..tx_count {
        let mut q = &mut dev.tx_queues[queue].as_mut().unwrap();

        for (i, e) in q.dma_tx.iter_mut().enumerate() {
            let last = i == DMA_TX_SIZE - 1;
            e.init_tx(last);
        }

    }
}

// http://10.45.1.22/source/xref/linux/drivers/net/ethernet/stmicro/stmmac/stmmac_main.c?r=77b28983#2629
fn hw_setup(dev: &mut Dev) {

    info!("init_dma_engine");

    // init_dma_engine()
    init_dma_engine(dev);

    info!("set_mac_address");

    // set_mac_address()
    {
        let (high, low) = encode_mac(&[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        write_u32(BASE + GMAC_ADDR_HIGH(0), high | GMAC_HI_REG_AE);
        write_u32(BASE + GMAC_ADDR_LOW(0), low);
    }

    // set hw_speed = 1000
    dev.speed = 1000;

    info!("gmac_core_init");

    // core_init()
    gmac_core_init(dev);

    // TODO mtl_configuration()

    // safety_feat_configuration()

    info!("mac_set");

    // mac_set()
    {
        let mut value = read_u32(BASE + GMAC_CONTROL);
        value |= (MAC_ENABLE_TX);
        write_u32(BASE + GMAC_CONTROL, value);
    }

    info!("dma_operation_mode");

    // dma_operation_mode()
    dma_operation_mode(dev);

    // TODO mmc_setup()

    // TODO ptp

    // tx_lpi_timer = ...

    // TODO riwt

    // pcs = rgmii
    assert_eq!(dev.dma_features.pcs, 0);
    // pcs_ctrl_ane() ->  dwmac1000_ctrl_ane(speed=1000,ane=1,loopback=0)


    // set_rings_length()

    // TODO enable tso

    // TODO enable split header

    // TODO enable vlan tag insertion

    // TODO any tbs stuff

    info!("start_all_dma");
    // start_all_dma()

    for i in 0..dev.platform.tx_queues_to_use {
        assert_eq!(i, 0);
        let mut value = read_u32(BASE + DMA_CONTROL);
        value |= DMA_CONTROL_ST;
        write_u32(BASE + DMA_CONTROL, value);
    }

}

fn init_dma_engine(dev: &mut Dev) {
    let atds = dev.extend_desc; // && ring mode == true

    info!("dma_reset");
    dma_reset();

    info!("dma_reset");
    dma_init(atds);

    // TODO dma_axi
    {
        let mut axi = read_u32(BASE + DMA_AXI_BUS_MODE);
        axi |= DMA_AXI_UNDEF;
        axi |= DMA_BURST_LEN_DEFAULT;
        axi |= DMA_AXI_MAX_OSR_LIMIT as u32;
        write_u32(BASE + DMA_AXI_BUS_MODE, axi);
    }


    info!("set tx chan");
    // set tx chan
    for i in 0..dev.platform.tx_queues_to_use {
        let mut q = dev.tx_queues[i].as_mut().unwrap();
        let ptr = q.dma_tx.as_ptr() as u64;
        assert!(ptr < u32::max_value() as u64);
        assert_eq!(i, 0);
        write_u32(BASE + DMA_TX_BASE_ADDR, ptr as u32);
    }

}

fn dma_reset() {
    let mut value = read_u32(BASE + DMA_BUS_MODE);
    value |= DMA_BUS_MODE_SFT_RESET;
    write_u32(BASE + DMA_BUS_MODE, value);
    read_u32_poll(BASE + DMA_BUS_MODE, None, |v| (v & DMA_BUS_MODE_SFT_RESET) == 0);
}

fn dma_init(atds: bool) {
    // programmable burst length
    let pbl = 8;
    let pblx8 = true;
    let aal = false; // address-aligned beats
    let fixed_burst = false;
    let mixed_burst = false;

    let mut value = read_u32(BASE + DMA_BUS_MODE);
    if pblx8 {
        value |= DMA_BUS_MODE_MAXPBL;
    }
    value |= DMA_BUS_MODE_USP;
    value &= !(DMA_BUS_MODE_PBL_MASK | DMA_BUS_MODE_RPBL_MASK);
    value |= (pbl << DMA_BUS_MODE_PBL_SHIFT); // transmit
    value |= (pbl << DMA_BUS_MODE_RPBL_SHIFT); // receive

    if fixed_burst {
        value |= DMA_BUS_MODE_FB;
    }
    if mixed_burst {
        value |= DMA_BUS_MODE_MB;
    }
    if atds {
        value |= DMA_BUS_MODE_ATDS;
    }
    if aal {
        value |= DMA_BUS_MODE_AAL;
    }

    write_u32(BASE + DMA_BUS_MODE, value);

    write_u32(BASE + DMA_INTR_ENA, DMA_INTR_DEFAULT_MASK);
}

// (high, low)
fn encode_mac(addr: &[u8]) -> (u32, u32) {
    assert_eq!(addr.len(), 6);
    let high = ((addr[5] as u32) << 8) | (addr[4] as u32);
    let low = ((addr[3] as u32) << 24) | ((addr[2] as u32) << 16) | ((addr[1] as u32) << 8) | (addr[0] as u32);
    (high, low)
}

fn gmac_core_init(dev: &mut Dev) {
    let mut value = read_u32(BASE + GMAC_CONTROL);

    value |= GMAC_CORE_INIT;

    /* Clear ACS bit because Ethernet switch tagging formats such as
     * Broadcom tags can look like invalid LLC/SNAP packets and cause the
     * hardware to truncate packets on reception.
     */
    value &= !GMAC_CONTROL_ACS;

    // value |= GMAC_CONTROL_2K; // mtu > 1500
    // value |= GMAC_CONTROL_JE; // mtu > 2000

    {
        assert_eq!(dev.speed, 1000);
        value |= GMAC_CONTROL_TE;

        // clear any speed flags...
        // value &= !(GMAC_CONTROL_PS | GMAC_CONTROL_FES);

        value = (GMAC_CONTROL_PS | GMAC_CONTROL_FES);
        // no flags == gigabit
    }

    info!("gmac control: {:#08x}", value);
    write_u32(BASE + GMAC_CONTROL, value);

    write_u32(BASE + GMAC_INT_MASK, GMAC_INT_DEFAULT_MASK);
}

fn dma_operation_mode(dev: &mut Dev) {
    // from DTB
    let mut txfifo_size = 2048;
    let mut rxfifo_size = 4096;

    txfifo_size /= dev.platform.tx_queues_to_use;
    rxfifo_size /= dev.platform.rx_queues_to_use;

    assert_ne!(dev.dma_features.tx_coe, 0);
    let txmode = SF_DMA_MODE;

    for i in 0..dev.platform.tx_queues_to_use {
        dma_tx_mode(i, txfifo_size, txmode);
    }

}

fn dma_tx_mode(channel: usize, fifo_size: usize, mode: usize) {
    assert_eq!(channel, 0);
    assert_eq!(mode, SF_DMA_MODE);

    let mut csr6 = read_u32(BASE + DMA_CONTROL);
    /* Transmit COE type 2 cannot be done in cut-through mode. */
    csr6 |= DMA_CONTROL_TSF;
    /* Operating on second frame increase the performance
  	 * especially when transmit store-and-forward is used.
  	 */
    csr6 |= DMA_CONTROL_OSF;

    write_u32(BASE + DMA_CONTROL, csr6);
}

