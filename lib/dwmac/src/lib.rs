#![no_std]
#![feature(const_if_match)]

macro_rules! const_assert_size {
    ($expr:tt, $size:tt) => {
    const _: fn(a: $expr) -> [u8; $size] = |a| unsafe { core::mem::transmute::<$expr, [u8; $size]>(a) };
    };
}

extern crate alloc;
#[macro_use]
extern crate log;

use alloc::vec::Vec;
use core::time::Duration;

use common_h::*;
use descs_h::*;
use dma_h::*;
use dwmac_h::*;
use mac_h::*;
use mini_alloc::{Alloc, AllocRef, MiniBox};

use crate::dma::DmaFeatures;
use crate::mdio::{mdio_read, mdio_write, MY_MII};
use core::ops::DerefMut;

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
        let v = read_u32(addr);
        if let Some(p) = &mut val {
            **p = v;
        }
        if break_fn(v) {
            break;
        }
    }
}

#[repr(align(1024))]
struct MyArp(pub [u8; 42]);

static MY_ARP_PACKET: MyArp = MyArp([
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

struct Gmac {
    dev: GmacDevice,
    rings: GmacRings,
}

impl Gmac {
    pub fn open<H: Hooks>(al: AllocRef) -> Result<Gmac, Error> {
        H::sleep(Duration::from_millis(3000));

        info!("dwmac");

        meson8b::init();
        H::sleep(Duration::from_millis(10));

        let mut dev = GmacDevice::default();

        // DMA cap
        {
            let value = read_u32(BASE + DMA_HW_FEATURE);
            dev.dma_features = DmaFeatures::from(value);

            info!("dma cap: {:#08x}", value);
            info!("dma cap: {:?}", &dev.dma_features);
        }

        let mut rings = device_init::<H>(&mut dev, al);

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

        Ok(Gmac {
            dev,
            rings,
        })
    }
}


pub fn do_stuff<H: Hooks>(al: AllocRef) {
    match Gmac::open::<H>(al) {
        Err(e) => {
            info!("do_stuff(): error: {:?}", e);
        }
        Ok(mut gmac) => {
            info!("dwmac::do_stuff() done");

            #[allow(mutable_transmutes)]
            {
                for _ in 0..5 {
                    unsafe { core::mem::transmute::<&MyArp, &mut MyArp>(&MY_ARP_PACKET).0[41] += 1; }
                    gmac.rings.tx_rings[0].transmit_frame::<H>(&MY_ARP_PACKET.0);
                    H::sleep(Duration::from_millis(100));
                }
            }
            // loop {
            //     gmac.rings.rx_rings[0].receive_frames(&mut |buf| {
            //         info!("Received: {:?}", buf);
            //     });
            // }

            for _ in 0..10 {
                let dma_stat = read_u32(BASE + DMA_STATUS);
                info!("rx dma status: {:#x}", (dma_stat & DMA_STATUS_RS_MASK) >> DMA_STATUS_RS_SHIFT);
                info!("tx dma status: {:#x}", (dma_stat & DMA_STATUS_TS_MASK) >> DMA_STATUS_TS_SHIFT);
                H::sleep(Duration::from_millis(500));
            }

        }
    }
}

const TX_NUM_DESC: usize = 32;
const RX_NUM_DESC: usize = 32;
const MAX_FRAME_BODY: usize = 1600; // add 100 so the buffer is divisible by 8 (RDES1[10:0])

#[derive(Debug, Clone)]
enum Error {
    Str(&'static str),
}

impl From<&'static str> for Error {
    fn from(s: &'static str) -> Self {
        Error::Str(s)
    }
}

struct GmacRings {
    tx_rings: Vec<GmacTxRing>,
    rx_rings: Vec<GmacRxRing>,
}

impl GmacRings {
    pub fn new(num_tx: usize, num_rx: usize, al: AllocRef) -> Self {
        let mut tx_rings = Vec::new();
        for i in 0..num_tx {
            tx_rings.push(GmacTxRing::new(i, al));
        }

        let mut rx_rings = Vec::new();
        for i in 0..num_rx {
            rx_rings.push(GmacRxRing::new(i, al));
        }

        GmacRings {
            tx_rings,
            rx_rings,
        }
    }
}


struct GmacTxRing {
    tx_buffer: MiniBox<[u8; GmacTxRing::BUF_SIZE]>,
    tx_queue: TxQueue,
}

impl GmacTxRing {
    const BUF_SIZE: usize = MAX_FRAME_BODY * TX_NUM_DESC;

    pub fn new(index: usize, al: AllocRef) -> Self {
        let tx_buffer: MiniBox<[u8; Self::BUF_SIZE]> = unsafe { MiniBox::new_zeroed(al) };
        info!("ADDR TxRing buf: {:#x}", tx_buffer.as_ptr() as usize);
        let tx_queue = TxQueue::new(index, al);

        GmacTxRing {
            tx_buffer,
            tx_queue,
        }
    }

    fn buffer_idx(&mut self, i: usize) -> &mut [u8] {
        &mut self.tx_buffer[MAX_FRAME_BODY * i..MAX_FRAME_BODY * (i + 1)]
    }

    pub fn transmit_frame<H: Hooks>(&mut self, frame: &[u8]) -> Result<(), Error> {
        let frame_len = frame.len();
        if frame_len > MAX_FRAME_BODY {
            return Err(Error::Str("frame too large"));
        }

        self.tx_queue.vacuum();

        let idx = self.tx_queue.next_id().ok_or("no free tx queue space")?;

        info!("using buf index: {}", idx);

        (&mut self.buffer_idx(idx)[..frame_len]).copy_from_slice(frame);

        self.tx_queue.dma_tx[idx].prepare_tx(true, true, false, STMMAC_RING_MODE, frame_len);
        let ptr = self.buffer_idx(idx).as_ptr() as usize;
        self.tx_queue.dma_tx[idx].set_address(ptr);
        self.tx_queue.dma_tx[idx].set_owner();

        H::memory_barrier();
        write_u32(BASE + DMA_XMT_POLL_DEMAND, 1);

        Ok(())
    }
}

struct GmacRxRing {
    rx_buffer: MiniBox<[u8; GmacRxRing::BUF_SIZE]>,
    rx_queue: RxQueue,
}

impl GmacRxRing {
    const BUF_SIZE: usize = MAX_FRAME_BODY * RX_NUM_DESC;

    pub fn new(index: usize, al: AllocRef) -> Self {
        let rx_buffer: MiniBox<[u8; Self::BUF_SIZE]> = unsafe { MiniBox::new_zeroed(al) };
        info!("ADDR RxRing buf: {:#x}", rx_buffer.as_ptr() as usize);

        let rx_queue = RxQueue::new(index, al);

        let mut ring = GmacRxRing {
            rx_buffer,
            rx_queue,
        };

        // for i in 0..RX_NUM_DESC {
        //     let mut desc = &mut ring.rx_queue.dma_rx[i];
        //     desc.init_rx(i + 1 == RX_NUM_DESC);
        //     desc.set_address(ring.rx_buffers[i].as_ptr() as usize);
        //     desc.prepare_rx(STMMAC_RING_MODE, MAX_FRAME_BODY);
        // }

        ring
    }

    fn buffer_idx(buffer: &[u8], i: usize) -> &[u8] {
        &buffer[MAX_FRAME_BODY * i..MAX_FRAME_BODY * (i + 1)]
    }

    pub fn receive_frames(&mut self, mut callback: &mut dyn FnMut(&[u8])) -> Result<(), Error> {
        let Self { rx_queue, rx_buffer } = self;

        rx_queue.process_received(&mut |idx, len| {
            callback(&Self::buffer_idx(rx_buffer.as_ref(), idx)[..len]);
        });

        write_u32(BASE + DMA_RCV_POLL_DEMAND, 1);

        Ok(())
    }
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
    des2: u32,
    des3: u32,
}

#[derive(Clone, Default)]
#[repr(C, packed)]
struct TxDesc(DmaDesc);

const_assert_size!(TxDesc, 16);

impl TxDesc {
    pub fn clear(&mut self) {
        self.0.des2 = 0; // only des2 needs to be zeroed to clear it for the dma.
    }

    pub fn init_tx(&mut self, end: bool) {
        self.0.des0 &= !TDES0_OWN;
        // assuming ring mode
        if end {
            self.0.des1 |= TDES1_END_RING;
        } else {
            self.0.des1 &= !TDES1_END_RING;
        }
    }

    pub fn set_address(&mut self, addr: usize) {
        assert!(addr < u32::max_value() as usize);
        self.0.des2 = addr as u32;
    }

    pub fn set_owner(&mut self) {
        self.0.des0 |= TDES0_OWN;
    }

    pub fn clear_owner(&mut self) {
        self.0.des0 &= !TDES0_OWN;
    }

    pub fn is_owned_by_dma(&self) -> bool {
        (self.0.des0 & TDES0_OWN) != 0
    }

    pub fn prepare_tx(&mut self, first_segment: bool, last_segment: bool, insert_checksum: bool, mode: u32, len: usize) {
        let mut des1 = self.0.des1;
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

        assert_eq!(mode, STMMAC_RING_MODE);

        des1 &= !TDES1_BUFFER1_SIZE_MASK;
        des1 |= (len as u32) & TDES1_BUFFER1_SIZE_MASK;

        self.0.des1 = des1;
    }

    pub fn dump(&self) {
        info!("des0: {:#08x}", self.0.des0);
        info!("des1: {:#08x}", self.0.des1);
        info!("des2: {:#08x}", self.0.des2);
        info!("des3: {:#08x}", self.0.des3);
    }
}

#[derive(Clone, Default)]
#[repr(C, packed)]
struct RxDesc(DmaDesc);

impl RxDesc {
    pub fn clear(&mut self) {
        self.0.des2 = 0; // only des2 needs to be zeroed to clear it for the dma.
    }

    pub fn init_rx(&mut self, end: bool) {
        self.0.des0 &= !RDES0_OWN;
        // assuming ring mode
        if end {
            self.0.des1 |= RDES1_END_RING;
        } else {
            self.0.des1 &= !RDES1_END_RING;
        }
    }

    pub fn set_address(&mut self, addr: usize) {
        assert!(addr < u32::max_value() as usize);
        self.0.des2 = addr as u32;
    }

    pub fn set_owned_by_dma(&mut self) {
        self.0.des0 |= RDES0_OWN;
    }

    pub fn clear_owned_by_dma(&mut self) {
        self.0.des0 &= !RDES0_OWN;
    }

    pub fn is_owned_by_dma(&self) -> bool {
        (self.0.des0 & RDES0_OWN) != 0
    }

    pub fn prepare_rx(&mut self, mode: u32, len: usize) {
        assert_eq!(mode, STMMAC_RING_MODE);

        let mut des1 = self.0.des1;

        des1 &= !RDES1_BUFFER1_SIZE_MASK;
        des1 |= (len as u32) & RDES1_BUFFER1_SIZE_MASK;

        self.0.des1 = des1;
    }

    pub fn dump(&self) {
        info!("des0: {:#08x}", self.0.des0);
        info!("des1: {:#08x}", self.0.des1);
        info!("des2: {:#08x}", self.0.des2);
        info!("des3: {:#08x}", self.0.des3);
    }
}

struct TxQueue {
    index: usize,
    dma_tx: MiniBox<[TxDesc; TX_NUM_DESC]>,
    next_idx: usize,
    sending_idx: usize,
}

impl TxQueue {
    pub fn new(index: usize, al: AllocRef) -> Self {
        let mut dma_tx: MiniBox<[TxDesc; TX_NUM_DESC]> = unsafe { MiniBox::new_zeroed(al) };
        info!("ADDR TxQueue dma: {:#x}", dma_tx.as_ptr() as usize);
        for (i, desc) in dma_tx.iter_mut().enumerate() {
            desc.init_tx(i == TX_NUM_DESC - 1);
        }

        Self {
            index,
            dma_tx,
            next_idx: 0,
            sending_idx: 0,
        }
    }

    pub fn next_id(&mut self) -> Option<usize> {
        let idx = self.next_idx;
        let next = (idx + 1) % TX_NUM_DESC;
        if next == self.sending_idx {
            return None
        }
        self.next_idx = next;
        Some(idx)
    }

    pub fn vacuum(&mut self) {
        while self.sending_idx != self.next_idx {
            if self.dma_tx[self.sending_idx].is_owned_by_dma() {
                break;
            }

            info!("vacuum index: {}", self.sending_idx);

            // TODO mark transaction successful?

            self.sending_idx = (self.sending_idx + 1) % TX_NUM_DESC;
        }
    }
}

struct RxQueue {
    index: usize,
    dma_rx: MiniBox<[RxDesc; RX_NUM_DESC]>,
    next_idx: usize,
    receive_idx: usize,
}

impl RxQueue {
    pub fn new(index: usize, al: AllocRef) -> Self {
        let mut dma_rx: MiniBox<[RxDesc; RX_NUM_DESC]> = unsafe { MiniBox::new_zeroed(al) };
        info!("ADDR RxQueue dma: {:#x}", dma_rx.as_ptr() as usize);
        for (i, desc) in dma_rx.iter_mut().enumerate() {
            desc.init_rx(i == RX_NUM_DESC - 1);
        }

        Self {
            index,
            dma_rx,
            next_idx: 0,
            receive_idx: 0,
        }
    }

    pub fn next_id(&mut self) -> Option<usize> {
        let idx = self.next_idx;
        let next = (idx + 1) % RX_NUM_DESC;
        if next == self.receive_idx {
            return None
        }
        self.next_idx = next;
        Some(idx)
    }

    pub fn process_received(&mut self, mut callback: &mut dyn FnMut(usize, usize)) {
        while self.receive_idx != self.next_idx {
            if self.dma_rx[self.receive_idx].is_owned_by_dma() {
                break;
            }

            let des0 = self.dma_rx[self.receive_idx].0.des0;
            assert_ne!(des0 & RDES0_FIRST_DESCRIPTOR, 0);
            assert_ne!(des0 & RDES0_LAST_DESCRIPTOR, 0);

            let length = (des0 & RDES0_FRAME_LEN_MASK) >> RDES0_FRAME_LEN_SHIFT;

            callback(self.receive_idx, length as usize);

            self.dma_rx[self.receive_idx].set_owned_by_dma();

            self.receive_idx = (self.receive_idx + 1) % RX_NUM_DESC;
        }
    }
}

#[derive(Default)]
struct GmacDevice {
    platform: Platform,
    dma_features: DmaFeatures,
    buf_size: usize,
    rx_copybreak: u32,
    extend_desc: bool,
    // todo should be true?
    speed: usize,
    // tx_queues: [Option<TxQueue>; MTL_MAX_TX_QUEUES],
}

// http://10.45.1.22/source/xref/linux/drivers/net/ethernet/stmicro/stmmac/stmmac_main.c?r=77b28983#2758
fn device_init<H: Hooks>(dev: &mut GmacDevice, al: AllocRef) -> GmacRings {

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

    let mut rings = GmacRings::new(dev.platform.tx_queues_to_use, dev.platform.rx_queues_to_use, al);

    info!("hw_setup");

    // hw_setup()
    hw_setup(dev, &mut rings);

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

    rings
}

fn init_phy<H: Hooks>(dev: &GmacDevice) {
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

// http://10.45.1.22/source/xref/linux/drivers/net/ethernet/stmicro/stmmac/stmmac_main.c?r=77b28983#2629
fn hw_setup(dev: &mut GmacDevice, rings: &mut GmacRings) {
    info!("init_dma_engine");

    // init_dma_engine()
    init_dma_engine(dev, rings);

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
        value |= MAC_ENABLE_TX;
        // value |= MAC_ENABLE_RX;
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

    // for i in 0..dev.platform.rx_queues_to_use {
    //     assert_eq!(i, 0);
    //     let mut value = read_u32(BASE + DMA_CONTROL);
    //     value |= DMA_CONTROL_SR;
    //     write_u32(BASE + DMA_CONTROL, value);
    // }
}

fn init_dma_engine(dev: &mut GmacDevice, rings: &mut GmacRings) {
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
        let ptr = rings.tx_rings[i].tx_queue.dma_tx.as_ptr() as u64;
        assert!(ptr < u32::max_value() as u64);
        assert_eq!(i, 0);
        write_u32(BASE + DMA_TX_BASE_ADDR, ptr as u32);
    }

    // set rx chan
    // for i in 0..dev.platform.rx_queues_to_use {
    //     let ptr = rings.rx_rings[i].rx_queue.dma_rx.as_ptr() as u64;
    //     assert!(ptr < u32::max_value() as u64);
    //     assert_eq!(i, 0);
    //     write_u32(BASE + DMA_RCV_BASE_ADDR, ptr as u32);
    // }


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

fn gmac_core_init(dev: &mut GmacDevice) {
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
        value |= GMAC_CONTROL_TE; // | GMAC_CONTROL_RE;

        // clear any speed flags...
        // value &= !(GMAC_CONTROL_PS | GMAC_CONTROL_FES);

        value = (GMAC_CONTROL_PS | GMAC_CONTROL_FES);
        // no flags == gigabit
    }

    info!("gmac control: {:#08x}", value);
    write_u32(BASE + GMAC_CONTROL, value);

    write_u32(BASE + GMAC_INT_MASK, GMAC_INT_DEFAULT_MASK);
}

fn dma_operation_mode(dev: &mut GmacDevice) {
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

