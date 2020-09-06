use core::fmt::Write;
use core::panic::PanicInfo;
use core::time::Duration;

use pi::pm::reset;
use pi::timer::spin_sleep;
use pi::uart::MiniUart;
use crate::mutex::Mutex;
use crate::{smp, hw};

static PANIC_LOCK: Mutex<bool> = mutex_new!(false);

#[panic_handler]
#[inline(never)]
fn panic(info: &PanicInfo) -> ! {
    let sp = aarch64::SP.get();
    do_panic(info, sp);
}

#[inline(never)]
fn do_panic(info: &PanicInfo, sp: usize) -> ! {
    let mut uart: karch::EarlyWriter;
    {
        let guard = m_lock!(PANIC_LOCK);
        uart = hw::arch().early_writer();

        // EZ
        writeln!(uart, "{}", info).ok();


        uart.write_str(r#"
                (
       (      )     )
         )   (    (
        (          `
    .-""^"""^""^"""^""-.
  (//\\//\\//\\//\\//\\//)
   ~\^^^^^^^^^^^^^^^^^^/~
     `================`

    The pi is overdone.

---------- PANIC ----------
"#).ok();

        if let Some(location) = info.location() {
            writeln!(uart, "FILE: {}", location.file()).ok();
            writeln!(uart, "LINE: {}", location.line()).ok();
            writeln!(uart, "COL: {}", location.column()).ok();
            writeln!(uart, "").ok();
        }

        if let Some(message) = info.message() {
            writeln!(uart, "{}", message).ok();
        } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
            writeln!(uart, "{}", payload).ok();
        }

        // spin_sleep(Duration::from_millis(1500));

        // while uart.has_byte() {
        //     uart.read_byte();
        // }

        let core = smp::core();
        writeln!(&mut uart, "my trace: core={}", core);

        for addr in crate::debug::stack_walker() {
            writeln!(&mut uart, "0x{:08x}", addr.link_register);
        }

        // for addr in crate::debug::stack_scanner(sp, None) {
        //     writeln!(&mut uart, "0x{:08x}", addr);
        // }

    }

    aarch64::brk!(8);

    writeln!(uart, "brk didn't kill us. requesting syscall exit").ok();

    kernel_api::syscall::exit();

    loop {}

    // writeln!(uart, "Press any key to reset...").ok();
    //
    // while !uart.has_byte() {}
    //
    // unsafe { reset(); }

}
