use core::panic::PanicInfo;
use core::fmt::{Write, self};
use pi::uart::MiniUart;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut uart = MiniUart::new();

    // logs "panicked at '$reason', src/main.rs:27:4" to the host stderr
    writeln!(uart, "{}", info).ok();

    loop {}
}
