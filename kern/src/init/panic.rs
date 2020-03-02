use core::panic::PanicInfo;
use core::fmt::{Write, self};
use pi::uart::MiniUart;
use pi::pm::reset;
use pi::timer::spin_sleep;
use core::time::Duration;


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut uart = MiniUart::new();

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
"#);

    if let Some(location) = info.location() {
        writeln!(uart, "FILE: {}", location.file());
        writeln!(uart, "LINE: {}", location.line());
        writeln!(uart, "COL: {}", location.column());
        writeln!(uart, "");
    }

    if let Some(message) = info.message() {
        writeln!(uart, "{}", message);
    } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
        writeln!(uart, "{}", payload);
    }

    spin_sleep(Duration::from_millis(1500));

    while uart.has_byte() {
        uart.read_byte();
    }

    writeln!(uart, "Press any key to reset...");

    while !uart.has_byte() {}

    unsafe { reset(); }

}
