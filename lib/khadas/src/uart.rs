const MMIO_BASE: u64 = 0xffd00000;

const MMIO_AO_BASE: u64 = 0xff800000;
const UART_AO_UART0_BASE: u64 = MMIO_AO_BASE + 0x3000;

const UART_EARLY_PRINT_BASE: u64 = UART_AO_UART0_BASE;

// ref: https://github.com/khadas/linux/blob/khadas-vims-nougat/drivers/amlogic/uart/uart/meson_uart.c

const MESON_AO_UART0_WFIFO: u64 = 0x0;
const MESON_AO_UART0_RFIFO: u64 = 0x4;
const MESON_AO_UART0_CONTROL: u64 = 0x8;
const MESON_AO_UART0_STATUS: u64 = 0xC;
const MESON_UART_RX_EMPTY: u32 = 1 << 20;
const MESON_UART_TX_FULL: u32 = 1 << 21;

pub fn has_byte() -> bool {
    let reg_status = (UART_EARLY_PRINT_BASE + MESON_AO_UART0_STATUS) as *const u32;
    aarch64::dsb();
    let q = unsafe { (reg_status.read_volatile() & MESON_UART_RX_EMPTY) == 0 };
    aarch64::dsb();
    q
}

static mut index: usize = 0;

pub fn read_byte() -> u8 {
    let reg_rfifo = (UART_EARLY_PRINT_BASE + MESON_AO_UART0_RFIFO) as *mut u32;
    aarch64::dsb();
    let q = unsafe { reg_rfifo.read_volatile() & 0xff };
    aarch64::dsb();
    q as u8
    // unsafe {
    //     let c = "sleep 500\nproc2\n".bytes().skip(index).next().unwrap_or(b'a');
    //     index += 1;
    //     c
    // }
}

pub fn get_status_and_control() -> (u32, u32) {
    let status = (UART_EARLY_PRINT_BASE + MESON_AO_UART0_STATUS) as *const u32;
    let control = (UART_EARLY_PRINT_BASE + MESON_AO_UART0_CONTROL) as *const u32;
    unsafe { (status.read_volatile(), control.read_volatile()) }
}

pub fn print_char(ch: u8) {
    let reg_status = (UART_EARLY_PRINT_BASE + MESON_AO_UART0_STATUS) as *const u32;
    let reg_wfifo = (UART_EARLY_PRINT_BASE + MESON_AO_UART0_WFIFO) as *mut u8;

    unsafe {
        while ({
            aarch64::dmb();
            reg_status.read_volatile() & MESON_UART_TX_FULL
        }) != 0 {}

        reg_wfifo.write_volatile(ch);

        aarch64::dmb();

        while ({
            aarch64::dmb();
            reg_status.read_volatile() & MESON_UART_TX_FULL
        }) != 0 {}
    }
}

pub unsafe fn print(s: &str) {
    for b in s.bytes() {
        print_char(b);
    }
}