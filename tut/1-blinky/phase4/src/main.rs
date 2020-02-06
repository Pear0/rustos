/*
 * Here lies lots of unsafe code.
 */

#![feature(asm)]
#![feature(global_asm)]

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

use rand_core::{RngCore, impls, Error};
use rand::Rng;

const MMIO_BASE: usize = 0x3F000000;
const GPIO_BASE: usize = MMIO_BASE + 0x200000;
const VIDEOCORE_MBOX: usize = MMIO_BASE + 0x0000B880;

const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;


const RNG_CTRL: *mut u32 = (GPIO_BASE + 0x00104000) as *mut u32;
const RNG_STATUS: *mut u32 = (GPIO_BASE + 0x00104004) as *mut u32;
const RNG_DATA: *mut u32 = (GPIO_BASE + 0x00104008) as *mut u32;
const RNG_INT_MASK: *mut u32 = (GPIO_BASE + 0x00104010) as *mut u32;

const MBOX_READ: *mut u32 = (VIDEOCORE_MBOX + 0x0) as *mut u32;
const MBOX_POLL: *mut u32 = (VIDEOCORE_MBOX + 0x10) as *mut u32;
const MBOX_SENDER: *mut u32 = (VIDEOCORE_MBOX + 0x14) as *mut u32;
const MBOX_STATUS: *mut u32 = (VIDEOCORE_MBOX + 0x18) as *mut u32;
const MBOX_CONFIG: *mut u32 = (VIDEOCORE_MBOX + 0x1C) as *mut u32;
const MBOX_WRITE: *mut u32 = (VIDEOCORE_MBOX + 0x20) as *mut u32;

const MBOX_RESPONSE: u32 = 0x80000000;
const MBOX_FULL: u32 = 0x80000000;
const MBOX_EMPTY: u32 = 0x40000000;

const MBOX_REQUEST: u32 = 0;
const MBOX_CH_PROP: u32 = 8;
const MBOX_TAG_SETCLKRATE: u32 = 0x38002;
const MBOX_TAG_LAST: u32 = 0;


#[inline(never)]
fn spin_sleep_ms(ms: usize) {
    for _ in 0..(ms * 6000) {
        unsafe { asm!("nop" :::: "volatile"); }
    }
}

#[repr(align(16))]
struct MailboxPayload([u32; 36]);

impl MailboxPayload {
    unsafe fn call(&mut self, ch: u8) -> i32 {

        let r = (((((self as *mut MailboxPayload) as *mut usize) as usize) & !0xF) as u32) | ((ch & 0xF) as u32);
        /* wait until we can write to the mailbox */

        while (MBOX_STATUS.read_volatile() & MBOX_FULL) != 0 {};

        /* write the address of our message to the mailbox with channel identifier */
        MBOX_WRITE.write_volatile(r);


        /* now wait for the response */
        loop {
            /* is there a response? */

            while (MBOX_STATUS.read_volatile() & MBOX_EMPTY ) != 0 {};

            /* is it a response to our message? */
            if r == MBOX_READ.read_volatile() {
                /* is it a valid successful response? */
                return if self.0[1] == MBOX_RESPONSE { 1 } else { 0 } ;
            }

        }
        return 0;
    }
}


#[derive(Default)]
struct RdRand;

impl RdRand {
    unsafe fn init() {
        RNG_STATUS.write_volatile(0x40000);

        // mask interrupt
        RNG_INT_MASK.write_volatile(RNG_INT_MASK.read_volatile() | 1);

        // enable
        RNG_CTRL.write_volatile(RNG_CTRL.read_volatile() | 1);

        while (RNG_STATUS.read_volatile() >> 24) == 0 {}
    }
}

impl RngCore for RdRand {
    fn next_u32(&mut self) -> u32 {
        // implement!
        unsafe { RNG_DATA.read_volatile() }
    }

    fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | (self.next_u32() as u64)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        impls::fill_bytes_via_next(self, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        Ok(self.fill_bytes(dest))
    }
}

unsafe fn kmain() -> ! {

    let mut timer_init = MailboxPayload([0; 36]);

    timer_init.0[0] = 9 * 4;
    timer_init.0[1] = MBOX_REQUEST;
    timer_init.0[2] = MBOX_TAG_SETCLKRATE; // set clock rate
    timer_init.0[3] = 12;
    timer_init.0[4] = 8;
    timer_init.0[5] = 2;        // UART clock
    timer_init.0[6] = 4000000;  // 4Mhz
    timer_init.0[7] = 0;        // clear turbo
    timer_init.0[8] = MBOX_TAG_LAST;
    timer_init.call(MBOX_CH_PROP as u8);


    // this is technically unsound but we'd need locks to properly handle global once-init
    // of RdRand.
    RdRand::init();

    let mut rng: RdRand = Default::default();

    GPIO_FSEL1.write_volatile((GPIO_FSEL1.read_volatile() & !(0x7 << 18)) | (0x1 << 18));

    loop {
        GPIO_SET0.write_volatile(1 << 16);

        spin_sleep_ms(rng.gen_range(0, 1000));

        GPIO_CLR0.write_volatile(1 << 16);

        spin_sleep_ms(rng.gen_range(0, 1000));
    }
}
