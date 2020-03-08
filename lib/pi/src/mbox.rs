use crate::common::*;

#[repr(align(16))]
pub struct MBox(pub [u32; 36]);

impl MBox {
    pub unsafe fn new() -> MBox {
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
                // asm!("" ::: "memory" : "volatile");
                aarch64::dsb();

                /* is it a valid successful response? */
                return self.0[1] == MBOX_RESPONSE;
            }
        }
    }

    pub fn serial_number(&mut self) -> Option<u64> {

        self.0[0] = 8*4;
        self.0[1] = MBOX_REQUEST;
        self.0[2] = MBOX_TAG_GETSERIAL;
        self.0[3] = 8;
        self.0[4] = 8;
        self.0[5] = 0;
        self.0[6] = 0;
        self.0[7] = MBOX_TAG_LAST;

        if unsafe { self.call(MBOX_CH_PROP) } {
            let ser: u64 = (self.0[5] as u64) | ((self.0[6] as u64) << 32);
            Some(ser)
        } else {
            None
        }
    }

    pub fn mac_address(&mut self) -> Option<u64> {

        self.0[0] = 8*4;
        self.0[1] = MBOX_REQUEST;
        self.0[2] = MBOX_TAG_GETMAC;
        self.0[3] = 6;
        self.0[4] = 8;
        self.0[5] = 0;
        self.0[6] = 0;
        self.0[7] = MBOX_TAG_LAST;

        if unsafe { self.call(MBOX_CH_PROP) } {
            let ser: u64 = (self.0[5] as u64) | ((self.0[6] as u64) << 32);
            Some(ser)
        } else {
            None
        }
    }

    pub fn board_revision(&mut self) -> Option<u32> {
        self.0[0] = 7*4;
        self.0[1] = MBOX_REQUEST;
        self.0[2] = MBOX_TAG_GETREVISION;
        self.0[3] = 4;
        self.0[4] = 8;
        self.0[5] = 0;
        self.0[6] = MBOX_TAG_LAST;

        if unsafe { self.call(MBOX_CH_PROP) } {
            Some(self.0[5])
        } else {
            None
        }
    }

    pub fn core_temperature(&mut self) -> Option<u32> {

        self.0[0] = 8*4;
        self.0[1] = MBOX_REQUEST;
        self.0[2] = MBOX_TAG_TEMPERATURE;
        self.0[3] = 8;
        self.0[4] = 8;
        self.0[5] = 0;
        self.0[6] = 0;
        self.0[7] = MBOX_TAG_LAST;

        if unsafe { self.call(MBOX_CH_PROP) } {
            Some(self.0[6])
        } else {
            None
        }
    }

    pub fn set_power_state(&mut self, device_id: u32, enable: bool) -> Option<bool> {

        let mut state = 0u32;
        state |= 1 << 1; // wait for power change
        if enable {
            state |= 1;
        }

        self.0[0] = 8*4;
        self.0[1] = MBOX_REQUEST;
        self.0[2] = MBOX_TAG_SET_POWER;
        self.0[3] = 8;
        self.0[4] = 8;
        self.0[5] = device_id;
        self.0[6] = state;
        self.0[7] = MBOX_TAG_LAST;

        if unsafe { self.call(MBOX_CH_PROP) } {
            Some((self.0[6] & 0b10) == 0)
        } else {
            None
        }
    }

}

