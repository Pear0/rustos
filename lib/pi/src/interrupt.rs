use crate::common::IO_BASE;

use shim::const_assert_size;
use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Interrupt {
    Timer1 = 1,
    Timer3 = 3,
    Usb = 9,
    Aux = 29,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    Uart = 57,
}

impl Interrupt {
    pub const MAX: usize = 9;

    pub fn iter() -> impl Iterator<Item=&'static Interrupt>  {
        use Interrupt::*;
        [Timer1, Timer3, Usb, Gpio0, Gpio1, Gpio2, Gpio3, Uart, Aux].iter()
    }

    pub fn to_index(i: Interrupt) -> usize {
        use Interrupt::*;
        match i {
            Timer1 => 0,
            Timer3 => 1,
            Usb => 2,
            Gpio0 => 3,
            Gpio1 => 4,
            Gpio2 => 5,
            Gpio3 => 6,
            Uart => 7,
            Aux => 8,
        }
    }

    pub fn from_index(i: usize) -> Interrupt {
        use Interrupt::*;
        match i {
            0 => Timer1,
            1 => Timer3,
            2 => Usb,
            3 => Gpio0,
            4 => Gpio1,
            5 => Gpio2,
            6 => Gpio3,
            7 => Uart,
            8 => Aux,
            _ => panic!("Unknown interrupt: {}", i),
        }
    }
}


impl From<usize> for Interrupt {
    fn from(irq: usize) -> Interrupt {
        use Interrupt::*;
        match irq {
            1 => Timer1,
            3 => Timer3,
            9 => Usb,
            49 => Gpio0,
            50 => Gpio1,
            51 => Gpio2,
            52 => Gpio3,
            57 => Uart,
            _ => panic!("Unkonwn irq: {}", irq),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum CoreInterrupt {
    CNTPSIRQ = 0,
    CNTPNSIRQ, //
    CNTHPIRQ,
    CNTVIRQ,
    Mailbox0,
    Mailbox1,
    Mailbox2,
    Mailbox3,
    GPU,
    PMU,
    AXIOutstanding, // core 0
    LocalTimer,
    #[allow(non_camel_case_types)] __last,
}

impl CoreInterrupt {
    pub const MAX: usize = 12;

    pub fn from_index(i: usize) -> CoreInterrupt {
        if i >= CoreInterrupt::__last as usize {
            panic!("unknown interrupt");
        }
        unsafe { core::mem::transmute(i as u8) }
    }

    pub fn iter() -> impl Iterator<Item=CoreInterrupt> {
        use CoreInterrupt::*;
        [CNTPSIRQ, CNTPNSIRQ, CNTHPIRQ, CNTVIRQ, Mailbox0, Mailbox1, Mailbox2, Mailbox3, GPU, PMU, AXIOutstanding, LocalTimer].iter().map(|x| *x)
    }

    pub fn read(core: usize) -> Option<CoreInterrupt> {
        let v = unsafe { ((0x4000_0060 + 4 * (core & 3)) as *const u32).read_volatile() };
        if v == 0 {
            return None;
        }
        let b = u32::trailing_zeros(v) as u8;
        if b >= CoreInterrupt::__last as u8 {
            return None
        }

        Some(unsafe { core::mem::transmute(b) })
    }

}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    // FIXME: Fill me in.
    irq_basic_pending: Volatile<u32>,
    irq_pending: [Volatile<u32>; 2],
    fiq_control: Volatile<u32>,
    irq_enable: [Volatile<u32>; 2],
    irq_basic_enable: Volatile<u32>,
    irq_disable: [Volatile<u32>; 2],
    irq_basic_disable: Volatile<u32>,
}

const_assert_size!(Registers, 40);

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    pub fn new() -> Controller {
        Controller {
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    fn mask(int: Interrupt, regs: &mut [Volatile<u32>; 2]) {
        regs[(int as usize) / 32].write(1 << ((int as usize) % 32))
    }

    /// Enables the interrupt `int`.
    pub fn enable(&mut self, int: Interrupt) {
        Controller::mask(int, &mut self.registers.irq_enable);
    }

    /// Disables the interrupt `int`.
    pub fn disable(&mut self, int: Interrupt) {
        Controller::mask(int, &mut self.registers.irq_disable);
    }

    /// Returns `true` if `int` is pending. Otherwise, returns `false`.
    pub fn is_pending(&self, int: Interrupt) -> bool {
        (self.registers.irq_pending[(int as usize) / 32].read() & (1 << ((int as usize) % 32))) != 0
    }
}
