use crate::common::*;

#[repr(align(16))]
pub struct MBox(pub [u32; 36]);

impl MBox {
    pub fn new() -> MBox {
        MBox([0; 36])
    }

    pub unsafe fn call(&mut self, ch: u8) -> bool {
        let addr_channel = ((self as *mut MBox as usize) & !0xFusize) | ((ch as usize) & 0xFusize);

        /* wait until we can write to the mailbox */
        while (MBOX_STATUS.read_volatile() & MBOX_FULL) != 0 {}

        /* write the address of our message to the mailbox with channel identifier */
        MBOX_WRITE.write_volatile(addr_channel as u32);

        /* now wait for the response */
        loop {
            /* is there a response? */
            while (MBOX_STATUS.read_volatile() & MBOX_EMPTY) != 0 {}

            /* is it a response to our message? */
            if addr_channel as u32 == MBOX_READ.read_volatile() {

                // Rust is smart enough to optimize out this read
                // which obviously breaks the mailbox.
                asm!("" ::: "memory" : "volatile");

                /* is it a valid successful response? */
                return self.0[1] == MBOX_RESPONSE;
            }
        }
    }

    pub fn serial_number() -> Option<u64> {
        let mut mbox = MBox::new();

        mbox.0[0] = 8*4;
        mbox.0[1] = MBOX_REQUEST;
        mbox.0[2] = MBOX_TAG_GETSERIAL;
        mbox.0[3] = 8;
        mbox.0[4] = 8;
        mbox.0[5] = 0;
        mbox.0[6] = 0;
        mbox.0[7] = MBOX_TAG_LAST;

        if unsafe { mbox.call(MBOX_CH_PROP) } {
            let ser: u64 = (mbox.0[5] as u64) | ((mbox.0[6] as u64) << 32);
            Some(ser)
        } else {
            None
        }
    }

    pub fn mac_address() -> Option<u64> {
        let mut mbox = MBox::new();

        mbox.0[0] = 8*4;
        mbox.0[1] = MBOX_REQUEST;
        mbox.0[2] = MBOX_TAG_GETMAC;
        mbox.0[3] = 6;
        mbox.0[4] = 8;
        mbox.0[5] = 0;
        mbox.0[6] = 0;
        mbox.0[7] = MBOX_TAG_LAST;

        if unsafe { mbox.call(MBOX_CH_PROP) } {
            let ser: u64 = (mbox.0[5] as u64) | ((mbox.0[6] as u64) << 32);
            Some(ser)
        } else {
            None
        }
    }

    pub fn board_revision() -> Option<u32> {
        let mut mbox = MBox::new();

        mbox.0[0] = 7*4;
        mbox.0[1] = MBOX_REQUEST;
        mbox.0[2] = MBOX_TAG_GETREVISION;
        mbox.0[3] = 4;
        mbox.0[4] = 8;
        mbox.0[5] = 0;
        mbox.0[6] = MBOX_TAG_LAST;

        if unsafe { mbox.call(MBOX_CH_PROP) } {
            Some(mbox.0[5])
        } else {
            None
        }
    }

}

