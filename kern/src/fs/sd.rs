use core::time::Duration;
use shim::io;
use shim::ioerr;
use pi::timer;
use crate::console::kprintln;

use fat32::traits::BlockDevice;

extern "C" {
    /// A global representing the last SD controller error that occured.
    static sd_err: i64;

    /// Initializes the SD card controller.
    ///
    /// Returns 0 if initialization is successful. If initialization fails,
    /// returns -1 if a timeout occured, or -2 if an error sending commands to
    /// the SD controller occured.
    fn sd_init() -> i32;

    /// Reads sector `n` (512 bytes) from the SD card and writes it to `buffer`.
    /// It is undefined behavior if `buffer` does not point to at least 512
    /// bytes of memory.
    ///
    /// On success, returns the number of bytes read: a positive number.
    ///
    /// On error, returns 0. The true error code is stored in the `sd_err`
    /// global. `sd_err` will be set to -1 if a timeout occured or -2 if an
    /// error sending commands to the SD controller occured. Other error codes
    /// are also possible but defined only as being less than zero.
    fn sd_readsector(n: i32, buffer: *mut u8) -> i32;
}

const sleep_multiplier: u64 = 1;
static mut wait_timeout: Duration = Duration::from_secs(0);

#[no_mangle]
fn wait_micros(num: u32) {
    timer::spin_sleep(Duration::from_micros(sleep_multiplier * (num as u64)));
//    let start = timer::current_time();
//    let mut num = Duration::from_micros(sleep_multiplier * (num as u64));
//
//    while timer::current_time() - start < num {
//
//        // exit early if we pass the timeout
//        if timer::current_time() > unsafe { wait_timeout } {
//            return;
//        }
//
//    }
}

/// A handle to an SD card controller.
#[derive(Debug)]
pub struct Sd;

impl Sd {
    /// Initializes the SD card controller and returns a handle to it.
    /// The caller should assure that the method is invoked only once during the
    /// kernel initialization. We can enforce the requirement in safe Rust code
    /// with atomic memory access, but we can't use it yet since we haven't
    /// written the memory management unit (MMU).
    pub unsafe fn new() -> Result<Sd, io::Error> {
        wait_timeout = timer::current_time() + Duration::from_secs(2);
        match sd_init() {
            0 => Ok(Sd{}),
            -1 => ioerr!(TimedOut, "sd init timed out"),
            -2 => ioerr!(BrokenPipe, "could not send init command"),
            _ => ioerr!(Other, "init unknown initialization error"),
        }
    }
}

impl BlockDevice for Sd {
    /// Reads sector `n` from the SD card into `buf`. On success, the number of
    /// bytes read is returned.
    ///
    /// # Errors
    ///
    /// An I/O error of kind `InvalidInput` is returned if `buf.len() < 512` or
    /// `n > 2^31 - 1` (the maximum value for an `i32`).
    ///
    /// An error of kind `TimedOut` is returned if a timeout occurs while
    /// reading from the SD card.
    ///
    /// An error of kind `Other` is returned for all other errors.
    fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() < 512 {
            return ioerr!(InvalidInput, "invalid buf len");
        }
        if n > i32::max_value() as u64 {
            kprintln!("Tried reading invalid block num: {}", n);
            return ioerr!(InvalidInput, "invalid block number");
        }

        kprintln!("read_sector({})", n);

        let result = unsafe {
            wait_timeout = timer::current_time() + Duration::from_secs(2);
            sd_readsector(n as i32, buf.as_mut_ptr())
        };

        if result == 0 {
            return match unsafe { sd_err } {
                -1 => ioerr!(TimedOut, "sd read timed out"),
                -2 => ioerr!(BrokenPipe, "could not send command"),
                _ => ioerr!(Other, "unknown initialization error"),
            }
        }

        kprintln!("> DONE");

        Ok(512)
    }

    fn write_sector(&mut self, _n: u64, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("SD card and file system are read only")
    }
}
