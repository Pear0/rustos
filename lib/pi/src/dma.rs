mod regs {
    // Control and Status register
    defbit32!(DMA_CS, [
        RESET [31-31],
        ABORT [30-30],
        DISDEBUG [29-29],
        WAIT_FOR_OUTSTANDING_WRITES [28-28],
        PANIC_PRI [23-20],
        PRI [19-16],
        ERROR [8-8],
        WAITING_FOR_OUTSTANDING_WRITES [6-6],
        DREQ_STOPS_DMA [5-5],
        PAUSED [4-4],
        DREQ [3-3],
        INT [2-2],
        END [1-1],
        ACTIVE [0-0],

        RES0 [27-24|15-9|7-7],
    ]);

    // Transfer Information
    defbit32!(DMA_TI, [
        NO_WIDE_BURSTS [26-26],
        WAITS [25-21],
        PERMAP [20-16],
        BURST_LEN [15-12],
        SRC_IGNORE [11-11],
        SRC_DREQ [10-10],
        SRC_WIDTH [9-9],
        SRC_INC [8-8],
        DEST_IGNORE [7-7],
        DEST_DREQ [6-6],
        DEST_WIDTH [5-5],
        DEST_INC [4-4],
        WAIT_RESP [3-3],
        TD_MODE [1-1],
        INTEN [0-0],

        RES0 [31-27|2-2],
    ]);

    // Transfer Length
    defbit32!(DMA_TXFR_LEN, [
        // x length in normal mode
        X_LEN [29-0],

        // 2d mode
        TD_Y_LEN [29-16],
        TD_X_LEN [15-0],

        RES0 [31-30],
    ]);

    // 2D Stride
    defbit32!(DMA_TD_STRIDE, [
        D_STRIDE [31-16],
        S_STRIDE [15-0],
    ]);

}

#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct BusAddress(u32);

impl BusAddress {
    pub fn new(addr: u64) -> Self {
        assert_eq!(addr >> 31, 0);
        let addr = addr + 0xC0000000; // SDRAM uncached offset on peripheral bus
        assert!(addr < u32::max_value() as u64);
        BusAddress(addr as u32)
    }
}

impl<T> From<&T> for BusAddress {
    fn from(t: &T) -> Self {
        BusAddress::new(t as *const T as u64)
    }
}

impl<T> From<*const T> for BusAddress {
    fn from(t: *const T) -> Self {
        BusAddress::new(t as u64)
    }
}

impl<T> From<*mut T> for BusAddress {
    fn from(t: *mut T) -> Self {
        BusAddress::new(t as u64)
    }
}


use regs::*;
use mini_alloc::MiniBox;
use volatile::{Volatile, ReadVolatile};
use crate::common::{IO_BASE, DMA_CHANNEL_BASE, DMA_CHANNEL_15};
use shim::const_assert_size;
use crate::timer::Waiter;
use core::cmp::min;
use core::time::Duration;

/// a struct matching the BCM 2835 DMA Control Block structure.
#[repr(C, align(32))]
#[derive(Clone, Debug, Default)]
struct InnerBlock {
    transfer: DMA_TI,
    source: BusAddress,
    destination: BusAddress,
    txfr_len: DMA_TXFR_LEN,
    td_stride: DMA_TD_STRIDE,
    next_block: BusAddress,
    __r0: u32,
    __r1: u32,
}

#[derive(Debug)]
pub enum Source {
    Increasing(*const u32, usize),
    Constant(u32),
}

#[derive(Debug)]
pub enum Destination {
    Increasing(*mut u32, usize),
}

/// A wrapper type that is used by Rust code to more ergonomically configure
/// DMA transfers and
#[derive(Debug)]
pub struct ControlBlock {
    inner: InnerBlock,
    const_src: [u32; 8],

    pub source: Source,
    pub destination: Destination,
    pub next: Option<MiniBox<ControlBlock>>,
}

impl ControlBlock {
    pub fn new() -> Self {
        Self {
            inner: InnerBlock::default(),
            const_src: [0; 8],
            source: Source::Constant(0),
            destination: Destination::Increasing(0 as *mut u32, 0),
            next: None,
        }
    }
}

#[repr(C)]
struct Registers {
    cs: Volatile<DMA_CS>,
    block: Volatile<BusAddress>,

    // set by DMA controller
    ti: ReadVolatile<DMA_TI>,
    source: ReadVolatile<BusAddress>,
    destination: ReadVolatile<BusAddress>,
    txfer_len: ReadVolatile<DMA_TXFR_LEN>,
    stride: ReadVolatile<DMA_TD_STRIDE>,
    next_block: ReadVolatile<BusAddress>,
}

const_assert_size!(Registers, 32);


fn get_registers_address(id: usize) -> usize {
    match id {
        0..=14 => DMA_CHANNEL_BASE + 0x100,
        15 => DMA_CHANNEL_15,
        _ => panic!("bad dma address: {}", id),
    }
}

pub struct Controller {
    registers: &'static mut Registers,
    dma_id: usize,
}

impl Controller {
    pub unsafe fn new(id: usize) -> Controller {
        let addr = get_registers_address(id);
        let registers = &mut *(addr as *mut Registers);

        Controller { registers, dma_id: id }
    }

    fn encode(block: &mut ControlBlock) {
        #![allow(irrefutable_let_patterns)]

        /*
        transfer: DMA_TI,
        txfr_len: DMA_TXFR_LEN,
        td_stride: DMA_TD_STRIDE,
        */

        block.inner.transfer.set(0);
        // block.inner.transfer.set_value(10, DMA_TI::WAITS);
        block.inner.transfer.set_value(1, DMA_TI::BURST_LEN);

        // block.inner.transfer.set_value(1, DMA_TI::WAIT_RESP);

        // block.inner.transfer.set_value(1, DMA_TI::NO_WIDE_BURSTS);

        block.inner.transfer.set_value(1, DMA_TI::SRC_WIDTH);
        block.inner.transfer.set_value(1, DMA_TI::DEST_WIDTH);

        if let Source::Increasing(_, _) = block.source {
            block.inner.transfer.set_bit(DMA_TI::SRC_INC);
        }
        if let Destination::Increasing(_, _) = block.destination {
            block.inner.transfer.set_bit(DMA_TI::DEST_INC);
        }

        let mut total_len = usize::max_value();

        block.inner.source = match &block.source {
            Source::Increasing(ptr, len) => {
                total_len = min(total_len, *len);
                (*ptr).into()
            },
            Source::Constant(val) => {
                let val = *val;
                for x in block.const_src.as_mut() {
                    *x = val;
                }

                (&block.const_src).into()
            },
        };

        block.inner.destination = match &block.destination {
            Destination::Increasing(ptr, len) => {
                total_len = min(total_len, *len);
                (*ptr).into()
            },
        };

        block.inner.txfr_len.set(0);
        block.inner.txfr_len.set_value((total_len * 4) as u32, DMA_TXFR_LEN::X_LEN);

        if let Some(next) = &mut block.next {
            block.inner.next_block = (&next.inner).into();
            Self::encode(next);
        } else {
            block.inner.next_block.0 = 0;
        }
    }

    fn write_cs(&mut self, val: u32, mask: u32) {
        use volatile::*;
        let mut cs: DMA_CS = self.registers.cs.read();
        cs.set_value(val, mask);
        self.registers.cs.write(cs);
    }

    fn read_cs(&self, mask: u32) -> u32 {
        use volatile::*;
        self.registers.cs.read().get_value(mask)
    }

    fn do_execute<W: Waiter>(&mut self, block: &ControlBlock) {
        use volatile::*;

        while self.read_cs(DMA_CS::ACTIVE) != 0 {
            W::wait(Duration::from_micros(500));
        }

        // self.write_cs(7, DMA_CS::PRI);
        self.write_cs(1, DMA_CS::RESET);
        self.write_cs(1, DMA_CS::WAIT_FOR_OUTSTANDING_WRITES);

        self.registers.block.write((&block.inner).into());

        //
        self.write_cs(1, DMA_CS::ACTIVE);


        while self.read_cs(DMA_CS::ACTIVE) != 0 {
            W::wait(Duration::from_micros(500));
        }
    }

    pub fn execute<W: Waiter>(&mut self, mut block: MiniBox<ControlBlock>) {
        Self::encode(&mut block);
        self.do_execute::<W>(&block);
    }


}




