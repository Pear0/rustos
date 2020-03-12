/// The address where I/O peripherals are mapped to.
pub const IO_BASE: usize = 0x3F00_0000;

pub const IO_PERIPHERAL_BASE_END: usize = 0x4000_0000;

// https://github.com/raspberrypi/documentation/blob/master/hardware/raspberrypi/bcm2836/QA7_rev3.4.pdf
pub const IO_BASE_END: usize = 0x4004_0000;

/// The base address of the `GPIO` registers
pub const GPIO_BASE: usize = IO_BASE + 0x20_0000;

pub const USB_BASE: usize = IO_BASE + 0x98_0000;

/// The number of cores in Rpi3
pub const NCORES: usize = 4;

/// The base of physical addresses that each core is spinning on
pub const SPINNING_BASE: *mut usize = 0xd8 as *mut usize;

/// Generates `pub enums` with no variants for each `ident` passed in.
pub macro states($ ($ name: ident), *) {
$ (
/// A possible state.
# [doc(hidden)]
pub enum $ name {}
) *
}

/// MBox
pub const VIDEOCORE_MBOX: usize = IO_BASE + 0x0000B880;

pub const MBOX_READ: *mut u32 = (VIDEOCORE_MBOX + 0x0) as *mut u32;
pub const MBOX_POLL: *mut u32 = (VIDEOCORE_MBOX + 0x10) as *mut u32;
pub const MBOX_SENDER: *mut u32 = (VIDEOCORE_MBOX + 0x14) as *mut u32;
pub const MBOX_STATUS: *mut u32 = (VIDEOCORE_MBOX + 0x18) as *mut u32;
pub const MBOX_CONFIG: *mut u32 = (VIDEOCORE_MBOX + 0x1C) as *mut u32;
pub const MBOX_WRITE: *mut u32 = (VIDEOCORE_MBOX + 0x20) as *mut u32;

pub const MBOX_RESPONSE: u32 = 0x80000000;
pub const MBOX_FULL: u32 = 0x80000000;
pub const MBOX_EMPTY: u32 = 0x40000000;

pub const MBOX_REQUEST: u32 = 0;

/// MBox channels
pub const MBOX_CH_POWER: u8 = 0;
pub const MBOX_CH_FB: u8 = 1;
pub const MBOX_CH_VUART: u8 = 2;
pub const MBOX_CH_VCHIQ: u8 = 3;
pub const MBOX_CH_LEDS: u8 = 4;
pub const MBOX_CH_BTNS: u8 = 5;
pub const MBOX_CH_TOUCH: u8 = 6;
pub const MBOX_CH_COUNT: u8 = 7;
pub const MBOX_CH_PROP: u8 = 8;

/// MBox tags
pub const MBOX_TAG_GETREVISION: u32 = 0x10002;
pub const MBOX_TAG_GETMAC: u32 = 0x10003;
pub const MBOX_TAG_GETSERIAL: u32 = 0x10004;
pub const MBOX_TAG_TEMPERATURE: u32 = 0x30006;
pub const MBOX_TAG_SET_POWER: u32 = 0x28001;
pub const MBOX_TAG_LAST: u32 = 0;


/// Power Management
pub const PM_RSTC: *mut u32 = (IO_BASE + 0x0010001c) as *mut u32;
pub const PM_RSTS: *mut u32 = (IO_BASE + 0x00100020) as *mut u32;
pub const PM_WDOG: *mut u32 = (IO_BASE + 0x00100024) as *mut u32;
pub const PM_WDOG_MAGIC: u32 = 0x5a000000;
pub const PM_RSTC_FULLRST: u32 = 0x00000020;
